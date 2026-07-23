job "gpu-metrics" {
  datacenters = ["home"]
  type        = "system"

  # Only run on nodes with GPUs
  constraint {
    attribute = "${meta.gpu}"
    value     = "true"
  }

  group "gpu-metrics" {
    network {
      port "metrics" {
        static = 9835
      }
    }

    task "gpu-exporter" {
      driver = "docker"

      config {
        image   = "python:3.12-slim"
        command = "python3"
        args    = ["/local/gpu_exporter.py"]
        ports   = ["metrics"]

        # Need access to nvidia-smi on the host
        volumes = [
          "/usr/bin/nvidia-smi:/usr/bin/nvidia-smi:ro",
          "/usr/lib/libnvidia-ml.so.1:/usr/lib/libnvidia-ml.so.1:ro",
          "/usr/lib/libnvidia-ml.so:/usr/lib/libnvidia-ml.so:ro",
        ]

        # Need GPU device access
        privileged = true
        devices = [
          "/dev/nvidiactl",
          "/dev/nvidia-uvm",
          "/dev/nvidia0",
          "/dev/nvidia1",
        ]
      }

      resources {
        cpu    = 100
        memory = 64
      }

      template {
        destination = "local/gpu_exporter.py"
        data        = <<-EOF
#!/usr/bin/env python3
"""Simple NVIDIA GPU Prometheus exporter using nvidia-smi."""
import http.server
import subprocess
import time
import sys

def get_gpu_metrics():
    """Query nvidia-smi and return prometheus metrics."""
    try:
        result = subprocess.run(
            ['nvidia-smi', '--query-gpu=index,name,uuid,utilization.gpu,utilization.memory,memory.used,memory.total,temperature.gpu,power.draw,power.limit,fan.speed,pstate',
             '--format=csv,noheader,nounits'],
            capture_output=True, text=True, timeout=10
        )
        if result.returncode != 0:
            return f"# nvidia-smi error: {result.stderr}\n"
    except Exception as e:
        return f"# nvidia-smi error: {e}\n"

    lines = []
    lines.append("# HELP nvidia_gpu_utilization_percent GPU utilization percentage")
    lines.append("# TYPE nvidia_gpu_utilization_percent gauge")
    lines.append("# HELP nvidia_gpu_memory_used_bytes GPU memory used in bytes")
    lines.append("# TYPE nvidia_gpu_memory_used_bytes gauge")
    lines.append("# HELP nvidia_gpu_memory_total_bytes GPU memory total in bytes")
    lines.append("# TYPE nvidia_gpu_memory_total_bytes gauge")
    lines.append("# HELP nvidia_gpu_memory_utilization_percent GPU memory utilization percentage")
    lines.append("# TYPE nvidia_gpu_memory_utilization_percent gauge")
    lines.append("# HELP nvidia_gpu_temperature_celsius GPU temperature in Celsius")
    lines.append("# TYPE nvidia_gpu_temperature_celsius gauge")
    lines.append("# HELP nvidia_gpu_power_draw_watts GPU power draw in watts")
    lines.append("# TYPE nvidia_gpu_power_draw_watts gauge")
    lines.append("# HELP nvidia_gpu_power_limit_watts GPU power limit in watts")
    lines.append("# TYPE nvidia_gpu_power_limit_watts gauge")
    lines.append("# HELP nvidia_gpu_fan_speed_percent GPU fan speed percentage")
    lines.append("# TYPE nvidia_gpu_fan_speed_percent gauge")

    for row in result.stdout.strip().split('\n'):
        if not row.strip():
            continue
        parts = [p.strip() for p in row.split(',')]
        if len(parts) < 12:
            continue

        idx, name, uuid, gpu_util, mem_util, mem_used, mem_total, temp, power, power_lim, fan, pstate = parts[:12]

        # Clean up values - handle [N/A] etc
        labels = f'gpu="{idx}",name="{name}",uuid="{uuid}"'

        def safe_float(v):
            try:
                return float(v)
            except (ValueError, TypeError):
                return None

        gpu_util_v = safe_float(gpu_util)
        mem_util_v = safe_float(mem_util)
        mem_used_v = safe_float(mem_used)
        mem_total_v = safe_float(mem_total)
        temp_v = safe_float(temp)
        power_v = safe_float(power)
        power_lim_v = safe_float(power_lim)
        fan_v = safe_float(fan)

        if gpu_util_v is not None:
            lines.append(f'nvidia_gpu_utilization_percent{{{labels}}} {gpu_util_v}')
        if mem_util_v is not None:
            lines.append(f'nvidia_gpu_memory_utilization_percent{{{labels}}} {mem_util_v}')
        if mem_used_v is not None:
            lines.append(f'nvidia_gpu_memory_used_bytes{{{labels}}} {mem_used_v * 1048576}')
        if mem_total_v is not None:
            lines.append(f'nvidia_gpu_memory_total_bytes{{{labels}}} {mem_total_v * 1048576}')
        if temp_v is not None:
            lines.append(f'nvidia_gpu_temperature_celsius{{{labels}}} {temp_v}')
        if power_v is not None:
            lines.append(f'nvidia_gpu_power_draw_watts{{{labels}}} {power_v}')
        if power_lim_v is not None:
            lines.append(f'nvidia_gpu_power_limit_watts{{{labels}}} {power_lim_v}')
        if fan_v is not None:
            lines.append(f'nvidia_gpu_fan_speed_percent{{{labels}}} {fan_v}')

    return '\n'.join(lines) + '\n'


class MetricsHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/metrics':
            metrics = get_gpu_metrics()
            self.send_response(200)
            self.send_header('Content-Type', 'text/plain; charset=utf-8')
            self.end_headers()
            self.wfile.write(metrics.encode())
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass  # Suppress access logs


if __name__ == '__main__':
    port = 9835
    print(f"GPU metrics exporter listening on :{port}", flush=True)
    server = http.server.HTTPServer(('0.0.0.0', port), MetricsHandler)
    server.serve_forever()
EOF
      }

      service {
        name = "gpu-metrics"
        port = "metrics"
        tags = ["metrics", "gpu", "prometheus"]
      }
    }
  }
}
