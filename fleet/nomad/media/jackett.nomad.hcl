job "jackett" {
  datacenters = ["home"]
  type        = "service"

  group "jackett" {
    count = 1

    network {
      mode = "host"
      port "http" { static = 9117 }
    }

    task "jackett" {
      driver = "docker"

      config {
        image        = "lscr.io/linuxserver/jackett:latest"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/jackett:/config",
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
        name     = "jackett"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.jackett.rule=Host(`jackett.fleet.clark.team`)",
          "traefik.http.routers.jackett.entrypoints=websecure",
          "traefik.http.routers.jackett.tls=true",
          "traefik.http.routers.jackett.tls.certresolver=letsencrypt",
          "traefik.http.services.jackett.loadbalancer.server.port=9117",
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
