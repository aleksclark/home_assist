job "airsonic" {
  datacenters = ["home"]
  type        = "service"

  group "airsonic" {
    count = 1

    network {
      port "http" { to = 80 }
    }

    update {
      healthy_deadline  = "10m"
      progress_deadline = "15m"
    }

    task "airsonic" {
      driver = "docker"

      config {
        image = "ghcr.io/aleksclark/airsonic-refix:latest"

        ports = ["http"]

        volumes = [
          "/mnt/moosefs/media/music:/music:ro",
          "/mnt/moosefs/media/playlists:/playlists",
        ]
      }

      env {
        SERVER_URL = "https://music-be.clark.team"
      }

      resources {
        cpu    = 500
        memory = 512
      }

      service {
        name     = "airsonic"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.airsonic.rule=Host(`music-be.clark.team`)",
          "traefik.http.routers.airsonic.entrypoints=websecure",
          "traefik.http.routers.airsonic.tls=true",
          "traefik.http.routers.airsonic.tls.certresolver=letsencrypt",
          "traefik.http.services.airsonic.loadbalancer.server.port=${NOMAD_PORT_http}",
        ]

        check {
          type     = "tcp"
          port     = "http"
          interval = "30s"
          timeout  = "5s"

          check_restart {
            limit = 3
            grace = "60s"
          }
        }
      }
    }
  }
}
