job "radarr" {
  datacenters = ["home"]
  type        = "service"

  group "radarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 7878 }
    }

    task "radarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/radarr"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/radarr:/config",
          "/mnt/moosefs/media:/media",
        ]
      }

      env {
        PUID  = "1000"
        PGID  = "1000"
        UMASK = "002"
        TZ    = "America/Chicago"
      }

      resources {
        cpu    = 300
        memory = 256
      }

      service {
        name     = "radarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.radarr.rule=Host(`radarr.fleet.clark.team`)",
          "traefik.http.routers.radarr.entrypoints=websecure",
          "traefik.http.routers.radarr.tls=true",
          "traefik.http.routers.radarr.tls.certresolver=letsencrypt",
          "traefik.http.services.radarr.loadbalancer.server.port=7878",
        ]

        check {
          type     = "tcp"
          port     = "http"
          interval = "15s"
          timeout  = "3s"
        }
      }
    }
  }
}
