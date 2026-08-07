package main

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"net"
	"net/http"
	"os"
	"os/signal"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"sync/atomic"
	"syscall"
	"time"

	mqtt "github.com/eclipse/paho.mqtt.golang"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/exporters/otlp/otlplog/otlploghttp"
	"go.opentelemetry.io/otel/exporters/otlp/otlpmetric/otlpmetrichttp"
	otellog "go.opentelemetry.io/otel/log"
	otelmetric "go.opentelemetry.io/otel/metric"
	sdklog "go.opentelemetry.io/otel/sdk/log"
	sdkmetric "go.opentelemetry.io/otel/sdk/metric"
	"go.opentelemetry.io/otel/sdk/resource"
	semconv "go.opentelemetry.io/otel/semconv/v1.26.0"
)

// Version is injected at build time (-X main.Version=...).
var Version = "dev"

// Observe-only MQTT topic suffixes (telemetry). Control/command paths are rejected.
var allowedGetSuffixes = []string{
	"CurrentTemperature/get",
	"TargetTempLow/get",
	"TargetTempHigh/get",
	"FANMode/get",
	"Buzzer/get",
	"Display/get",
	"ACMode/get",
	"HVACAction/get",
}

// controlTopicMarkers are forbidden for subscribe or publish in this process.
var controlTopicMarkers = []string{
	"/set",
	"/cmnd",
	"cmnd/",
	"/command",
	"/rpc",
	"stat/RESULT",
}

type Device struct {
	Name string
	IP   string
}

type Config struct {
	OTLPEndpoint  string
	MQTTBroker    string
	MQTTUsername  string
	MQTTPassword  string
	PollInterval  time.Duration
	Devices       []Device
	HealthAddr    string
	ServiceEnv    string
}

func loadConfig() (Config, error) {
	cfg := Config{
		OTLPEndpoint: getEnv("OTEL_EXPORTER_OTLP_ENDPOINT", "http://otel-collector.fleet.clark.team:4318"),
		MQTTBroker:   getEnv("MQTT_BROKER", "tcp://mqtt.fleet.clark.team:1883"),
		MQTTUsername: os.Getenv("MQTT_USERNAME"),
		MQTTPassword: os.Getenv("MQTT_PASSWORD"),
		PollInterval: 10 * time.Second,
		HealthAddr:   getEnv("HEALTH_ADDR", ":9105"),
		ServiceEnv:   getEnv("OTEL_RESOURCE_ATTRIBUTES_DEPLOYMENT_ENVIRONMENT", "fleet"),
	}

	if interval := os.Getenv("POLL_INTERVAL"); interval != "" {
		d, err := time.ParseDuration(interval)
		if err != nil {
			return cfg, fmt.Errorf("POLL_INTERVAL: %w", err)
		}
		cfg.PollInterval = d
	}

	devicesStr := os.Getenv("DEVICES")
	if devicesStr == "" {
		return cfg, fmt.Errorf("DEVICES is required (name:ip,...); no hardcoded device inventory")
	}
	devs, err := parseDevices(devicesStr)
	if err != nil {
		return cfg, err
	}
	cfg.Devices = devs
	return cfg, nil
}

