job "omada" {
  datacenters = ["home"]
  type        = "service"

  group "omada" {
    count = 1

    # Pin to node-3 — same L2 as APs, lightest loaded
    constraint {
      attribute = "${node.unique.name}"
      value     = "node-3"
    }

    network {
      mode = "host"
      port "manage" {
        static = 8043
      }
    }

    volume "omada-data" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    task "omada-controller" {
      driver = "docker"

      config {
        image        = "mbentley/omada-controller:6.2"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/omada/data:/opt/tplink/EAPController/data",
          "/mnt/moosefs/configs/omada/logs:/opt/tplink/EAPController/logs",
        ]
      }

      env {
        PUID                = "508"
        PGID                = "508"
        TZ                  = "America/Chicago"
        MANAGE_HTTP_PORT    = "8088"
        MANAGE_HTTPS_PORT   = "8043"
        PORTAL_HTTP_PORT    = "8088"
        PORTAL_HTTPS_PORT   = "8843"
        PORT_ADOPT_V1       = "29812"
        PORT_APP_DISCOVERY  = "27001"
        PORT_DISCOVERY      = "29810"
        PORT_MANAGER_V1     = "29811"
        PORT_MANAGER_V2     = "29814"
        PORT_TRANSFER_V2    = "29815"
        PORT_RTTY           = "29816"
        PORT_UPGRADE_V1     = "29813"
        SHOW_SERVER_LOGS    = "true"
        SHOW_MONGODB_LOGS   = "false"
      }

      resources {
        cpu    = 500
        memory     = 2560
        memory_max = 3072
      }

      # Omada takes ~45s to start (embedded MongoDB + Java)
      restart {
        attempts = 3
        interval = "5m"
        delay    = "30s"
        mode     = "delay"
      }

      service {
        name         = "omada"
        provider     = "nomad"
        port         = "manage"
        tags         = [
          "traefik.enable=true",
          "traefik.http.routers.omada.rule=Host(`omada.fleet.local`)",
          "traefik.http.routers.omada.entrypoints=websecure",
          "traefik.http.routers.omada.tls=true",
          "traefik.http.services.omada.loadbalancer.server.scheme=https",
        ]
      }
    }
  }
}
