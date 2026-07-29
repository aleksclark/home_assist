job "goshelf" {
  datacenters = ["home"]
  type        = "service"

  group "goshelf" {
    count = 1

    network {
      port "http" {
        static = 8580
      }
    }

    volume "moosefs-media" {
      type      = "host"
      source    = "moosefs-media"
      read_only = true
    }

    volume "moosefs-configs" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    service {
      name     = "goshelf"
      port     = "http"
      provider = "nomad"

      tags = [
        "traefik.enable=true",
        "traefik.http.routers.goshelf.rule=Host(`books.fleet.clark.team`) || Host(`books.clark.team`)",
        "traefik.http.routers.goshelf.entrypoints=websecure",
        "traefik.http.routers.goshelf.tls.certresolver=letsencrypt",
      ]

      check {
        type     = "http"
        path     = "/login"
        interval = "30s"
        timeout  = "5s"
      }
    }

    task "goshelf" {
      driver = "docker"

      config {
        image = "ghcr.io/aleksclark/goshelf:v2026.7.16"
        ports = ["http"]
      }

      volume_mount {
        volume      = "moosefs-media"
        destination = "/audiobooks"
        read_only   = true
      }

      volume_mount {
        volume      = "moosefs-configs"
        destination = "/configs"
        read_only   = false
      }

      env {
        LISTEN_ADDR     = ":${NOMAD_PORT_http}"
        READARR_URL     = "http://192.168.0.24:8787"
        READARR_API_KEY = "124c86cb3f13445c8f20e951919fb896"
        MEDIA_PATH      = "/audiobooks/audiobooks"
        DB_PATH         = "/configs/goshelf/goshelf.db"
      }

      resources {
        cpu    = 200
        memory = 128
      }
    }
  }
}
