// Authoritative Nomad jobspec for minisplit-otel-poller (project-owned).
// Deploy via reviewed plan + CAS from this file after release image digest pin.
// Secrets (optional MQTT auth): Nomad Variable nomad/jobs/minisplit-otel-poller
// keys mqtt_username, mqtt_password via workload identity.
//
// Observe-only: MQTT subscribe + HTTP GET lograw + OTLP export. No device control.
// Image pin is updated after each release to an immutable tag@digest.

job "minisplit-otel-poller" {
  datacenters = ["home"]
  type        = "service"

  meta {
    observe_only      = "true"
    rollout_generation = "1"
  }

  group "poller" {
    count = 1

    network {
      mode = "host"

      port "health" {
        static = 9105
      }
    }

    update {
      max_parallel      = 1
      health_check      = "checks"
      min_healthy_time  = "15s"
      healthy_deadline  = "5m"
      progress_deadline = "10m"
      auto_revert       = true
      canary            = 0
    }

    restart {
      attempts = 10
      interval = "30m"
      delay    = "15s"
      mode     = "delay"
    }

    task "poller" {
      driver = "docker"

      identity {
        name = "default"
        aud  = ["nomadproject.io"]
      }

      config {
        # Pin updated post-release to exact linux/amd64 digest.
        # PLACEHOLDER_DIGEST replaced by pin PR after first GHCR publish.
        image        = "ghcr.io/aleksclark/minisplit-otel-poller:v2026.8.0@sha256:ee9aea30da3aa9f65e96c98750b1e8de9b9ae0ef584925f0bf0e6a3bcda3dffb"
        network_mode = "host"
        ports        = ["health"]
      }

      env {
        # Durable fleet DNS — mqtt VIP resolves to 192.168.0.100:1883.
        MQTT_BROKER = "tcp://mqtt.fleet.clark.team:1883"
        # Durable OTEL collector (never stale node-only authority).
        OTEL_EXPORTER_OTLP_ENDPOINT = "http://otel-collector.fleet.clark.team:4318"
        OTEL_RESOURCE_ATTRIBUTES_DEPLOYMENT_ENVIRONMENT = "fleet"
        POLL_INTERVAL = "10s"
        HEALTH_ADDR   = ":9105"
        # Device inventory (name:ip). Not a secret; required config.
        # Keep count stable with live (3 Della units) unless inventory changes intentionally.
        DEVICES = "kitchen:192.168.0.4,livingroom:192.168.0.21,amos:192.168.0.25"
      }

      # Optional MQTT credentials from Nomad Variable (empty OK for anonymous broker).
      template {
        destination = "secrets/mqtt.env"
        env         = true
        change_mode = "restart"
        data        = <<-EOF
{{- with nomadVar "nomad/jobs/minisplit-otel-poller" -}}
MQTT_USERNAME={{ .mqtt_username }}
MQTT_PASSWORD={{ .mqtt_password }}
{{- end -}}
        EOF
      }

      resources {
        cpu    = 100
        memory = 64
      }

      service {
        name     = "minisplit-otel-poller"
        port     = "health"
        provider = "nomad"

        check {
          name     = "minisplit-liveness"
          type     = "http"
          path     = "/healthz"
          interval = "15s"
          timeout  = "3s"
        }

        check {
          name     = "minisplit-readiness"
          type     = "http"
          path     = "/readyz"
          interval = "15s"
          timeout  = "5s"
        }
      }
    }
  }
}
