package main

import (
	"context"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"regexp"
	"strconv"
	"strings"
	"sync"
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

type Device struct {
	Name string
	IP   string
}

type Config struct {
	OTLPEndpoint string
	MQTTBroker   string
	PollInterval time.Duration
	Devices      []Device
}

func loadConfig() Config {
	cfg := Config{
		OTLPEndpoint: getEnv("OTEL_EXPORTER_OTLP_ENDPOINT", "http://node-3:4318"),
		MQTTBroker:   getEnv("MQTT_BROKER", "tcp://mqtt.fleet.clark.team:1883"),
		PollInterval: 10 * time.Second,
	}

	if interval := os.Getenv("POLL_INTERVAL"); interval != "" {
		if d, err := time.ParseDuration(interval); err == nil {
			cfg.PollInterval = d
		}
	}

	devicesStr := getEnv("DEVICES", "kitchen:192.168.0.4,livingroom:192.168.0.21,amos:192.168.0.25")
	for _, entry := range strings.Split(devicesStr, ",") {
		entry = strings.TrimSpace(entry)
		parts := strings.SplitN(entry, ":", 2)
		if len(parts) == 2 {
			cfg.Devices = append(cfg.Devices, Device{Name: parts[0], IP: parts[1]})
		}
	}

	return cfg
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// MQTTCache holds the latest MQTT values per device.
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

func main() {
	cfg := loadConfig()

	slog.Info("minisplit-otel-poller starting",
		"endpoint", cfg.OTLPEndpoint,
		"mqtt_broker", cfg.MQTTBroker,
		"poll_interval", cfg.PollInterval,
		"devices", len(cfg.Devices),
	)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Setup OTEL resource
	res, err := resource.New(ctx,
		resource.WithAttributes(
			semconv.ServiceName("minisplit-otel-poller"),
		),
	)
	if err != nil {
		slog.Error("failed to create resource", "error", err)
		os.Exit(1)
	}

	// Setup OTLP log exporter
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

	// Setup OTLP metric exporter
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

	// Setup MQTT
	mqttCache := NewMQTTCache()
	mqttClient := setupMQTT(cfg, mqttCache)
	defer mqttClient.Disconnect(1000)

	// Setup metrics collector
	collector := NewCollector(cfg, logger, meter, mqttCache)

	// Signal handling
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	ticker := time.NewTicker(cfg.PollInterval)
	defer ticker.Stop()

	// Initial poll
	collector.Poll(ctx)

	for {
		select {
		case <-ticker.C:
			collector.Poll(ctx)
		case sig := <-sigCh:
			slog.Info("received signal, shutting down", "signal", sig)
			cancel()
			return
		}
	}
}

func stripScheme(endpoint string) string {
	endpoint = strings.TrimPrefix(endpoint, "http://")
	endpoint = strings.TrimPrefix(endpoint, "https://")
	return endpoint
}

func setupMQTT(cfg Config, cache *MQTTCache) mqtt.Client {
	opts := mqtt.NewClientOptions().
		AddBroker(cfg.MQTTBroker).
		SetClientID("minisplit-otel-poller").
		SetAutoReconnect(true).
		SetOnConnectHandler(func(c mqtt.Client) {
			slog.Info("MQTT connected, subscribing to topics")
			subscribeAll(c, cfg.Devices, cache)
		})

	client := mqtt.NewClient(opts)
	token := client.Connect()
	token.Wait()
	if token.Error() != nil {
		slog.Error("MQTT connect failed", "error", token.Error())
	}
	return client
}

func subscribeAll(client mqtt.Client, devices []Device, cache *MQTTCache) {
	suffixes := []string{
		"CurrentTemperature/get",
		"TargetTempLow/get",
		"TargetTempHigh/get",
		"FANMode/get",
		"Buzzer/get",
		"Display/get",
		"ACMode/get",
		"HVACAction/get",
	}

	for _, dev := range devices {
		for _, suffix := range suffixes {
			topic := fmt.Sprintf("minisplit_%s/%s", dev.Name, suffix)
			devName := dev.Name
			sfx := suffix
			client.Subscribe(topic, 0, func(_ mqtt.Client, msg mqtt.Message) {
				cache.Set(devName, sfx, string(msg.Payload()))
			})
		}
	}
}

// Collector handles polling devices and emitting telemetry.
type Collector struct {
	cfg       Config
	logger    otellog.Logger
	meter     otelmetric.Meter
	mqttCache *MQTTCache
	client    *http.Client
}

func NewCollector(cfg Config, logger otellog.Logger, meter otelmetric.Meter, mqttCache *MQTTCache) *Collector {
	return &Collector{
		cfg:       cfg,
		logger:    logger,
		meter:     meter,
		mqttCache: mqttCache,
		client:    &http.Client{Timeout: 5 * time.Second},
	}
}

func (c *Collector) Poll(ctx context.Context) {
	for _, dev := range c.cfg.Devices {
		c.pollDevice(ctx, dev)
	}
}

func (c *Collector) pollDevice(ctx context.Context, dev Device) {
	// Fetch logs
	logURL := fmt.Sprintf("http://%s/lograw", dev.IP)
	resp, err := c.client.Get(logURL)
	if err != nil {
		slog.Warn("failed to fetch logs", "device", dev.Name, "error", err)
	} else {
		body, _ := io.ReadAll(resp.Body)
		resp.Body.Close()
		c.processLogs(ctx, dev, string(body))
	}

	// Emit metrics from MQTT cache
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

		// Emit as OTEL log record
		var record otellog.Record
		record.SetTimestamp(time.Now())
		record.SetBody(otellog.StringValue(message))
		record.SetSeverity(mapSeverity(level))
		record.AddAttributes(
			otellog.KeyValue{Key: "device_name", Value: otellog.StringValue(dev.Name)},
			otellog.KeyValue{Key: "device_ip", Value: otellog.StringValue(dev.IP)},
			otellog.KeyValue{Key: "log_level", Value: otellog.StringValue(level)},
			otellog.KeyValue{Key: "log_source", Value: otellog.StringValue(source)},
		)
		c.logger.Emit(ctx, record)

		// Parse MAIN log for system metrics
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

	c.recordGauge(ctx, "minisplit_uptime_seconds", uptime, attrs)
	c.recordGauge(ctx, "minisplit_free_heap_bytes", freeHeap, attrs)
	c.recordGauge(ctx, "minisplit_mqtt_connected", mqttConnected, attrs)
	c.recordGauge(ctx, "minisplit_wifi_connected", wifiConnected, attrs)
	c.recordGauge(ctx, "minisplit_sockets_used", socketsUsed, attrs)
}

func (c *Collector) emitMetrics(ctx context.Context, dev Device) {
	cached := c.mqttCache.GetAll(dev.Name)
	if len(cached) == 0 {
		return
	}

	// Get mode and action for attributes
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

	gaugeMap := map[string]string{
		"minisplit_current_temperature_celsius": "CurrentTemperature/get",
		"minisplit_target_temp_low_celsius":     "TargetTempLow/get",
		"minisplit_target_temp_high_celsius":    "TargetTempHigh/get",
		"minisplit_fan_speed":                   "FANMode/get",
		"minisplit_buzzer_on":                   "Buzzer/get",
		"minisplit_display_on":                  "Display/get",
	}

	for metricName, suffix := range gaugeMap {
		if val, ok := cached[suffix]; ok {
			if f, err := strconv.ParseFloat(val, 64); err == nil {
				c.recordGauge(ctx, metricName, f, attrs)
			}
		}
	}
}

func (c *Collector) recordGauge(ctx context.Context, name string, value float64, attrs otelmetric.MeasurementOption) {
	gauge, err := c.meter.Float64Gauge(name)
	if err != nil {
		slog.Warn("failed to create gauge", "name", name, "error", err)
		return
	}
	gauge.Record(ctx, value, attrs)
}