func parseDevices(devicesStr string) ([]Device, error) {
	var out []Device
	for _, entry := range strings.Split(devicesStr, ",") {
		entry = strings.TrimSpace(entry)
		if entry == "" {
			continue
		}
		parts := strings.SplitN(entry, ":", 2)
		if len(parts) != 2 || parts[0] == "" || parts[1] == "" {
			return nil, fmt.Errorf("invalid DEVICES entry (want name:ip)")
		}
		// Reject control-looking names.
		if strings.Contains(strings.ToLower(parts[0]), "cmnd") {
			return nil, fmt.Errorf("invalid device name")
		}
		out = append(out, Device{Name: parts[0], IP: parts[1]})
	}
	if len(out) == 0 {
		return nil, fmt.Errorf("DEVICES empty after parse")
	}
	return out, nil
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// topicIsObserveOnly reports whether a topic is allowed for subscribe (telemetry get only).
func topicIsObserveOnly(topic string) bool {
	low := strings.ToLower(topic)
	for _, m := range controlTopicMarkers {
		if strings.Contains(low, strings.ToLower(m)) {
			return false
		}
	}
	// Must end with one of the allowed /get suffixes.
	for _, sfx := range allowedGetSuffixes {
		if strings.HasSuffix(topic, sfx) {
			return true
		}
	}
	return false
}

// buildSubscribeTopics returns observe-only topics for the configured devices.
func buildSubscribeTopics(devices []Device) ([]string, error) {
	var topics []string
	for _, dev := range devices {
		for _, suffix := range allowedGetSuffixes {
			topic := fmt.Sprintf("minisplit_%s/%s", dev.Name, suffix)
			if !topicIsObserveOnly(topic) {
				return nil, fmt.Errorf("refusing non-observe topic class")
			}
			topics = append(topics, topic)
		}
	}
	return topics, nil
}

// MQTTCache holds the latest MQTT values per device (telemetry only).
type MQTTCache struct {
	mu   sync.RWMutex
	data map[string]map[string]string // device_name -> topic_suffix -> value
}

func NewMQTTCache() *MQTTCache {
	return &MQTTCache{data: make(map[string]map[string]string)}
}

func (c *MQTTCache) Set(device, suffix, value string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.data[device] == nil {
		c.data[device] = make(map[string]string)
	}
	c.data[device][suffix] = value
}

func (c *MQTTCache) GetAll(device string) map[string]string {
	c.mu.RLock()
	defer c.mu.RUnlock()
	result := make(map[string]string)
	if m, ok := c.data[device]; ok {
		for k, v := range m {
			result[k] = v
		}
	}
	return result
}

// RuntimeStats is value-free operational counters for health/readiness.
type RuntimeStats struct {
	pollOK      atomic.Uint64
	pollErr     atomic.Uint64
	metricOK    atomic.Uint64
	metricSkip  atomic.Uint64
	mqttMsgs    atomic.Uint64
	lastPollUnix atomic.Int64
	mqttConnected atomic.Bool
	startedUnix atomic.Int64
}

type App struct {
	cfg        Config
	cache      *MQTTCache
	mqtt       mqtt.Client
	stats      *RuntimeStats
	collector  *Collector
	httpServer *http.Server
}

func main() {
	cfg, err := loadConfig()
	if err != nil {
		slog.Error("config error", "error", err)
		os.Exit(1)
	}

	slog.Info("minisplit-otel-poller starting",
		"version", Version,
		"endpoint_class", endpointHostClass(cfg.OTLPEndpoint),
		"mqtt_broker_class", brokerHostClass(cfg.MQTTBroker),
		"poll_interval", cfg.PollInterval.String(),
		"devices", len(cfg.Devices),
		"health_addr", cfg.HealthAddr,
		"observe_only", true,
	)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	res, err := resource.New(ctx,
		resource.WithAttributes(
			semconv.ServiceName("minisplit-otel-poller"),
			semconv.ServiceVersion(Version),
			semconv.DeploymentEnvironment(cfg.ServiceEnv),
		),
	)
	if err != nil {
		slog.Error("failed to create resource", "error", err)
		os.Exit(1)
	}

	logExporter, err := otlploghttp.New(ctx,
		otlploghttp.WithEndpoint(stripScheme(cfg.OTLPEndpoint)),
		otlploghttp.WithInsecure(),
	)
	if err != nil {
		slog.Error("failed to create log exporter", "error", err)
		os.Exit(1)
	}
	logProvider := sdklog.NewLoggerProvider(
		sdklog.WithResource(res),
		sdklog.WithProcessor(sdklog.NewBatchProcessor(logExporter)),
	)
	defer func() { _ = logProvider.Shutdown(context.Background()) }()
	logger := logProvider.Logger("minisplit-otel-poller")

	metricExporter, err := otlpmetrichttp.New(ctx,
		otlpmetrichttp.WithEndpoint(stripScheme(cfg.OTLPEndpoint)),
		otlpmetrichttp.WithInsecure(),
	)
	if err != nil {
		slog.Error("failed to create metric exporter", "error", err)
		os.Exit(1)
	}
	meterProvider := sdkmetric.NewMeterProvider(
		sdkmetric.WithResource(res),
		sdkmetric.WithReader(sdkmetric.NewPeriodicReader(metricExporter,
			sdkmetric.WithInterval(cfg.PollInterval))),
	)
	defer func() { _ = meterProvider.Shutdown(context.Background()) }()
	meter := meterProvider.Meter("minisplit-otel-poller")

	stats := &RuntimeStats{}
	stats.startedUnix.Store(time.Now().Unix())
	cache := NewMQTTCache()

	mqttClient, err := setupMQTT(cfg, cache, stats)
	if err != nil {
		slog.Error("MQTT setup failed", "error", err)
		os.Exit(1)
	}
	defer mqttClient.Disconnect(1000)

	collector, err := NewCollector(cfg, logger, meter, cache, stats)
	if err != nil {
		slog.Error("collector setup failed", "error", err)
		os.Exit(1)
	}

	app := &App{
		cfg:       cfg,
		cache:     cache,
		mqtt:      mqttClient,
		stats:     stats,
		collector: collector,
	}
	app.startHealthServer()

	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	ticker := time.NewTicker(cfg.PollInterval)
	defer ticker.Stop()

	collector.Poll(ctx)

	for {
		select {
		case <-ticker.C:
			collector.Poll(ctx)
		case sig := <-sigCh:
			slog.Info("received signal, shutting down", "signal", sig.String())
			cancel()
			shutdownCtx, c := context.WithTimeout(context.Background(), 5*time.Second)
			defer c()
			if app.httpServer != nil {
				_ = app.httpServer.Shutdown(shutdownCtx)
			}
			return
		}
	}
}

func endpointHostClass(endpoint string) string {
	h := stripScheme(endpoint)
	host, _, err := net.SplitHostPort(h)
	if err != nil {
		host = h
	}
	if net.ParseIP(host) != nil {
		return "ip"
	}
	return "fqdn"
}

func brokerHostClass(broker string) string {
	b := strings.TrimPrefix(broker, "tcp://")
	b = strings.TrimPrefix(b, "ssl://")
	host, _, err := net.SplitHostPort(b)
	if err != nil {
		host = b
	}
	if net.ParseIP(host) != nil {
		return "ip"
	}
	return "fqdn"
}

func stripScheme(endpoint string) string {
	endpoint = strings.TrimPrefix(endpoint, "http://")
	endpoint = strings.TrimPrefix(endpoint, "https://")
	return endpoint
}

func setupMQTT(cfg Config, cache *MQTTCache, stats *RuntimeStats) (mqtt.Client, error) {
	topics, err := buildSubscribeTopics(cfg.Devices)
	if err != nil {
		return nil, err
	}

	opts := mqtt.NewClientOptions().
		AddBroker(cfg.MQTTBroker).
		SetClientID("minisplit-otel-poller").
		SetAutoReconnect(true).
		SetConnectRetry(true).
		SetOrderMatters(false).
		SetOnConnectHandler(func(c mqtt.Client) {
			stats.mqttConnected.Store(true)
			slog.Info("MQTT connected, subscribing observe-only topics", "topic_count", len(topics))
			subscribeAll(c, cfg.Devices, cache, stats)
		}).
		SetConnectionLostHandler(func(_ mqtt.Client, _ error) {
			stats.mqttConnected.Store(false)
			slog.Warn("MQTT connection lost")
		})

	if cfg.MQTTUsername != "" {
		opts.SetUsername(cfg.MQTTUsername)
		opts.SetPassword(cfg.MQTTPassword)
	}

	// Hard fail: never set a default publish handler that could send control.
	opts.SetDefaultPublishHandler(func(_ mqtt.Client, msg mqtt.Message) {
		// Ignore unexpected publishes to this client; do not forward or republish.
		_ = msg
	})

	client := mqtt.NewClient(opts)
	token := client.Connect()
	token.Wait()
	if token.Error() != nil {
		// Non-fatal at start: auto-reconnect will retry; readiness stays false.
		slog.Error("MQTT initial connect failed", "error", token.Error())
	}
	return client, nil
}

func subscribeAll(client mqtt.Client, devices []Device, cache *MQTTCache, stats *RuntimeStats) {
	for _, dev := range devices {
		for _, suffix := range allowedGetSuffixes {
			topic := fmt.Sprintf("minisplit_%s/%s", dev.Name, suffix)
			if !topicIsObserveOnly(topic) {
				slog.Error("refusing subscribe to non-observe topic class")
				continue
			}
			devName := dev.Name
			sfx := suffix
			tok := client.Subscribe(topic, 0, func(_ mqtt.Client, msg mqtt.Message) {
				// Observe-only: cache payload; never publish, never write device config.
				if !topicIsObserveOnly(msg.Topic()) {
					return
				}
				cache.Set(devName, sfx, string(msg.Payload()))
				stats.mqttMsgs.Add(1)
			})
			tok.Wait()
			if tok.Error() != nil {
				slog.Warn("MQTT subscribe failed", "error", tok.Error())
			}
		}
	}
}

// Collector handles polling devices and emitting telemetry. Observe-only: HTTP GET lograw only.
type Collector struct {
	cfg       Config
	logger    otellog.Logger
	mqttCache *MQTTCache
	stats     *RuntimeStats
	client    *http.Client

	uptimeGauge     otelmetric.Float64Gauge
	heapGauge       otelmetric.Float64Gauge
	mqttConnGauge   otelmetric.Float64Gauge
	wifiGauge       otelmetric.Float64Gauge
	socksGauge      otelmetric.Float64Gauge
	tempGauge       otelmetric.Float64Gauge
	lowGauge        otelmetric.Float64Gauge
	highGauge       otelmetric.Float64Gauge
	fanGauge        otelmetric.Float64Gauge
	buzzerGauge     otelmetric.Float64Gauge
	displayGauge    otelmetric.Float64Gauge
	pollOKCounter   otelmetric.Int64Counter
	pollErrCounter  otelmetric.Int64Counter
}

func NewCollector(cfg Config, logger otellog.Logger, meter otelmetric.Meter, mqttCache *MQTTCache, stats *RuntimeStats) (*Collector, error) {
	c := &Collector{
		cfg:       cfg,
		logger:    logger,
		mqttCache: mqttCache,
		stats:     stats,
		client:    &http.Client{Timeout: 5 * time.Second},
	}
	var err error
	if c.uptimeGauge, err = meter.Float64Gauge("minisplit_uptime_seconds"); err != nil {
		return nil, err
	}
	if c.heapGauge, err = meter.Float64Gauge("minisplit_free_heap_bytes"); err != nil {
		return nil, err
	}
	if c.mqttConnGauge, err = meter.Float64Gauge("minisplit_mqtt_connected"); err != nil {
		return nil, err
	}
	if c.wifiGauge, err = meter.Float64Gauge("minisplit_wifi_connected"); err != nil {
		return nil, err
	}
	if c.socksGauge, err = meter.Float64Gauge("minisplit_sockets_used"); err != nil {
		return nil, err
	}
	if c.tempGauge, err = meter.Float64Gauge("minisplit_current_temperature_celsius"); err != nil {
		return nil, err
	}
	if c.lowGauge, err = meter.Float64Gauge("minisplit_target_temp_low_celsius"); err != nil {
		return nil, err
	}
	if c.highGauge, err = meter.Float64Gauge("minisplit_target_temp_high_celsius"); err != nil {
		return nil, err
	}
	if c.fanGauge, err = meter.Float64Gauge("minisplit_fan_speed"); err != nil {
		return nil, err
	}
	if c.buzzerGauge, err = meter.Float64Gauge("minisplit_buzzer_on"); err != nil {
		return nil, err
	}
	if c.displayGauge, err = meter.Float64Gauge("minisplit_display_on"); err != nil {
		return nil, err
	}
	if c.pollOKCounter, err = meter.Int64Counter("minisplit_poll_success_total"); err != nil {
		return nil, err
	}
	if c.pollErrCounter, err = meter.Int64Counter("minisplit_poll_error_total"); err != nil {
		return nil, err
	}
	return c, nil
}

func (c *Collector) Poll(ctx context.Context) {
	for _, dev := range c.cfg.Devices {
		c.pollDevice(ctx, dev)
	}
	c.stats.lastPollUnix.Store(time.Now().Unix())
}

func (c *Collector) pollDevice(ctx context.Context, dev Device) {
	// Observe-only HTTP: GET lograw. Never POST/PUT device control endpoints.
	logURL := fmt.Sprintf("http://%s/lograw", dev.IP)
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, logURL, nil)
	if err != nil {
		c.stats.pollErr.Add(1)
		c.pollErrCounter.Add(ctx, 1, otelmetric.WithAttributes(attribute.String("device_name", dev.Name)))
		return
	}
	resp, err := c.client.Do(req)
	if err != nil {
		slog.Warn("failed to fetch logs", "device", dev.Name, "error", err)
		c.stats.pollErr.Add(1)
		c.pollErrCounter.Add(ctx, 1, otelmetric.WithAttributes(attribute.String("device_name", dev.Name)))
	} else {
		body, _ := io.ReadAll(io.LimitReader(resp.Body, 1<<20))
		resp.Body.Close()
		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			c.stats.pollOK.Add(1)
			c.pollOKCounter.Add(ctx, 1, otelmetric.WithAttributes(attribute.String("device_name", dev.Name)))
			c.processLogs(ctx, dev, string(body))
		} else {
			c.stats.pollErr.Add(1)
			c.pollErrCounter.Add(ctx, 1, otelmetric.WithAttributes(attribute.String("device_name", dev.Name)))
		}
	}

	c.emitMetrics(ctx, dev)
}

