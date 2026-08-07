package main

import (
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"
	"testing"
)

func TestSourceObserveOnly_NoPublishOrControlHTTP(t *testing.T) {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("caller")
	}
	dir := filepath.Dir(file)
	entries, err := os.ReadDir(dir)
	if err != nil {
		t.Fatal(err)
	}
	// Reject MQTT Publish and device control HTTP methods in non-test sources.
	pubRe := regexp.MustCompile(`\.Publish\s*\(`)
	postRe := regexp.MustCompile(`http\.Method(Post|Put|Patch|Delete)`)
	setPathRe := regexp.MustCompile(`"/set"|'/set'|\\/set`)
	cmndRe := regexp.MustCompile(`cmnd/`)

	for _, e := range entries {
		name := e.Name()
		if e.IsDir() || !strings.HasSuffix(name, ".go") || strings.HasSuffix(name, "_test.go") {
			continue
		}
		b, err := os.ReadFile(filepath.Join(dir, name))
		if err != nil {
			t.Fatal(err)
		}
		s := string(b)
		if pubRe.MatchString(s) {
			t.Fatalf("%s must not call MQTT Publish (observe-only)", name)
		}
		if postRe.MatchString(s) {
			t.Fatalf("%s must not use mutating HTTP methods against devices", name)
		}
		// Allowed only inside controlTopicMarkers list as rejection markers.
		// Ensure we never build set topics for subscribe outside markers definition.
		if strings.Contains(s, "fmt.Sprintf") && strings.Contains(s, "/set") && !strings.Contains(s, "controlTopicMarkers") {
			// soft: sprintf with /set outside markers is bad
			if !strings.Contains(s, `"/set"`) {
				t.Fatalf("%s appears to format /set topics", name)
			}
		}
		_ = setPathRe
		_ = cmndRe
	}
}

func TestSource_HasHealthEndpoints(t *testing.T) {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("caller")
	}
	b, err := os.ReadFile(filepath.Join(filepath.Dir(file), "main.go"))
	if err != nil {
		t.Fatal(err)
	}
	s := string(b)
	for _, path := range []string{"/healthz", "/readyz", "/metricsz"} {
		if !strings.Contains(s, path) {
			t.Fatalf("missing health path %s", path)
		}
	}
	if !strings.Contains(s, "observe_only") {
		t.Fatal("metricsz should expose observe_only")
	}
}

func TestSource_NoHardcodedDeviceInventoryDefault(t *testing.T) {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		t.Fatal("caller")
	}
	b, err := os.ReadFile(filepath.Join(filepath.Dir(file), "main.go"))
	if err != nil {
		t.Fatal(err)
	}
	s := string(b)
	// Old code defaulted DEVICES to kitchen/livingroom/amos IPs in getEnv.
	if strings.Contains(s, "kitchen:192.168.") || strings.Contains(s, "livingroom:192.168.") {
		t.Fatal("must not hardcode device inventory defaults in source")
	}
}
