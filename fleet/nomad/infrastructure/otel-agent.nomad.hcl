job "otel-agent" {
  datacenters = ["home"]
  type        = "system"

  group "otel-agent" {
    network {
      mode = "host"
      # Agent OTLP ports for local app instrumentation
      port "otlp-grpc" {
        static = 4327
      }
      port "otlp-http" {
        static = 4328
      }
      port "health" {
        static = 13134
      }
    }

    task "otel-collector" {
      driver = "docker"

      # Numeric UID required — image has no /etc/passwd entry for "root"
      user = "0:0"

      config {
        image        = "otel/opentelemetry-collector-contrib:0.123.0"
        network_mode = "host"
        privileged   = true

        args = [
          "--config=/etc/otelcol-contrib/config.yaml",
        ]

        volumes = [
          "local/config.yaml:/etc/otelcol-contrib/config.yaml:ro",
          "/:/hostfs:ro",
          "/var/run/docker.sock:/var/run/docker.sock:ro",
          "/var/lib/docker/containers:/var/lib/docker/containers:ro",
        ]
      }

      # Use Nomad's env interpolation for the node name
      env {
        OTEL_RESOURCE_ATTRIBUTES = "host.name=${node.unique.name}"
        NODE_IP                  = "${attr.unique.network.ip-address}"
      }

      template {
        destination     = "local/config.yaml"
        left_delimiter  = "{{{"
        right_delimiter = "}}}"
        data            = <<-YAML
receivers:
  # Local OTLP endpoint for app instrumentation
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4327
      http:
        endpoint: 0.0.0.0:4328

  # Host-level metrics
  hostmetrics:
    collection_interval: 60s
    root_path: /hostfs
    scrapers:
      cpu: {}
      disk: {}
      load: {}
      filesystem:
        exclude_mount_points:
          mount_points: ["/hostfs/proc", "/hostfs/sys", "/hostfs/dev", "/hostfs/run"]
          match_type: strict
      memory: {}
      network: {}
      paging: {}
      process:
        mute_process_name_error: true
        mute_process_exe_error: true
        mute_process_io_error: true
        mute_process_user_error: true
      processes: {}

  # Docker container stats
  docker_stats:
    endpoint: unix:///var/run/docker.sock
    collection_interval: 30s
    timeout: 20s
    api_version: "1.40"

  # Nomad metrics
  prometheus:
    config:
      scrape_configs:
        - job_name: nomad
          metrics_path: /v1/metrics
          params:
            format: ["prometheus"]
          static_configs:
            - targets: ["$${env:NODE_IP}:4646"]
        - job_name: otel-agent
          static_configs:
            - targets: ["127.0.0.1:8889"]

  # Docker container logs via file
  filelog:
    include: ["/var/lib/docker/containers/*/*-json.log"]
    start_at: end
    include_file_name: false
    include_file_path: true
    operators:
      - id: container-parser
        type: container
        format: docker
        add_metadata_from_filepath: false

processors:
  batch:
    send_batch_size: 1000
    timeout: 10s
  resourcedetection:
    detectors: [env, system, docker]
    timeout: 2s
    system:
      hostname_sources: [os]
  memory_limiter:
    check_interval: 5s
    limit_mib: 256
    spike_limit_mib: 64

exporters:
  otlp:
    endpoint: "192.168.0.24:4317"
    tls:
      insecure: true

extensions:
  health_check:
    endpoint: 0.0.0.0:13134

service:
  telemetry:
    logs:
      encoding: json
    metrics:
      readers:
        - pull:
            exporter:
              prometheus:
                host: "0.0.0.0"
                port: 8889
  extensions: [health_check]
  pipelines:
    traces:
      receivers: [otlp]
      processors: [memory_limiter, resourcedetection, batch]
      exporters: [otlp]
    metrics:
      receivers: [otlp, hostmetrics, docker_stats]
      processors: [memory_limiter, resourcedetection, batch]
      exporters: [otlp]
    metrics/nomad:
      receivers: [prometheus]
      processors: [memory_limiter, resourcedetection, batch]
      exporters: [otlp]
    logs:
      receivers: [otlp, filelog]
      processors: [memory_limiter, resourcedetection, batch]
      exporters: [otlp]
        YAML
      }

      resources {
        cpu    = 200
        memory = 256
      }

      service {
        name     = "otel-agent"
        provider = "nomad"
        port     = "health"
        tags     = ["monitoring"]

        check {
          type     = "http"
          port     = "health"
          path     = "/health"
          interval = "30s"
          timeout  = "5s"
        }
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
