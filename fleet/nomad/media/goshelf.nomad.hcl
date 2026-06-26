job "goshelf" {
  datacenters = ["home"]
  type        = "service"

  group "goshelf" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      port "http" {
        static = 8580
        to     = 8080
      }
    }

    task "goshelf" {
      driver = "docker"

      config {
        image      = "goshelf@sha256:0a90da9f4ddbf3d727e13995de7ce1965070fef535de74d60167109c54eb1af2"
        ports = ["http"]

        volumes = [
          "/mnt/moosefs/media/audiobooks:/audiobooks:ro",
          "/mnt/moosefs/configs/goshelf:/data",
        ]
      }

      env {
        READARR_URL     = "http://192.168.0.24:8787"
        READARR_API_KEY = "124c86cb3f13445c8f20e951919fb896"
        MEDIA_PATH      = "/audiobooks"
        LISTEN_ADDR     = ":8080"
        DB_PATH         = "/data/goshelf.db"
      }

      resources {
        cpu    = 200
        memory = 128
      }

      restart {
        attempts = 3
        interval = "5m"
        delay    = "10s"
        mode     = "delay"
      }

      service {
        name     = "goshelf"
        provider = "nomad"
        port     = "http"
        tags     = [
          "traefik.enable=true",
          "traefik.http.routers.goshelf.rule=Host(`goshelf.fleet.clark.team`) || Host(`books.fleet.clark.team`) || Host(`books.clark.team`)",
          "traefik.http.routers.goshelf.entrypoints=websecure",
          "traefik.http.routers.goshelf.tls=true",
        ]
      }
    }
  }
}
