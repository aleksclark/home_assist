job "sonarr" {
  datacenters = ["home"]
  type        = "service"

  group "sonarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 8989 }
    }

    task "sonarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/sonarr"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/sonarr:/config",
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
        name     = "sonarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.sonarr.rule=Host(`sonarr.fleet.clark.team`)",
          "traefik.http.routers.sonarr.entrypoints=websecure",
          "traefik.http.routers.sonarr.tls=true",
          "traefik.http.routers.sonarr.tls.certresolver=letsencrypt",
          "traefik.http.services.sonarr.loadbalancer.server.port=8989",
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
