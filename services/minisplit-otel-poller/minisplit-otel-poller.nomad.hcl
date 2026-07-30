job "minisplit-otel-poller" {
  datacenters = ["home"]
  type        = "service"

  group "poller" {
    count = 1

    network {
      mode = "host"
    }

    task "poller" {
      driver = "docker"

      config {
        image        = "minisplit-otel-poller:v1"
        network_mode = "host"
      }

      env {
        OTEL_EXPORTER_OTLP_ENDPOINT = "http://192.168.0.89:4318"
        MQTT_BROKER                 = "tcp://192.168.0.24:1883"
        POLL_INTERVAL               = "10s"
        DEVICES                     = "kitchen:192.168.0.4,livingroom:192.168.0.21,amos:192.168.0.25"
      }

      resources {
        cpu    = 100
        memory = 64
      }

      restart {
        attempts = 10
        interval = "30m"
        delay    = "15s"
        mode     = "delay"
      }
    }
  }
}
