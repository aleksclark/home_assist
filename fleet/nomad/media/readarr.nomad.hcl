job "readarr" {
  datacenters = ["home"]
  type        = "service"

  group "readarr" {
    count = 1

    

    network {
      mode = "host"
      port "http" { static = 8788 }
    }

    task "readarr" {
      driver = "docker"

      config {
        image        = "linuxserver/readarr@sha256:2f06a46938d1ca59b9efb8f6f1a5f53fcc6b413cfe33ad49ffa53527e11e573d"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/readarr:/config",
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
        name     = "readarr"
        provider = "nomad"
        port     = "http"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.readarr.rule=Host(`readarr.fleet.clark.team`)",
          "traefik.http.routers.readarr.entrypoints=websecure",
          "traefik.http.routers.readarr.tls=true",
          "traefik.http.routers.readarr.tls.certresolver=letsencrypt",
          "traefik.http.services.readarr.loadbalancer.server.port=8788",
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
