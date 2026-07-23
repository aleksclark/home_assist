job "syncthing" {
  datacenters = ["home"]
  type        = "service"

  group "syncthing" {
    count = 1

    network {
      mode = "host"
      port "webui"    { static = 8384 }
      port "sync"     { static = 22000 }
      port "discover" { static = 21027 }
    }

    update {
      healthy_deadline  = "10m"
      progress_deadline = "15m"
    }

    task "syncthing" {
      driver = "docker"

      config {
        image        = "lscr.io/linuxserver/syncthing:latest"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/syncthing:/config",
          "/mnt/moosefs/family/phones:/phone_data",
        ]
      }

      env {
        PUID = "1000"
        PGID = "1000"
        TZ   = "America/Chicago"
      }

      resources {
        cpu    = 500
        memory = 512
      }

      service {
        name     = "syncthing"
        provider = "nomad"
        port     = "webui"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.syncthing.rule=Host(`syncthing.fleet.clark.team`)",
          "traefik.http.routers.syncthing.entrypoints=websecure",
          "traefik.http.routers.syncthing.tls=true",
          "traefik.http.routers.syncthing.tls.certresolver=letsencrypt",
          "traefik.http.services.syncthing.loadbalancer.server.port=8384",
        ]

        check {
          type     = "http"
          path     = "/rest/noauth/health"
          port     = "webui"
          interval = "30s"
          timeout  = "5s"

          check_restart {
            limit = 3
            grace = "60s"
          }
        }
      }
    }
  }
}
