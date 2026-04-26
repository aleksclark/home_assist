job "speakarr" {
  datacenters = ["home"]
  type        = "service"

  group "speakarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 8787 }
    }

    task "speakarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/radarr:release-6.1.1.10360"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/speakarr:/config",
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
        name     = "speakarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.speakarr.rule=Host(`speakarr.fleet.clark.team`)",
          "traefik.http.routers.speakarr.entrypoints=websecure",
          "traefik.http.routers.speakarr.tls=true",
          "traefik.http.routers.speakarr.tls.certresolver=letsencrypt",
          "traefik.http.services.speakarr.loadbalancer.server.port=8787",
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
