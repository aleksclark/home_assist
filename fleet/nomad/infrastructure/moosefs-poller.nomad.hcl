job "moosefs-poller" {
  datacenters = ["home"]
  type        = "service"

  group "poller" {
    count = 1

    # Pin to master node where mfscli is installed
    constraint {
      attribute = "${node.unique.name}"
      value     = "node-1"
    }

    network {
      mode = "host"
    }

    task "poller" {
      driver = "docker"

      config {
        image        = "python:3.14-slim"
        network_mode = "host"
        command      = "python3"
        args         = ["/local/moosefs-poller.py"]

        # Mount host mfscli binary + libraries into the container
        volumes = [
          "/usr/bin/mfscli:/usr/local/bin/mfscli:ro",
          "/usr/lib/python3.14:/usr/lib/python3.14:ro",
        ]
      }

      env {
        MOOSEFS_MASTER_HOST    = "192.168.0.89"
        MOOSEFS_POLL_INTERVAL  = "60"
        OTEL_EXPORTER_OTLP_ENDPOINT = "http://192.168.0.24:4318"
      }

      template {
        destination     = "local/moosefs-poller.py"
        left_delimiter  = "{{{"
        right_delimiter = "}}}"
        data            = <<-PYTHON
#!/usr/bin/env python3
"""MooseFS metrics poller — scrapes mfscli and pushes OTLP metrics to SigNoz."""

import json
import os
import subprocess
import sys
import time
import logging
import urllib.request
from dataclasses import dataclass, field

OTEL_ENDPOINT = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "http://192.168.0.24:4318")
MFSCLI_HOST = os.environ.get("MOOSEFS_MASTER_HOST", "mfsmaster")
POLL_INTERVAL = int(os.environ.get("MOOSEFS_POLL_INTERVAL", "60"))
SERVICE_NAME = "moosefs"

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
log = logging.getLogger("moosefs-poller")

def run_mfscli(*flags):
    cmd = ["python3", "/usr/local/bin/mfscli", "-H", MFSCLI_HOST] + list(flags)
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    if result.returncode != 0:
        raise RuntimeError(f"mfscli {' '.join(flags)} failed: {result.stderr}")
    return result.stdout

@dataclass
class Metric:
    name: str
    description: str
    unit: str
    value: float
    attributes: dict = field(default_factory=dict)

def parse_cluster_summary(output):
    metrics = []
    kv = {}
    for line in output.splitlines():
        if not line.startswith("cluster summary:"):
            continue
        parts = line.split("\t")
        if len(parts) >= 3:
            kv[parts[1].strip()] = parts[2].strip()

    mapping = [
        ("moosefs.cluster.total_space", "Total cluster space", "By", "total space"),
        ("moosefs.cluster.avail_space", "Available cluster space", "By", "avail space"),
        ("moosefs.cluster.free_space", "Free cluster space", "By", "free space"),
        ("moosefs.cluster.trash_space", "Trash space", "By", "trash space"),
        ("moosefs.cluster.trash_files", "Trash files", "{files}", "trash files"),
        ("moosefs.cluster.fs_objects", "FS objects", "{objects}", "all fs objects"),
        ("moosefs.cluster.directories", "Directories", "{dirs}", "directories"),
        ("moosefs.cluster.files", "Files", "{files}", "files"),
        ("moosefs.cluster.chunks", "Chunks", "{chunks}", "chunks"),
        ("moosefs.cluster.chunk_copies", "Chunk copies", "{copies}", "all full chunk copies"),
        ("moosefs.master.ram_used", "Master RAM", "By", "RAM used"),
        ("moosefs.master.cpu_system", "Master CPU sys", "1", "CPU used (system)"),
        ("moosefs.master.cpu_user", "Master CPU user", "1", "CPU used (user)"),
        ("moosefs.master.last_save_duration", "Save duration", "s", "last save duration"),
    ]
    for name, desc, unit, key in mapping:
        if key in kv:
            try:
                metrics.append(Metric(name, desc, unit, float(kv[key])))
            except ValueError:
                pass

    if "total space" in kv and "avail space" in kv:
        try:
            total = float(kv["total space"])
            avail = float(kv["avail space"])
            if total > 0:
                metrics.append(Metric("moosefs.cluster.space_used_percent",
                    "Space utilization %", "%", ((total - avail) / total) * 100))
        except ValueError:
            pass
    return metrics

def parse_chunk_health(output):
    metrics = []
    for line in output.splitlines():
        if "all chunks matrix" not in line:
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        key = parts[1].strip().replace("chunkclass ", "")
        val = parts[-1].strip()
        if key in ("missing", "endangered", "undergoal", "stable", "overgoal"):
            try:
                metrics.append(Metric(f"moosefs.chunks.{key}", f"Chunks {key}",
                    "{chunks}", float(val), {"state": key}))
            except ValueError:
                pass
    return metrics

def parse_check_loop(output):
    metrics = []
    for line in output.splitlines():
        if not line.startswith("check loop"):
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        key = parts[1].strip().replace("-", "_").replace(" ", "_")
        val = parts[2].strip()
        if key in ("under_goal_files", "missing_files", "missing_chunks", "under_goal_chunks"):
            try:
                metrics.append(Metric(f"moosefs.check.{key}", f"Check: {key}",
                    "{count}", float(val)))
            except ValueError:
                pass
    return metrics

def parse_chunkservers(output):
    metrics = []
    for line in output.splitlines():
        if not line.startswith("chunkservers:"):
            continue
        parts = line.split("\t")
        if len(parts) < 12:
            continue
        ip = parts[1].strip()
        num_chunks = parts[6].strip()
        status = parts[7].strip()
        used_str = parts[10].strip()
        total_str = parts[11].strip()
        attrs = {"host": ip, "status": status}
        try:
            metrics.append(Metric("moosefs.chunkserver.chunks", "CS chunks",
                "{chunks}", float(num_chunks), attrs))
            used = float(used_str)
            total = float(total_str)
            metrics.append(Metric("moosefs.chunkserver.used_space", "CS used",
                "By", used, attrs))
            metrics.append(Metric("moosefs.chunkserver.total_space", "CS total",
                "By", total, attrs))
            if total > 0:
                metrics.append(Metric("moosefs.chunkserver.used_percent", "CS %",
                    "%", (used / total) * 100, attrs))
        except ValueError:
            pass
        metrics.append(Metric("moosefs.chunkserver.healthy", "CS healthy",
            "1", 1.0 if status in ("Normal", "Rebalance") else 0.0, attrs))
    return metrics

def parse_disks(output):
    metrics = []
    for line in output.splitlines():
        if not line.startswith("disks:"):
            continue
        parts = line.split("\t")
        if len(parts) < 15:
            continue
        disk_id = parts[1].strip()
        id_parts = disk_id.split(":")
        host = id_parts[0] if len(id_parts) >= 3 else disk_id
        path = ":".join(id_parts[2:]) if len(id_parts) >= 3 else ""
        chunks = parts[2].strip()
        error_status = parts[3].strip()
        disk_status = parts[4].strip()
        attrs = {"host": host, "path": path}
        try:
            metrics.append(Metric("moosefs.disk.chunks", "Disk chunks",
                "{chunks}", float(chunks), attrs))
        except ValueError:
            pass
        metrics.append(Metric("moosefs.disk.has_errors", "Disk errors",
            "1", 0.0 if error_status == "no errors" else 1.0, attrs))
        metrics.append(Metric("moosefs.disk.healthy", "Disk healthy",
            "1", 1.0 if disk_status == "ok" else 0.0, attrs))
        try:
            used = float(parts[-2].strip())
            total = float(parts[-1].strip())
            metrics.append(Metric("moosefs.disk.used_space", "Disk used",
                "By", used, attrs))
            metrics.append(Metric("moosefs.disk.total_space", "Disk total",
                "By", total, attrs))
            if total > 0:
                metrics.append(Metric("moosefs.disk.used_percent", "Disk %",
                    "%", (used / total) * 100, attrs))
        except (ValueError, IndexError):
            pass
    return metrics

def parse_memory(output):
    metrics = []
    for line in output.splitlines():
        if "memory usage detailed info:" not in line:
            continue
        parts = line.split("\t")
        if len(parts) < 4:
            continue
        component = parts[1].strip().lower().replace(" ", "_")
        try:
            metrics.append(Metric("moosefs.master.memory.used", f"Mem {component}",
                "By", float(parts[2].strip()), {"component": component}))
            metrics.append(Metric("moosefs.master.memory.allocated", f"Alloc {component}",
                "By", float(parts[3].strip()), {"component": component}))
        except ValueError:
            pass
    return metrics

def build_otlp_payload(metrics, timestamp_ns):
    gauge_metrics = {}
    for m in metrics:
        if m.name not in gauge_metrics:
            gauge_metrics[m.name] = {"name": m.name, "description": m.description,
                "unit": m.unit, "data_points": []}
        dp = {"timeUnixNano": str(timestamp_ns), "asDouble": m.value}
        if m.attributes:
            dp["attributes"] = [{"key": k, "value": {"stringValue": str(v)}}
                for k, v in m.attributes.items()]
        gauge_metrics[m.name]["data_points"].append(dp)

    return {"resourceMetrics": [{"resource": {"attributes": [
        {"key": "service.name", "value": {"stringValue": SERVICE_NAME}},
        {"key": "host.name", "value": {"stringValue": MFSCLI_HOST}},
    ]}, "scopeMetrics": [{"scope": {"name": "moosefs-poller", "version": "1.0.0"},
        "metrics": [{"name": d["name"], "description": d["description"],
            "unit": d["unit"], "gauge": {"dataPoints": d["data_points"]}}
            for d in gauge_metrics.values()]}]}]}

def push_otlp(payload):
    url = f"{OTEL_ENDPOINT}/v1/metrics"
    data = json.dumps(payload).encode()
    req = urllib.request.Request(url, data=data,
        headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            return resp.status < 300
    except Exception as e:
        log.error("OTLP push failed: %s", e)
        return False

def collect_all():
    metrics = []
    try:
        sig = run_mfscli("-SIG")
        metrics.extend(parse_cluster_summary(sig))
        metrics.extend(parse_chunk_health(sig))
        metrics.extend(parse_check_loop(sig))
    except Exception as e:
        log.error("cluster summary: %s", e)
    try:
        metrics.extend(parse_chunkservers(run_mfscli("-SCS")))
    except Exception as e:
        log.error("chunkservers: %s", e)
    try:
        metrics.extend(parse_disks(run_mfscli("-SHD")))
    except Exception as e:
        log.error("disks: %s", e)
    try:
        metrics.extend(parse_memory(run_mfscli("-SMU")))
    except Exception as e:
        log.error("memory: %s", e)
    return metrics

def main():
    log.info("MooseFS poller starting — master=%s endpoint=%s interval=%ds",
        MFSCLI_HOST, OTEL_ENDPOINT, POLL_INTERVAL)
    while True:
        try:
            start = time.monotonic()
            metrics = collect_all()
            ts = int(time.time() * 1e9)
            if metrics:
                ok = push_otlp(build_otlp_payload(metrics, ts))
                log.info("Collected %d metrics, push %s (%.1fs)",
                    len(metrics), "OK" if ok else "FAILED", time.monotonic() - start)
            else:
                log.warning("No metrics collected")
        except Exception as e:
            log.error("Collection failed: %s", e)
        time.sleep(POLL_INTERVAL)

if __name__ == "__main__":
    main()
        PYTHON
      }

      resources {
        cpu    = 100
        memory = 128
      }

      restart {
        attempts = 10
        interval = "30m"
        delay    = "30s"
        mode     = "delay"
      }
    }
  }
}
