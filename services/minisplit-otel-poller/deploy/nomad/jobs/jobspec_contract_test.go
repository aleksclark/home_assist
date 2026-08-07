package deploycontract_test

import (
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
	"testing"
)

func repoServiceRoot(t *testing.T) string {
	t.Helper()
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("caller")
	}
	// deploy/nomad/jobs -> service root ../../..
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", ".."))
}

func readJob(t *testing.T) string {
	t.Helper()
	p := filepath.Join(repoServiceRoot(t), "deploy", "nomad", "jobs", "minisplit-otel-poller.nomad.hcl")
	b, err := os.ReadFile(p)
	if err != nil {
		t.Fatalf("read jobspec: %v", err)
	}
	return string(b)
}

func TestJobspecExistsAndJobID(t *testing.T) {
	body := readJob(t)
	if !strings.Contains(body, `job "minisplit-otel-poller"`) {
		t.Fatal(`missing job "minisplit-otel-poller"`)
	}
}

func TestJobspecImmutableGHCRImage(t *testing.T) {
	body := readJob(t)
	if strings.Contains(body, "minisplit-otel-poller:v1") && !strings.Contains(body, "ghcr.io") {
		t.Fatal("local-only image tag forbidden as authority")
	}
	if !strings.Contains(body, "ghcr.io/aleksclark/minisplit-otel-poller:") {
		t.Fatal("must use ghcr.io image")
	}
	if !strings.Contains(body, "@sha256:") {
		t.Fatal("must pin image digest")
	}
	if strings.Contains(body, ":latest") {
		t.Fatal(":latest must not be deploy authority")
	}
}

func TestJobspecDurableEndpoints(t *testing.T) {
	body := readJob(t)
	if !strings.Contains(body, `MQTT_BROKER = "tcp://mqtt.fleet.clark.team:1883"`) {
		t.Fatal("MQTT_BROKER must be durable mqtt.fleet.clark.team:1883")
	}
	// Per-node otel-agent contract (same durable class as fleet-health-monitor).
	wantOTLP := `OTEL_EXPORTER_OTLP_ENDPOINT = "http://${attr.unique.network.ip-address}:4328"`
	if !strings.Contains(body, wantOTLP) {
		t.Fatal("OTLP must be per-node otel-agent http://${attr.unique.network.ip-address}:4328")
	}
	// Reject stale collector FQDN / central :4318 / hardcoded node IPs as OTLP authority.
	if strings.Contains(body, "otel-collector.fleet.clark.team") {
		t.Fatal("stale otel-collector FQDN forbidden in jobspec; use per-node :4328 agent")
	}
	for _, line := range strings.Split(body, "\n") {
		if strings.Contains(line, "MQTT_BROKER") && strings.Contains(line, "192.168.0.24") {
			t.Fatal("stale node-2 MQTT broker forbidden")
		}
		if !strings.Contains(line, "OTEL_EXPORTER_OTLP_ENDPOINT") {
			continue
		}
		if strings.Contains(line, "192.168.0.89") || strings.Contains(line, "192.168.0.41") {
			t.Fatal("stale direct OTLP node IP forbidden; use attr.unique.network.ip-address:4328")
		}
		if strings.Contains(line, ":4318") {
			t.Fatal("central collector :4318 forbidden; use local otel-agent :4328")
		}
		if strings.Contains(line, ":4328") && !strings.Contains(line, "${attr.unique.network.ip-address}") {
			t.Fatal("OTLP :4328 must use Nomad attr.unique.network.ip-address interpolation")
		}
	}
}

func TestJobspecWorkloadIdentityAndNomadVar(t *testing.T) {
	body := readJob(t)
	if !strings.Contains(body, "identity {") {
		t.Fatal("missing workload identity")
	}
	if !strings.Contains(body, `nomadVar "nomad/jobs/minisplit-otel-poller"`) {
		t.Fatal("missing nomadVar path")
	}
	if !strings.Contains(body, ".mqtt_username") || !strings.Contains(body, ".mqtt_password") {
		t.Fatal("must template mqtt_username and mqtt_password key names")
	}
	// No secret-looking long literals assigned to password keys.
	re := regexp.MustCompile(`(?i)MQTT_PASSWORD\s*=\s*"[^"]{8,}"`)
	if re.MatchString(body) {
		t.Fatal("inline MQTT password forbidden")
	}
}

func TestJobspecHealthAndObserveMeta(t *testing.T) {
	body := readJob(t)
	if !strings.Contains(body, `path     = "/healthz"`) {
		t.Fatal("missing /healthz")
	}
	if !strings.Contains(body, `path     = "/readyz"`) {
		t.Fatal("missing /readyz")
	}
	if !strings.Contains(body, `observe_only      = "true"`) {
		t.Fatal("missing observe_only meta")
	}
	if !strings.Contains(body, "auto_revert") {
		t.Fatal("missing auto_revert")
	}
	if !strings.Contains(body, `static = 9105`) {
		t.Fatal("missing health port 9105")
	}
}

func TestJobspecDeviceCount(t *testing.T) {
	body := readJob(t)
	// DEVICES line should list exactly 3 name:ip pairs for continuity.
	re := regexp.MustCompile(`DEVICES\s*=\s*"([^"]+)"`)
	m := re.FindStringSubmatch(body)
	if m == nil {
		t.Fatal("missing DEVICES")
	}
	parts := strings.Split(m[1], ",")
	if len(parts) != 3 {
		t.Fatalf("device_count=%d want=3", len(parts))
	}
}

func TestLegacyRootJobspecRedirected(t *testing.T) {
	// Old path services/minisplit-otel-poller/minisplit-otel-poller.nomad.hcl must not remain authoritative.
	p := filepath.Join(repoServiceRoot(t), "minisplit-otel-poller.nomad.hcl")
	if _, err := os.Stat(p); err == nil {
		b, _ := os.ReadFile(p)
		s := string(b)
		if strings.Contains(s, "minisplit-otel-poller:v1") && !strings.Contains(s, "MOVED") {
			t.Fatal("legacy jobspec still local-image authority; replace with pointer or delete")
		}
	}
}
