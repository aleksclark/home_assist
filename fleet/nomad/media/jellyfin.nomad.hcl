job "jellyfin" {
  datacenters = ["home"]
  type        = "service"

  group "jellyfin" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 8096 }
    }

    # Allow extra time for initial image pull
    update {
      healthy_deadline = "15m"
      progress_deadline = "20m"
    }

    task "jellyfin" {
      driver = "docker"

      config {
        image        = "jellyfin/jellyfin:latest"
        network_mode = "host"

        volumes = [
          "/home/aleks/jellyfin_config:/config",
          "/home/aleks/jellyfin_config/cache:/cache",
          "/mnt/moosefs/media:/media",
          "/mnt/moosefs/family/photos:/photos",
        ]
      }

      env {
        JELLYFIN_CONFIG_DIR  = "/config"
        JELLYFIN_DATA_DIR    = "/config/data"
        JELLYFIN_CACHE_DIR   = "/cache"
        JELLYFIN_LOG_DIR     = "/config/log"
        TZ                   = "America/Chicago"
      }

      resources {
        cpu    = 2000
        memory = 2048
      }

      service {
        name     = "jellyfin"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.jellyfin.rule=Host(`jellyfin.fleet.clark.team`)",
          "traefik.http.routers.jellyfin.entrypoints=websecure",
          "traefik.http.routers.jellyfin.tls=true",
          "traefik.http.routers.jellyfin.tls.certresolver=letsencrypt",
          "traefik.http.services.jellyfin.loadbalancer.server.port=8096",
        ]

        check {
          type     = "http"
          path     = "/health"
          port     = "http"
          interval = "30s"
          timeout  = "5s"

          check_restart {
            limit = 3
            grace = "90s"
          }
        }
      }
    }
  }
}
