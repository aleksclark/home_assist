job "readarr" {
  datacenters = ["home"]
  type        = "service"

  group "readarr" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "http" { static = 8788 }
    }

    task "readarr" {
      driver = "docker"

      config {
        image        = "ghcr.io/hotio/readarr@sha256:71c8394ed337e75df687f7babc40c7feb4654b90fedc91be76f15674a2529d8e"
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
        WEBUI_PORTS = "8788/tcp,8788/udp"
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
