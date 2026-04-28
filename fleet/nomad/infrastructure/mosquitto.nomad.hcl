job "mosquitto" {
  datacenters = ["home"]
  type        = "service"

  group "mosquitto" {
    count = 1

    network {
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
        image = "eclipse-mosquitto:2"

        ports = ["mqtt"]

        volumes = [
          "/mnt/moosefs/configs/mosquitto/config:/mosquitto/config",
          "/mnt/moosefs/configs/mosquitto/data:/mosquitto/data",
          "/mnt/moosefs/configs/mosquitto/log:/mosquitto/log",
        ]
      }

      resources {
        cpu    = 100
        memory = 64
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
