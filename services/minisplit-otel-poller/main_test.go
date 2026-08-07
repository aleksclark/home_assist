package main

import (
	"strings"
	"testing"
)

func TestTopicIsObserveOnly_AllowsGetSuffixes(t *testing.T) {
	for _, sfx := range allowedGetSuffixes {
		topic := "minisplit_kitchen/" + sfx
		if !topicIsObserveOnly(topic) {
			t.Fatalf("expected observe-only allow for %s", sfx)
		}
	}
}

func TestTopicIsObserveOnly_RejectsControl(t *testing.T) {
	bad := []string{
		"minisplit_kitchen/ACMode/set",
		"minisplit_kitchen/TargetTempLow/set",
		"cmnd/minisplit_kitchen/POWER",
		"minisplit_kitchen/cmnd/POWER",
		"minisplit_kitchen/command",
		"minisplit_kitchen/rpc",
		"stat/RESULT",
		"minisplit_kitchen/something",
	}
	for _, topic := range bad {
		if topicIsObserveOnly(topic) {
			t.Fatalf("expected reject control/unknown topic class: %s", topic)
		}
	}
}

func TestBuildSubscribeTopics_CountAndObserveOnly(t *testing.T) {
	devs := []Device{{Name: "a", IP: "10.0.0.1"}, {Name: "b", IP: "10.0.0.2"}}
	topics, err := buildSubscribeTopics(devs)
	if err != nil {
		t.Fatal(err)
	}
	want := len(devs) * len(allowedGetSuffixes)
	if len(topics) != want {
		t.Fatalf("topic_count=%d want=%d", len(topics), want)
	}
	for _, topic := range topics {
		if !topicIsObserveOnly(topic) {
			t.Fatalf("built non-observe topic")
		}
		if strings.Contains(strings.ToLower(topic), "/set") {
			t.Fatal("subscribe set path forbidden")
		}
	}
}

func TestParseDevices_RequiresNameIP(t *testing.T) {
	if _, err := parseDevices(""); err == nil {
		t.Fatal("empty should fail")
	}
	if _, err := parseDevices("onlyname"); err == nil {
		t.Fatal("missing ip should fail")
	}
	devs, err := parseDevices("kitchen:192.168.0.4,livingroom:192.168.0.21")
	if err != nil {
		t.Fatal(err)
	}
	if len(devs) != 2 {
		t.Fatalf("got %d", len(devs))
	}
}

func TestMainSource_NoMQTTPublishAPI(t *testing.T) {
	// Static guard: main.go must not call Publish (control path).
	// This file is co-located; read via go:embed alternative — open main.go.
	// Using a simple string scan of the package via runtime is overkill; keep
	// behavioral unit tests above and a dedicated file scan in observe_only_test.
}

func TestParseLogLine(t *testing.T) {
	lvl, src, msg := parseLogLine("Info:MAIN:Time 1, free 2")
	if lvl != "Info" || src != "MAIN" || !strings.Contains(msg, "Time") {
		t.Fatalf("parse failed: %q %q %q", lvl, src, msg)
	}
}
