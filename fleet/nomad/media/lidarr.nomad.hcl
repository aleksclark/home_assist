job "lidarr" {
  datacenters = ["home"]
  type        = "service"

  group "lidarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 8686 }
    }

    task "lidarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/lidarr"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/lidarr:/config",
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
        name     = "lidarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.lidarr.rule=Host(`lidarr.fleet.clark.team`)",
          "traefik.http.routers.lidarr.entrypoints=websecure",
          "traefik.http.routers.lidarr.tls=true",
          "traefik.http.routers.lidarr.tls.certresolver=letsencrypt",
          "traefik.http.services.lidarr.loadbalancer.server.port=8686",
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
