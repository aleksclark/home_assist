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

  # Docker container stats — with Nomad service tagging
  docker_stats:
    endpoint: unix:///var/run/docker.sock
    collection_interval: 30s
    timeout: 20s
    api_version: "1.40"
    # Nomad env vars → metric resource attributes
    env_vars_to_metric_labels:
      NOMAD_JOB_NAME: nomad.job.name
      NOMAD_TASK_NAME: nomad.task.name
      NOMAD_GROUP_NAME: nomad.group.name
      NOMAD_NAMESPACE: nomad.namespace
    # Nomad Docker labels → metric resource attributes (after Nomad config deploy)
    container_labels_to_metric_labels:
      com.hashicorp.nomad.job_name: service.name
      com.hashicorp.nomad.task_name: nomad.task.name
      com.hashicorp.nomad.node_name: host.name

  # Nomad + infrastructure service metrics
  prometheus:
    config:
      scrape_configs:
        - job_name: nomad
          metrics_path: /v1/metrics
          params:
            format: ["prometheus"]
          static_configs:
            - targets: ["$${env:NODE_IP}:4646"]
              labels:
                service.name: nomad
        - job_name: traefik
          static_configs:
            - targets: ["127.0.0.1:8082"]
              labels:
                service.name: traefik
        - job_name: coredns
          static_configs:
            - targets: ["127.0.0.1:9153"]
              labels:
                service.name: coredns
        - job_name: otel-agent
          static_configs:
            - targets: ["127.0.0.1:8889"]
              labels:
                service.name: otel-agent

  # Docker container logs via file
  filelog:
    include: ["/var/lib/docker/containers/*/*-json.log"]
    start_at: end
    include_file_name: false
    include_file_path: true
    operators:
      # Parse Docker JSON log line: {"log":"...","stream":"...","attrs":{"tag":"..."},"time":"..."}
      - id: docker-json
        type: json_parser
        timestamp:
          parse_from: attributes.time
          layout: '2006-01-02T15:04:05.999999999Z'
          layout_type: gotime
      # Move "log" field to body
      - id: log-to-body
        type: move
        from: attributes.log
        to: body
      # Move "stream" to standard attribute
      - id: stream-to-attr
        type: move
        from: attributes.stream
        to: attributes["log.iostream"]
      # Remove parsed time from attributes
      - id: remove-time
        type: remove
        field: attributes.time
      # Extract container ID from file path for correlation
      - id: extract-container-id
        type: regex_parser
        regex: '/var/lib/docker/containers/(?P<container_id>[a-f0-9]{64})/'
        parse_from: attributes["log.file.path"]
        parse_to: resource
        on_error: send

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
    limit_mib: 300
    spike_limit_mib: 80
  # Extract Nomad service name from Docker container tag in logs
  # Docker daemon adds attrs.tag = container name (e.g. "coredns-2724eb3a-...")
  # Nomad container names follow: {task_name}-{alloc_id}
  transform/logs:
    error_mode: ignore
    log_statements:
      - context: log
        statements:
          - set(resource.attributes["service.name"], attributes["attrs"]["tag"])
            where attributes["attrs"]["tag"] != nil
          - replace_pattern(resource.attributes["service.name"],
              "^([a-zA-Z0-9_-]+)-[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$$",
              "$$1")
            where resource.attributes["service.name"] != nil
          - delete_key(attributes, "attrs")
            where attributes["attrs"] != nil

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
    metrics/infra:
      receivers: [prometheus]
      processors: [memory_limiter, resourcedetection, batch]
      exporters: [otlp]
    logs:
      receivers: [otlp, filelog]
      processors: [memory_limiter, resourcedetection, transform/logs, batch]
      exporters: [otlp]
        YAML
      }

      resources {
        cpu    = 300
        memory = 320
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
