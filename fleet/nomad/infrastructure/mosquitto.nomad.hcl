job "mosquitto" {
  datacenters = ["home"]
  type        = "service"

  group "mosquitto" {
    count = 1

    # Pin to node-2 — ESP devices have this IP hardcoded as MQTT broker
    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "mqtt" {
        static = 1883
      }
    }

    volume "mosquitto-data" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = false
    }

    task "mosquitto" {
      driver = "docker"

      config {
        image        = "eclipse-mosquitto:2"
        network_mode = "host"

        ports = ["mqtt"]

        volumes = [
          "/mnt/moosefs/configs/mosquitto/config:/mosquitto/config",
          "/mnt/moosefs/configs/mosquitto/data:/mosquitto/data",
          "/mnt/moosefs/configs/mosquitto/log:/mosquitto/log",
        ]
      }

      resources {
        cpu        = 200
        memory     = 64
        memory_max = 256
      }

      service {
        name     = "mosquitto"
        provider = "nomad"
        port     = "mqtt"

        check {
          type     = "tcp"
          port     = "mqtt"
          interval = "10s"
          timeout  = "2s"
        }
      }
    }
  }
}
