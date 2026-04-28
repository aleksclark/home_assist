job "cloudflared" {
  datacenters = ["home"]
  type        = "service"

  group "cloudflared" {
    count = 1

    volume "cloudflared-config" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    task "cloudflared" {
      driver = "docker"

      config {
        image = "cloudflare/cloudflared:latest"

        args = [
          "tunnel", "--no-autoupdate", "run",
          "--token", "${TUNNEL_TOKEN}",
        ]
      }

      env {
        TUNNEL_TOKEN = "eyJhIjoiMWFjN2Q1MzA1ZTEwMjcxOTViNDViZDFhZjdlM2IwMjAiLCJ0IjoiODRkOTY1ODUtNWJlNi00ZjBkLTk3ZWItYzZkYTI2YjY0NDk0IiwicyI6IlpqZGtPRGd4TVRFdE56ZGtZeTAwTkdGaExUbGlaV0l0WlRSbU5UQXlNR0kyTW1SaCJ9"
      }

      resources {
        cpu    = 100
        memory = 128
      }

      restart {
        attempts = 5
        interval = "5m"
        delay    = "10s"
        mode     = "delay"
      }
    }
  }
}
