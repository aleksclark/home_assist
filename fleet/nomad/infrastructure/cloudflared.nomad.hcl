job "cloudflared" {
  datacenters = ["home"]
  type        = "service"

  group "cloudflared" {
    count = 1

    network {
      mode = "host"
    }

    task "cloudflared" {
      driver = "docker"

      config {
        image        = "cloudflare/cloudflared:latest"
        network_mode = "host"

        args = [
          "tunnel", "--no-autoupdate", "--config", "/etc/cloudflared/config.yml", "run",
        ]

        volumes = [
          "/mnt/moosefs/configs/cloudflared/config.yml:/etc/cloudflared/config.yml:ro",
          "/mnt/moosefs/configs/cloudflared/84d96585-5be6-4f0d-97eb-c6da26b64494.json:/etc/cloudflared/84d96585-5be6-4f0d-97eb-c6da26b64494.json:ro",
        ]
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
