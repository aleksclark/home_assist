job "matter-server" {
  datacenters = ["home"]
  type        = "service"

  group "matter-server" {
    count = 1

    # Co-locate with HA on node-3
    constraint {
      attribute = "${node.unique.name}"
      value     = "node-3"
    }

    network {
      mode = "host"
      port "ws" {
        static = 5580
      }
    }

    volume "matter-data" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    task "matter-server" {
      driver = "docker"

      config {
        image        = "ghcr.io/home-assistant-libs/python-matter-server:stable"
        network_mode = "host"

        volumes = [
          "/mnt/moosefs/configs/matter-server:/data",
        ]

        security_opt = [
          "apparmor=unconfined",
        ]
      }

      resources {
        cpu    = 200
        memory = 256
      }

      restart {
        attempts = 3
        interval = "5m"
        delay    = "15s"
        mode     = "delay"
      }

      service {
        name     = "matter-server"
        provider = "nomad"
        port     = "ws"

        check {
          type     = "tcp"
          port     = "ws"
          interval = "15s"
          timeout  = "3s"
        }
      }
    }
  }
}
