job "homeassistant" {
  datacenters = ["home"]
  type        = "service"

  group "homeassistant" {
    count = 1

    # Pin to node-3 — 32GB RAM, co-located with matter-server and mosquitto
    constraint {
      attribute = "${node.unique.name}"
      value     = "node-3"
    }

    network {
      mode = "host"
      port "http" {
        static = 8123
      }
    }

    volume "ha-config" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    task "homeassistant" {
      driver = "docker"

      config {
        image        = "ghcr.io/home-assistant/home-assistant:stable"
        network_mode = "host"
        privileged   = true

        volumes = [
          "/mnt/moosefs/configs/homeassistant:/config",
          "/etc/localtime:/etc/localtime:ro",
          "/run/dbus:/run/dbus:ro",
        ]
      }

      env {
        TZ = "America/Chicago"
      }

      resources {
        cpu        = 500
        memory     = 512
        memory_max = 1024
      }

      restart {
        attempts = 3
        interval = "5m"
        delay    = "15s"
        mode     = "delay"
      }

      service {
        name     = "homeassistant"
        provider = "nomad"
        port     = "http"
        tags     = [
          "traefik.enable=true",
          "traefik.http.routers.ha.rule=Host(`ha.fleet.clark.team`)",
          "traefik.http.routers.ha.entrypoints=websecure",
          "traefik.http.routers.ha.tls=true",
          "traefik.http.services.ha.loadbalancer.server.scheme=http",
          "traefik.http.services.ha.loadbalancer.server.port=8123",
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
