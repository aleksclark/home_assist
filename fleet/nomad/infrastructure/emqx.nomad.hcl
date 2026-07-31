job "emqx" {
  datacenters = ["home"]
  type        = "service"

  # 3-node EMQX cluster on node-1, node-3, node-6
  # Static seeds: 192.168.0.23 (node-1), 192.168.0.89 (node-3), 192.168.0.99 (node-6)

  group "emqx-1" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-1"
    }

    network {
      mode = "host"
      port "mqtt" { static = 1883 }
      port "mqttssl" { static = 8883 }
      port "ws" { static = 8083 }
      port "dashboard" { static = 18083 }
      port "ekka" { static = 4370 }
    }

    task "emqx" {
      driver = "docker"

      config {
        image        = "emqx:5.8.8"
        network_mode = "host"
        ports        = ["mqtt", "mqttssl", "ws", "dashboard", "ekka"]

        volumes = [
          "/mnt/moosefs/configs/emqx/node1/data:/opt/emqx/data",
          "/mnt/moosefs/configs/emqx/node1/log:/opt/emqx/log",
        ]
      }

      env {
        EMQX_NODE__NAME                  = "emqx@192.168.0.23"
        EMQX_CLUSTER__DISCOVERY_STRATEGY = "static"
        EMQX_CLUSTER__STATIC__SEEDS      = "[emqx@192.168.0.23,emqx@192.168.0.89,emqx@192.168.0.99]"
        EMQX_DASHBOARD__DEFAULT_USERNAME = "admin"
        EMQX_DASHBOARD__DEFAULT_PASSWORD = "emqx-fleet-2026"
        EMQX_LOG__CONSOLE__LEVEL         = "warning"
      }

      resources {
        cpu        = 500
        memory     = 512
        memory_max = 1024
      }

      service {
        name     = "emqx-mqtt"
        port     = "mqtt"
        provider = "nomad"
        check {
          name     = "mqtt-tcp"
          type     = "tcp"
          port     = "mqtt"
          interval = "15s"
          timeout  = "5s"
        }
      }

      service {
        name     = "emqx-dashboard"
        port     = "dashboard"
        provider = "nomad"
        check {
          name     = "dashboard-http"
          type     = "http"
          path     = "/status"
          port     = "dashboard"
          interval = "30s"
          timeout  = "5s"
        }
      }
    }
  }

  group "emqx-2" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-3"
    }

    network {
      mode = "host"
      port "mqtt" { static = 1883 }
      port "mqttssl" { static = 8883 }
      port "ws" { static = 8083 }
      port "dashboard" { static = 18083 }
      port "ekka" { static = 4370 }
    }

    task "emqx" {
      driver = "docker"

      config {
        image        = "emqx:5.8.8"
        network_mode = "host"
        ports        = ["mqtt", "mqttssl", "ws", "dashboard", "ekka"]

        volumes = [
          "/mnt/moosefs/configs/emqx/node2/data:/opt/emqx/data",
          "/mnt/moosefs/configs/emqx/node2/log:/opt/emqx/log",
        ]
      }

      env {
        EMQX_NODE__NAME                  = "emqx@192.168.0.89"
        EMQX_CLUSTER__DISCOVERY_STRATEGY = "static"
        EMQX_CLUSTER__STATIC__SEEDS      = "[emqx@192.168.0.23,emqx@192.168.0.89,emqx@192.168.0.99]"
        EMQX_DASHBOARD__DEFAULT_USERNAME = "admin"
        EMQX_DASHBOARD__DEFAULT_PASSWORD = "emqx-fleet-2026"
        EMQX_LOG__CONSOLE__LEVEL         = "warning"
      }

      resources {
        cpu        = 500
        memory     = 512
        memory_max = 1024
      }

      service {
        name     = "emqx-mqtt"
        port     = "mqtt"
        provider = "nomad"
        check {
          name     = "mqtt-tcp"
          type     = "tcp"
          port     = "mqtt"
          interval = "15s"
          timeout  = "5s"
        }
      }

      service {
        name     = "emqx-dashboard"
        port     = "dashboard"
        provider = "nomad"
        check {
          name     = "dashboard-http"
          type     = "http"
          path     = "/status"
          port     = "dashboard"
          interval = "30s"
          timeout  = "5s"
        }
      }
    }
  }

  group "emqx-3" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-6"
    }

    network {
      mode = "host"
      port "mqtt" { static = 1883 }
      port "mqttssl" { static = 8883 }
      port "ws" { static = 8083 }
      port "dashboard" { static = 18083 }
      port "ekka" { static = 4370 }
    }

    task "emqx" {
      driver = "docker"

      config {
        image        = "emqx:5.8.8"
        network_mode = "host"
        ports        = ["mqtt", "mqttssl", "ws", "dashboard", "ekka"]

        volumes = [
          "/mnt/moosefs/configs/emqx/node3/data:/opt/emqx/data",
          "/mnt/moosefs/configs/emqx/node3/log:/opt/emqx/log",
        ]
      }

      env {
        EMQX_NODE__NAME                  = "emqx@192.168.0.99"
        EMQX_CLUSTER__DISCOVERY_STRATEGY = "static"
        EMQX_CLUSTER__STATIC__SEEDS      = "[emqx@192.168.0.23,emqx@192.168.0.89,emqx@192.168.0.99]"
        EMQX_DASHBOARD__DEFAULT_USERNAME = "admin"
        EMQX_DASHBOARD__DEFAULT_PASSWORD = "emqx-fleet-2026"
        EMQX_LOG__CONSOLE__LEVEL         = "warning"
      }

      resources {
        cpu        = 500
        memory     = 512
        memory_max = 1024
      }

      service {
        name     = "emqx-mqtt"
        port     = "mqtt"
        provider = "nomad"
        check {
          name     = "mqtt-tcp"
          type     = "tcp"
          port     = "mqtt"
          interval = "15s"
          timeout  = "5s"
        }
      }

      service {
        name     = "emqx-dashboard"
        port     = "dashboard"
        provider = "nomad"
        check {
          name     = "dashboard-http"
          type     = "http"
          path     = "/status"
          port     = "dashboard"
          interval = "30s"
          timeout  = "5s"
        }
      }
    }
  }
}

