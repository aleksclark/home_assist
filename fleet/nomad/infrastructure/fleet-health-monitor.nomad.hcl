job "fleet-health-monitor" {
  datacenters = ["home"]
  type        = "service"

  group "monitor" {
    count = 1

    network {
      mode = "host"
    }

    volume "moosefs-configs" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = true
    }

    task "monitor" {
      driver = "docker"

      config {
        image        = "python:3.12-alpine"
        network_mode = "host"
        command      = "python3"
        args         = ["-u", "/monitoring/fleet-health-monitor.py"]

        volumes = [
          "/mnt/moosefs/configs/monitoring:/monitoring:ro",
        ]
      }

      env {
        OTEL_ENDPOINT       = "http://otel-collector.fleet.clark.team:4318"
        CHECK_INTERVAL      = "60"
        NOMAD_TOKEN         = "f1f7e915-b3a9-4a1e-1a9e-733c1d59d52e"
        OMADA_CLIENT_ID     = "67863a1fa043421284537d4e9e0f972f"
        OMADA_CLIENT_SECRET = "63ebbede0e714a80844fde80d4c3c2a5"
      }

      resources {
        cpu    = 100
        memory = 128
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
