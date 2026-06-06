job "flaresolverr" {
  datacenters = ["home"]
  type        = "service"

  group "flaresolverr" {
    count = 1

    network {
      mode = "host"
      port "http" { static = 8191 }
    }

    task "flaresolverr" {
      driver = "docker"

      config {
        image        = "ghcr.io/flaresolverr/flaresolverr:latest"
        network_mode = "host"
      }

      env {
        LOG_LEVEL = "info"
        TZ        = "America/Chicago"
      }

      resources {
        cpu    = 500
        memory = 512
      }

      service {
        name     = "flaresolverr"
        provider = "nomad"
        port     = "http"

        check {
          type     = "http"
          path     = "/health"
          port     = "http"
          interval = "30s"
          timeout  = "5s"
        }
      }
    }
  }
}
