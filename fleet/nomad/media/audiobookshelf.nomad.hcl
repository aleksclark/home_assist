job "audiobookshelf" {
  datacenters = ["home"]
  type        = "service"

  group "audiobookshelf" {
    count = 1

    network {
      port "http" {
        static = 13378
        to     = 80
      }
    }

    task "audiobookshelf" {
      driver = "docker"

      config {
        image = "ghcr.io/advplyr/audiobookshelf:latest"
        ports = ["http"]

        volumes = [
          "/mnt/moosefs/configs/audiobookshelf/config:/config",
          "/mnt/moosefs/configs/audiobookshelf/metadata:/metadata",
          "/mnt/moosefs/media/audiobooks:/audiobooks",
        ]
      }

      env {
        TZ = "America/Chicago"
      }

      resources {
        cpu    = 500
        memory = 512
      }

      restart {
        attempts = 3
        interval = "5m"
        delay    = "15s"
        mode     = "delay"
      }

      service {
        name     = "audiobookshelf"
        provider = "nomad"
        port     = "http"
        tags     = [
          "traefik.enable=true",
          "traefik.http.routers.audiobookshelf.rule=Host(`audiobookshelf.fleet.clark.team`) || Host(`books.fleet.clark.team`) || Host(`books.clark.team`)",
          "traefik.http.routers.audiobookshelf.entrypoints=websecure",
          "traefik.http.routers.audiobookshelf.tls=true",
        ]
      }
    }
  }
}