func (c *Collector) processLogs(ctx context.Context, dev Device, body string) {
	lines := strings.Split(strings.TrimSpace(body), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}

		level, source, message := parseLogLine(line)

		var record otellog.Record
		record.SetTimestamp(time.Now())
		record.SetBody(otellog.StringValue(message))
		record.SetSeverity(mapSeverity(level))
		record.AddAttributes(
			otellog.KeyValue{Key: "device_name", Value: otellog.StringValue(dev.Name)},
			// device_ip intentionally omitted from OTEL attributes to reduce identity surface.
			otellog.KeyValue{Key: "log_level", Value: otellog.StringValue(level)},
			otellog.KeyValue{Key: "log_source", Value: otellog.StringValue(source)},
		)
		c.logger.Emit(ctx, record)

		if source == "MAIN" {
			c.parseMainMetrics(ctx, dev, message)
		}
	}
}

func parseLogLine(line string) (level, source, message string) {
	parts := strings.SplitN(line, ":", 3)
	if len(parts) == 3 {
		return parts[0], parts[1], parts[2]
	}
	if len(parts) == 2 {
		return parts[0], parts[1], ""
	}
	return "Info", "UNKNOWN", line
}

func mapSeverity(level string) otellog.Severity {
	switch strings.ToLower(level) {
	case "error":
		return otellog.SeverityError
	case "warn", "warning":
		return otellog.SeverityWarn
	default:
		return otellog.SeverityInfo
	}
}

