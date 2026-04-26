job "prowlarr" {
  datacenters = ["home"]
  type        = "service"

  group "prowlarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 9696 }
    }

    task "prowlarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/prowlarr"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/prowlarr:/config",
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
        name     = "prowlarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.prowlarr.rule=Host(`prowlarr.fleet.clark.team`)",
          "traefik.http.routers.prowlarr.entrypoints=websecure",
          "traefik.http.routers.prowlarr.tls=true",
          "traefik.http.routers.prowlarr.tls.certresolver=letsencrypt",
          "traefik.http.services.prowlarr.loadbalancer.server.port=9696",
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
