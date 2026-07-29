job "signoz" {
  datacenters = ["home"]
  type        = "service"

  group "signoz" {
    count = 1

    # Run on node-3 (has RAM headroom, close to CH cluster)
    constraint {
      attribute = "${node.unique.name}"
      value     = "node-3"
    }

    update {
      min_healthy_time  = "30s"
      healthy_deadline  = "30m"
      progress_deadline = "35m"
    }

    network {
      mode = "host"
      port "signoz-ui" {
        static = 8080
      }
      port "otel-grpc" {
        static = 4317
      }
      port "otel-http" {
        static = 4318
      }
    }

    # ─── DB Migrator (one-shot) ───

    task "migrator" {
      driver = "docker"
      lifecycle {
        hook = "prestart"
      }

      config {
        image        = "signoz/signoz-otel-collector:v0.144.3"
        network_mode = "host"
        entrypoint   = ["/bin/sh"]
        args         = ["/local/migrate.sh"]
      }

      template {
        destination     = "local/migrate.sh"
        perms           = "755"
        left_delimiter  = "{{{"
        right_delimiter = "}}}"
        data            = <<-EOF
          #!/bin/sh
          echo "Running migrations..."
          /signoz-otel-collector migrate bootstrap &&
          /signoz-otel-collector migrate sync up &&
          /signoz-otel-collector migrate async up
        EOF
      }

      env {
        SIGNOZ_OTEL_COLLECTOR_CLICKHOUSE_DSN         = "tcp://192.168.0.24:9000"
        SIGNOZ_OTEL_COLLECTOR_CLICKHOUSE_CLUSTER      = "cluster"
        SIGNOZ_OTEL_COLLECTOR_CLICKHOUSE_REPLICATION   = "true"
        SIGNOZ_OTEL_COLLECTOR_TIMEOUT                  = "10m"
      }

      resources {
        cpu    = 500
        memory = 1024
      }

      restart {
        attempts = 5
        interval = "10m"
        delay    = "15s"
        mode     = "delay"
      }
    }

    # ─── SigNoz Query Service + Frontend ───

    task "signoz" {
      driver = "docker"

      config {
        image        = "signoz/signoz:v0.120.0"
        network_mode = "host"
      }

      env {
        SIGNOZ_ALERTMANAGER_PROVIDER                   = "signoz"
        SIGNOZ_TELEMETRYSTORE_CLICKHOUSE_DSN           = "tcp://192.168.0.24:9000"
        SIGNOZ_SQLSTORE_PROVIDER                       = "postgres"
        SIGNOZ_SQLSTORE_POSTGRES_DSN                   = "postgres://signoz:signoz_pass_2026@192.168.0.24:5432/signoz?sslmode=disable"
        SIGNOZ_TOKENIZER_JWT_SECRET                    = "clark-fleet-signoz-jwt-secret-2026"
        SIGNOZ_USER_ROOT_ENABLED                       = "true"
        SIGNOZ_USER_ROOT_EMAIL                         = "aleks@clark.team"
        SIGNOZ_USER_ROOT_PASSWORD                      = "FleetAdmin2026!"
        SIGNOZ_USER_ROOT_ORG_NAME                      = "Clark Fleet"
      }

      resources {
        cpu    = 500
        memory = 1024
      }

      service {
        name     = "signoz"
        provider = "nomad"
        port     = "signoz-ui"
        tags     = [
          "traefik.enable=true",
          "traefik.http.routers.signoz.rule=Host(`signoz.fleet.clark.team`)",
          "traefik.http.routers.signoz.entrypoints=websecure",
          "traefik.http.routers.signoz.tls=true",
        ]

        check {
          type     = "http"
          port     = "signoz-ui"
          path     = "/api/v1/health"
          interval = "30s"
          timeout  = "5s"
        }
      }
    }

    # ─── OTel Collector ───

    task "otel-collector" {
      driver = "docker"

      config {
        image        = "signoz/signoz-otel-collector:v0.144.3"
        network_mode = "host"
        entrypoint   = ["/bin/sh"]
        args         = ["/local/start.sh"]

        volumes = [
          "local/otel-collector-config.yaml:/etc/otel-collector-config.yaml:ro",
          "local/manager-config.yaml:/etc/manager-config.yaml:ro",
        ]
      }

      template {
        destination     = "local/start.sh"
        perms           = "755"
        left_delimiter  = "{{{"
        right_delimiter = "}}}"
        data            = <<-EOF
          #!/bin/sh
          echo "Waiting for ClickHouse and migrations..."
          i=0
          while [ $i -lt 120 ]; do
            /signoz-otel-collector migrate sync check 2>/dev/null && break
            i=$((i + 1))
            sleep 3
          done
          echo "Starting collector..."
          exec /signoz-otel-collector \
            --config=/etc/otel-collector-config.yaml \
            --copy-path=/var/tmp/collector-config.yaml
        EOF
      }

      env {
        OTEL_RESOURCE_ATTRIBUTES                       = "host.name=signoz-host,os.type=linux"
        LOW_CARDINAL_EXCEPTION_GROUPING                = "false"
        SIGNOZ_OTEL_COLLECTOR_CLICKHOUSE_DSN           = "tcp://192.168.0.24:9000"
        SIGNOZ_OTEL_COLLECTOR_CLICKHOUSE_CLUSTER       = "cluster"
        SIGNOZ_OTEL_COLLECTOR_CLICKHOUSE_REPLICATION   = "true"
        SIGNOZ_OTEL_COLLECTOR_TIMEOUT                  = "10m"
      }

      template {
        destination     = "local/otel-collector-config.yaml"
        left_delimiter  = "{{{"
        right_delimiter = "}}}"
        data            = <<-YAML
connectors:
  signozmeter:
    metrics_flush_interval: 1h
    dimensions:
      - name: service.name
      - name: deployment.environment
      - name: host.name
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318
  prometheus:
    config:
      global:
        scrape_interval: 60s
      scrape_configs:
        - job_name: otel-collector
          static_configs:
          - targets:
              - localhost:8888
            labels:
              job_name: otel-collector
processors:
  batch:
    send_batch_size: 10000
    send_batch_max_size: 11000
    timeout: 10s
  batch/meter:
    send_batch_max_size: 25000
    send_batch_size: 20000
    timeout: 1s
  resourcedetection:
    detectors: [env, system]
    timeout: 2s
  signozspanmetrics/delta:
    metrics_exporter: signozclickhousemetrics
    metrics_flush_interval: 60s
    latency_histogram_buckets: [100us, 1ms, 2ms, 6ms, 10ms, 50ms, 100ms, 250ms, 500ms, 1000ms, 1400ms, 2000ms, 5s, 10s, 20s, 40s, 60s]
    dimensions_cache_size: 100000
    aggregation_temporality: AGGREGATION_TEMPORALITY_DELTA
    enable_exp_histogram: true
    dimensions:
      - name: service.namespace
        default: default
      - name: deployment.environment
        default: default
      - name: signoz.collector.id
      - name: service.version
      - name: k8s.cluster.name
      - name: k8s.node.name
      - name: k8s.namespace.name
      - name: host.name
      - name: host.type
      - name: container.name
extensions:
  health_check:
    endpoint: 0.0.0.0:13133
  pprof:
    endpoint: 0.0.0.0:1777
exporters:
  clickhousetraces:
    datasource: tcp://192.168.0.24:9000/signoz_traces
    low_cardinal_exception_grouping: false
    use_new_schema: true
  signozclickhousemetrics:
    dsn: tcp://192.168.0.24:9000/signoz_metrics
  clickhouselogsexporter:
    dsn: tcp://192.168.0.24:9000/signoz_logs
    timeout: 10s
    use_new_schema: true
  signozclickhousemeter:
    dsn: tcp://192.168.0.24:9000/signoz_meter
    timeout: 45s
    sending_queue:
      enabled: false
  metadataexporter:
    cache:
      provider: in_memory
    dsn: tcp://192.168.0.24:9000/signoz_metadata
    enabled: true
    timeout: 45s
service:
  telemetry:
    logs:
      encoding: json
  extensions:
    - health_check
    - pprof
  pipelines:
    traces:
      receivers: [otlp]
      processors: [signozspanmetrics/delta, batch]
      exporters: [clickhousetraces, metadataexporter, signozmeter]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [signozclickhousemetrics, metadataexporter, signozmeter]
    metrics/prometheus:
      receivers: [prometheus]
      processors: [batch]
      exporters: [signozclickhousemetrics, metadataexporter, signozmeter]
    logs:
      receivers: [otlp]
      processors: [batch]
      exporters: [clickhouselogsexporter, metadataexporter, signozmeter]
    metrics/meter:
      receivers: [signozmeter]
      processors: [batch/meter]
      exporters: [signozclickhousemeter]
        YAML
      }

      template {
        destination = "local/manager-config.yaml"
        data        = <<-YAML
server_endpoint: ws://127.0.0.1:4320/v1/opamp
        YAML
      }

      resources {
        cpu    = 500
        memory = 1536
      }

      service {
        name     = "signoz-otel"
        provider = "nomad"
        port     = "otel-grpc"
        tags     = ["otel-collector"]
      }
    }

    restart {
      attempts = 5
      interval = "10m"
      delay    = "30s"
      mode     = "delay"
    }
  }
}