var mainLogRe = regexp.MustCompile(
	`Time (\d+),.*free (\d+),.*MQTT (\d+)\(\d+\),.*bWifi (\d+),.*socks (\d+)/\d+`,
)

func (c *Collector) parseMainMetrics(ctx context.Context, dev Device, message string) {
	matches := mainLogRe.FindStringSubmatch(message)
	if matches == nil {
		return
	}

	uptime, _ := strconv.ParseFloat(matches[1], 64)
	freeHeap, _ := strconv.ParseFloat(matches[2], 64)
	mqttConnected, _ := strconv.ParseFloat(matches[3], 64)
	wifiConnected, _ := strconv.ParseFloat(matches[4], 64)
	socketsUsed, _ := strconv.ParseFloat(matches[5], 64)

	attrs := otelmetric.WithAttributeSet(attribute.NewSet(
		attribute.String("device_name", dev.Name),
	))

	c.uptimeGauge.Record(ctx, uptime, attrs)
	c.heapGauge.Record(ctx, freeHeap, attrs)
	c.mqttConnGauge.Record(ctx, mqttConnected, attrs)
	c.wifiGauge.Record(ctx, wifiConnected, attrs)
	c.socksGauge.Record(ctx, socketsUsed, attrs)
	c.stats.metricOK.Add(5)
}

