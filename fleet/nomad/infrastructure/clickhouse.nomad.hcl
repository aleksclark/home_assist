job "clickhouse" {
  datacenters = ["home"]
  type        = "service"

  # One group per node for the 3-replica cluster
  # Each group is pinned to its specific node for local disk access

  # ─── Node 2 (192.168.0.24) ───
  group "clickhouse-node2" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-2"
    }

    network {
      mode = "host"
      port "native" { static = 9000 }
      port "http"   { static = 8124 }
      port "keeper" { static = 9181 }
      port "raft"   { static = 9234 }
    }

    volume "moosefs-configs" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = true
    }

    task "clickhouse" {
      driver = "docker"

      config {
        image        = "clickhouse/clickhouse-server:25.5.6"
        network_mode = "host"

        volumes = [
          "/data/clickhouse:/var/lib/clickhouse",
          "/data/clickhouse-keeper:/var/lib/clickhouse-keeper",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/config.xml:/etc/clickhouse-server/config.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/users.xml:/etc/clickhouse-server/users.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/cluster.xml:/etc/clickhouse-server/config.d/cluster.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/keeper.xml:/etc/clickhouse-server/config.d/keeper.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/node-2/macros.xml:/etc/clickhouse-server/config.d/macros.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/node-2/performance.xml:/etc/clickhouse-server/config.d/performance.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse/custom-function.xml:/etc/clickhouse-server/custom-function.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse/user_scripts:/var/lib/clickhouse/user_scripts",
        ]

        ulimit {
          nproc  = "65535"
          nofile = "262144:262144"
        }
      }

      env {
        CLICKHOUSE_SKIP_USER_SETUP = "1"
        CH_KEEPER_SERVER_ID        = "1"
      }

      resources {
        cpu    = 200
        memory = 128
        memory_max = 4096
      }

      service {
        name     = "clickhouse"
        provider = "nomad"
        port     = "http"
        tags     = ["clickhouse", "node-2"]

        check {
          type     = "http"
          port     = "http"
          path     = "/ping"
          interval = "15s"
          timeout  = "5s"
        }
      }
    }

    update {
      min_healthy_time  = "30s"
      healthy_deadline  = "10m"
      progress_deadline = "15m"
    }
  }

  # ─── Node 3 (192.168.0.89) ───
  group "clickhouse-node3" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-3"
    }

    network {
      mode = "host"
      port "native" { static = 9000 }
      port "http"   { static = 8124 }
      port "keeper" { static = 9181 }
      port "raft"   { static = 9234 }
    }

    volume "moosefs-configs" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = true
    }

    task "clickhouse" {
      driver = "docker"

      config {
        image        = "clickhouse/clickhouse-server:25.5.6"
        network_mode = "host"

        volumes = [
          "/data/clickhouse:/var/lib/clickhouse",
          "/data/clickhouse-keeper:/var/lib/clickhouse-keeper",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/config.xml:/etc/clickhouse-server/config.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/users.xml:/etc/clickhouse-server/users.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/cluster.xml:/etc/clickhouse-server/config.d/cluster.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/keeper.xml:/etc/clickhouse-server/config.d/keeper.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/node-3/macros.xml:/etc/clickhouse-server/config.d/macros.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/node-3/performance.xml:/etc/clickhouse-server/config.d/performance.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse/custom-function.xml:/etc/clickhouse-server/custom-function.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse/user_scripts:/var/lib/clickhouse/user_scripts",
        ]

        ulimit {
          nproc  = "65535"
          nofile = "262144:262144"
        }
      }

      env {
        CLICKHOUSE_SKIP_USER_SETUP = "1"
        CH_KEEPER_SERVER_ID        = "2"
      }

      resources {
        cpu    = 2000
        memory = 12288
      }

      service {
        name     = "clickhouse"
        provider = "nomad"
        port     = "http"
        tags     = ["clickhouse", "node-3"]

        check {
          type     = "http"
          port     = "http"
          path     = "/ping"
          interval = "15s"
          timeout  = "5s"
        }
      }
    }

    update {
      min_healthy_time  = "30s"
      healthy_deadline  = "10m"
      progress_deadline = "15m"
    }
  }

  # ─── Node 4 (192.168.0.41) ───
  group "clickhouse-node4" {
    count = 1

    constraint {
      attribute = "${node.unique.name}"
      value     = "node-4"
    }

    network {
      mode = "host"
      port "native" { static = 9000 }
      port "http"   { static = 8124 }
      port "keeper" { static = 9181 }
      port "raft"   { static = 9234 }
    }

    volume "moosefs-configs" {
      type      = "host"
      source    = "moosefs-configs"
      read_only = true
    }

    task "clickhouse" {
      driver = "docker"

      config {
        image        = "clickhouse/clickhouse-server:25.5.6"
        network_mode = "host"

        volumes = [
          "/data/clickhouse:/var/lib/clickhouse",
          "/data/clickhouse-keeper:/var/lib/clickhouse-keeper",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/config.xml:/etc/clickhouse-server/config.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/users.xml:/etc/clickhouse-server/users.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/cluster.xml:/etc/clickhouse-server/config.d/cluster.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/keeper.xml:/etc/clickhouse-server/config.d/keeper.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/node-4/macros.xml:/etc/clickhouse-server/config.d/macros.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse-cluster/node-4/performance.xml:/etc/clickhouse-server/config.d/performance.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse/custom-function.xml:/etc/clickhouse-server/custom-function.xml:ro",
          "/mnt/moosefs/configs/signoz/clickhouse/user_scripts:/var/lib/clickhouse/user_scripts",
        ]

        ulimit {
          nproc  = "65535"
          nofile = "262144:262144"
        }
      }

      env {
        CLICKHOUSE_SKIP_USER_SETUP = "1"
        CH_KEEPER_SERVER_ID        = "3"
      }

      resources {
        cpu    = 2000
        memory = 16384
      }

      service {
        name     = "clickhouse"
        provider = "nomad"
        port     = "http"
        tags     = ["clickhouse", "node-4"]

        check {
          type     = "http"
          port     = "http"
          path     = "/ping"
          interval = "15s"
          timeout  = "5s"
        }
      }
    }

    update {
      min_healthy_time  = "30s"
      healthy_deadline  = "10m"
      progress_deadline = "15m"
    }
  }
}
