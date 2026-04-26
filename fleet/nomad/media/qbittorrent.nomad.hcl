job "qbittorrent" {
  datacenters = ["home"]
  type        = "service"

  group "qbittorrent" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      port "webui" { static = 8090 }
    }

    task "qbittorrent" {
      driver = "docker"

      config {
        image = "ghcr.io/hotio/qbittorrent"

        ports = ["webui"]

        cap_add = ["NET_ADMIN"]

        volumes = [
          "/mnt/moosefs/configs/qbittorrent:/config",
          "/mnt/moosefs/media:/media",
        ]
      }

      env {
        PUID             = "1000"
        PGID             = "1000"
        UMASK            = "002"
        TZ               = "America/Chicago"
        VPN_ENABLED      = "true"
        VPN_LAN_NETWORK  = "192.168.0.0/20"
        VPN_CONF         = "wg0"
        VPN_IP_CHECK_DELAY = "5"
        VPN_IP_CHECK_EXIT  = "false"
        VPN_AUTO_PORT_FORWARD = "false"
        WEBUI_PORTS      = "8090/tcp,8090/udp"
        PRIVOXY_ENABLED  = "false"
      }

      resources {
        cpu    = 500
        memory = 512
      }

      service {
        name     = "qbittorrent"
        provider = "nomad"
        port     = "webui"

        tags = [
          "traefik.enable=true",
          "traefik.http.routers.qbittorrent.rule=Host(`qbittorrent.fleet.clark.team`)",
          "traefik.http.routers.qbittorrent.entrypoints=websecure",
          "traefik.http.routers.qbittorrent.tls=true",
          "traefik.http.routers.qbittorrent.tls.certresolver=letsencrypt",
          "traefik.http.services.qbittorrent.loadbalancer.server.port=8090",
        ]

        check {
          type     = "tcp"
          port     = "webui"
          interval = "30s"
          timeout  = "5s"
        }
      }
    }
  }
}