func (c *Collector) emitMetrics(ctx context.Context, dev Device) {
	cached := c.mqttCache.GetAll(dev.Name)
	if len(cached) == 0 {
		c.stats.metricSkip.Add(1)
		return
	}

	mode := cached["ACMode/get"]
	action := cached["HVACAction/get"]

	attrKVs := []attribute.KeyValue{attribute.String("device_name", dev.Name)}
	if mode != "" {
		attrKVs = append(attrKVs, attribute.String("mode", mode))
	}
	if action != "" {
		attrKVs = append(attrKVs, attribute.String("action", action))
	}
	attrs := otelmetric.WithAttributeSet(attribute.NewSet(attrKVs...))

	type pair struct {
		g   otelmetric.Float64Gauge
		key string
	}
	for _, p := range []pair{
		{c.tempGauge, "CurrentTemperature/get"},
		{c.lowGauge, "TargetTempLow/get"},
		{c.highGauge, "TargetTempHigh/get"},
		{c.fanGauge, "FANMode/get"},
		{c.buzzerGauge, "Buzzer/get"},
		{c.displayGauge, "Display/get"},
	} {
		if val, ok := cached[p.key]; ok {
			if f, err := strconv.ParseFloat(val, 64); err == nil {
				p.g.Record(ctx, f, attrs)
				c.stats.metricOK.Add(1)
			}
		}
	}
}

