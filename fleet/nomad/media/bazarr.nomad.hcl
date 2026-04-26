job "bazarr" {
  datacenters = ["home"]
  type        = "service"

  group "bazarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 6767 }
    }

    task "bazarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/bazarr"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/bazarr:/config",
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
        name     = "bazarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.bazarr.rule=Host(`bazarr.fleet.clark.team`)",
          "traefik.http.routers.bazarr.entrypoints=websecure",
          "traefik.http.routers.bazarr.tls=true",
          "traefik.http.routers.bazarr.tls.certresolver=letsencrypt",
          "traefik.http.services.bazarr.loadbalancer.server.port=6767",
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