func (a *App) startHealthServer() {
	mux := http.NewServeMux()
	mux.HandleFunc("/healthz", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("ok\n"))
	})
	mux.HandleFunc("/readyz", func(w http.ResponseWriter, r *http.Request) {
		// Ready when MQTT connected, devices configured, and at least one poll cycle completed.
		ok := a.stats.mqttConnected.Load() &&
			len(a.cfg.Devices) > 0 &&
			a.stats.lastPollUnix.Load() > 0
		if !ok {
			w.WriteHeader(http.StatusServiceUnavailable)
			_, _ = w.Write([]byte("not ready\n"))
			return
		}
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("ready\n"))
	})
	mux.HandleFunc("/metricsz", func(w http.ResponseWriter, r *http.Request) {
		// Value-free operational counters only (no device payloads/topics).
		w.Header().Set("Content-Type", "text/plain; charset=utf-8")
		fmt.Fprintf(w, "version %s\n", Version)
		fmt.Fprintf(w, "devices %d\n", len(a.cfg.Devices))
		fmt.Fprintf(w, "mqtt_connected %t\n", a.stats.mqttConnected.Load())
		fmt.Fprintf(w, "poll_ok %d\n", a.stats.pollOK.Load())
		fmt.Fprintf(w, "poll_err %d\n", a.stats.pollErr.Load())
		fmt.Fprintf(w, "metric_ok %d\n", a.stats.metricOK.Load())
		fmt.Fprintf(w, "metric_skip %d\n", a.stats.metricSkip.Load())
		fmt.Fprintf(w, "mqtt_msgs %d\n", a.stats.mqttMsgs.Load())
		fmt.Fprintf(w, "last_poll_unix %d\n", a.stats.lastPollUnix.Load())
		fmt.Fprintf(w, "started_unix %d\n", a.stats.startedUnix.Load())
		fmt.Fprintf(w, "observe_only true\n")
	})

	a.httpServer = &http.Server{
		Addr:              a.cfg.HealthAddr,
		Handler:           mux,
		ReadHeaderTimeout: 5 * time.Second,
	}
	go func() {
		ln, err := net.Listen("tcp", a.cfg.HealthAddr)
		if err != nil {
			slog.Error("health listen failed", "error", err)
			return
		}
		slog.Info("health server listening", "addr", a.cfg.HealthAddr)
		if err := a.httpServer.Serve(ln); err != nil && err != http.ErrServerClosed {
			slog.Error("health server error", "error", err)
		}
	}()
}
