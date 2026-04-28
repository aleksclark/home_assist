#!/usr/bin/env python3
"""MooseFS metrics poller — scrapes mfscli and pushes OTLP metrics to SigNoz.

Runs every 60s, collects:
  - Cluster summary (space, chunks, objects, master CPU/RAM)
  - Chunkserver status (per-CS space, chunk counts, status)
  - Disk health (per-disk space, errors, I/O stats)
  - Chunk redundancy matrix (missing, endangered, undergoal, stable, overgoal)
  - Memory usage breakdown

Exports via OTLP/HTTP (protobuf or JSON) to the local otel-agent or direct to SigNoz.
"""

import json
import os
import re
import subprocess
import sys
import time
import logging
from dataclasses import dataclass, field
from typing import Any

# --- OTLP export via HTTP JSON (no protobuf dependency needed) ---

import urllib.request

OTEL_ENDPOINT = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4328")
MFSCLI_HOST = os.environ.get("MOOSEFS_MASTER_HOST", "192.168.0.23")
POLL_INTERVAL = int(os.environ.get("MOOSEFS_POLL_INTERVAL", "60"))
SERVICE_NAME = "moosefs"

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(levelname)s %(message)s",
    datefmt="%Y-%m-%d %H:%M:%S",
)
log = logging.getLogger("moosefs-poller")


def run_mfscli(*flags: str) -> str:
    """Run mfscli and return stdout. Raises on failure."""
    cmd = ["mfscli", "-H", MFSCLI_HOST] + list(flags)
    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    if result.returncode != 0:
        raise RuntimeError(f"mfscli {' '.join(flags)} failed: {result.stderr}")
    return result.stdout


# --- Metric collection ---


@dataclass
class Metric:
    name: str
    description: str
    unit: str
    value: float
    attributes: dict = field(default_factory=dict)
    is_gauge: bool = True  # gauge vs sum


def parse_cluster_summary(output: str) -> list[Metric]:
    """Parse mfscli -SIG output."""
    metrics = []
    kv: dict[str, str] = {}
    for line in output.splitlines():
        if not line.startswith("cluster summary:"):
            continue
        parts = line.split("\t")
        if len(parts) >= 3:
            key = parts[1].strip()
            val = parts[2].strip()
            kv[key] = val

    def g(name: str, desc: str, unit: str, key: str, scale: float = 1.0):
        if key in kv:
            try:
                metrics.append(Metric(name, desc, unit, float(kv[key]) * scale))
            except ValueError:
                pass

    g("moosefs.cluster.total_space", "Total cluster space", "By", "total space")
    g("moosefs.cluster.avail_space", "Available cluster space", "By", "avail space")
    g("moosefs.cluster.free_space", "Free cluster space (incl trash)", "By", "free space")
    g("moosefs.cluster.trash_space", "Space used by trash", "By", "trash space")
    g("moosefs.cluster.trash_files", "Files in trash", "{files}", "trash files")
    g("moosefs.cluster.fs_objects", "Total filesystem objects", "{objects}", "all fs objects")
    g("moosefs.cluster.directories", "Directory count", "{dirs}", "directories")
    g("moosefs.cluster.files", "File count", "{files}", "files")
    g("moosefs.cluster.chunks", "Total chunks", "{chunks}", "chunks")
    g("moosefs.cluster.chunk_copies", "Total chunk copies", "{copies}", "all full chunk copies")
    g("moosefs.master.ram_used", "Master RAM usage", "By", "RAM used")
    g("moosefs.master.cpu_system", "Master CPU system", "1", "CPU used (system)")
    g("moosefs.master.cpu_user", "Master CPU user", "1", "CPU used (user)")
    g("moosefs.master.last_save_duration", "Last metadata save duration", "s", "last save duration")

    # Space utilization as a percentage
    if "total space" in kv and "avail space" in kv:
        try:
            total = float(kv["total space"])
            avail = float(kv["avail space"])
            if total > 0:
                used_pct = ((total - avail) / total) * 100
                metrics.append(Metric(
                    "moosefs.cluster.space_used_percent",
                    "Cluster space utilization percentage",
                    "%", used_pct,
                ))
        except ValueError:
            pass

    return metrics


def parse_chunk_matrix(output: str) -> list[Metric]:
    """Parse chunk health from -SIG output."""
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
                metrics.append(Metric(
                    f"moosefs.chunks.{key}",
                    f"Chunks in {key} state",
                    "{chunks}", float(val),
                    {"state": key},
                ))
            except ValueError:
                pass
    return metrics


def parse_chunkservers(output: str) -> list[Metric]:
    """Parse mfscli -SCS output."""
    metrics = []
    for line in output.splitlines():
        if not line.startswith("chunkservers:"):
            continue
        parts = line.split("\t")
        if len(parts) < 15:
            continue
        # chunkservers: IP port id label version chunks status maint used total ...
        ip = parts[1].strip()
        port = parts[2].strip()
        version = parts[5].strip()
        num_chunks = parts[6].strip()
        status = parts[7].strip()
        maintenance = parts[8].strip()
        used_str = parts[10].strip()
        total_str = parts[11].strip()

        attrs = {"host": ip, "port": port, "version": version, "status": status}

        try:
            metrics.append(Metric(
                "moosefs.chunkserver.chunks", "Chunks on chunkserver",
                "{chunks}", float(num_chunks), attrs,
            ))
        except ValueError:
            pass

        try:
            used = float(used_str)
            total = float(total_str)
            metrics.append(Metric(
                "moosefs.chunkserver.used_space", "Used space on chunkserver",
                "By", used, attrs,
            ))
            metrics.append(Metric(
                "moosefs.chunkserver.total_space", "Total space on chunkserver",
                "By", total, attrs,
            ))
            if total > 0:
                pct = (used / total) * 100
                metrics.append(Metric(
                    "moosefs.chunkserver.used_percent", "Chunkserver space utilization",
                    "%", pct, attrs,
                ))
        except ValueError:
            pass

        # Status as a boolean (1=healthy, 0=unhealthy)
        healthy = 1.0 if status in ("Normal", "Rebalance") else 0.0
        metrics.append(Metric(
            "moosefs.chunkserver.healthy", "Chunkserver health (1=ok, 0=bad)",
            "1", healthy, attrs,
        ))

    return metrics


def parse_disks(output: str) -> list[Metric]:
    """Parse mfscli -SHD output."""
    metrics = []
    for line in output.splitlines():
        if not line.startswith("disks:"):
            continue
        parts = line.split("\t")
        if len(parts) < 15:
            continue
        # disks: host:port:path chunks errors status read_ops ...
        disk_id = parts[1].strip()
        # Parse host:port:path
        id_parts = disk_id.split(":")
        if len(id_parts) >= 3:
            host = id_parts[0]
            port = id_parts[1]
            path = ":".join(id_parts[2:])
        else:
            host = disk_id
            port = ""
            path = ""

        chunks = parts[2].strip()
        error_status = parts[3].strip()
        disk_status = parts[4].strip()

        attrs = {"host": host, "port": port, "path": path}

        try:
            metrics.append(Metric(
                "moosefs.disk.chunks", "Chunks on disk",
                "{chunks}", float(chunks), attrs,
            ))
        except ValueError:
            pass

        # Disk error status (1=ok, 0=has errors)
        has_errors = 0.0 if error_status == "no errors" else 1.0
        metrics.append(Metric(
            "moosefs.disk.has_errors", "Disk error flag (0=ok, 1=errors)",
            "1", has_errors, attrs,
        ))

        # Disk status
        healthy = 1.0 if disk_status == "ok" else 0.0
        metrics.append(Metric(
            "moosefs.disk.healthy", "Disk health (1=ok, 0=bad)",
            "1", healthy, attrs,
        ))

        # Space: used and total are the last two numeric fields
        try:
            used = float(parts[-2].strip())
            total = float(parts[-1].strip())
            metrics.append(Metric(
                "moosefs.disk.used_space", "Used space on disk",
                "By", used, attrs,
            ))
            metrics.append(Metric(
                "moosefs.disk.total_space", "Total space on disk",
                "By", total, attrs,
            ))
            if total > 0:
                pct = (used / total) * 100
                metrics.append(Metric(
                    "moosefs.disk.used_percent", "Disk space utilization",
                    "%", pct, attrs,
                ))
        except (ValueError, IndexError):
            pass

    return metrics


def parse_memory(output: str) -> list[Metric]:
    """Parse mfscli -SMU output."""
    metrics = []
    for line in output.splitlines():
        if "memory usage detailed info:" not in line:
            continue
        parts = line.split("\t")
        if len(parts) < 4:
            continue
        component = parts[1].strip().lower().replace(" ", "_")
        used = parts[2].strip()
        allocated = parts[3].strip()
        try:
            metrics.append(Metric(
                "moosefs.master.memory.used",
                f"Master memory used by {component}",
                "By", float(used),
                {"component": component},
            ))
            metrics.append(Metric(
                "moosefs.master.memory.allocated",
                f"Master memory allocated for {component}",
                "By", float(allocated),
                {"component": component},
            ))
        except ValueError:
            pass
    return metrics


def parse_check_loop(output: str) -> list[Metric]:
    """Parse check loop info from -SIG output."""
    metrics = []
    for line in output.splitlines():
        if not line.startswith("check loop"):
            continue
        parts = line.split("\t")
        if len(parts) < 3:
            continue
        key = parts[1].strip().replace("-", "_")
        val = parts[2].strip()
        if key in ("under_goal_files", "missing_files", "missing_trash_files",
                    "missing_sustained_files", "under_goal_chunks", "missing_chunks"):
            try:
                metrics.append(Metric(
                    f"moosefs.check.{key}",
                    f"Check loop: {key}",
                    "{count}", float(val),
                ))
            except ValueError:
                pass
    return metrics


# --- OTLP JSON export ---


def build_otlp_payload(metrics: list[Metric], timestamp_ns: int) -> dict:
    """Build OTLP metrics JSON payload."""
    gauge_metrics: dict[str, dict] = {}
    for m in metrics:
        key = m.name
        if key not in gauge_metrics:
            gauge_metrics[key] = {
                "name": m.name,
                "description": m.description,
                "unit": m.unit,
                "data_points": [],
            }
        dp: dict[str, Any] = {
            "timeUnixNano": str(timestamp_ns),
            "asDouble": m.value,
        }
        if m.attributes:
            dp["attributes"] = [
                {"key": k, "value": {"stringValue": str(v)}}
                for k, v in m.attributes.items()
            ]
        gauge_metrics[key]["data_points"].append(dp)

    otlp_metrics = []
    for m_data in gauge_metrics.values():
        otlp_metrics.append({
            "name": m_data["name"],
            "description": m_data["description"],
            "unit": m_data["unit"],
            "gauge": {
                "dataPoints": m_data["data_points"],
            },
        })

    return {
        "resourceMetrics": [{
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": SERVICE_NAME}},
                    {"key": "host.name", "value": {"stringValue": MFSCLI_HOST}},
                ],
            },
            "scopeMetrics": [{
                "scope": {"name": "moosefs-poller", "version": "1.0.0"},
                "metrics": otlp_metrics,
            }],
        }],
    }


def push_otlp(payload: dict) -> bool:
    """Push metrics via OTLP/HTTP JSON."""
    url = f"{OTEL_ENDPOINT}/v1/metrics"
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            if resp.status < 300:
                return True
            log.warning("OTLP push got status %d", resp.status)
            return False
    except Exception as e:
        log.error("OTLP push failed: %s", e)
        return False


# --- Main loop ---


def collect_all() -> list[Metric]:
    """Run all mfscli commands and collect metrics."""
    metrics: list[Metric] = []

    # -SIG gives cluster summary + chunk matrix + check loop
    try:
        sig_output = run_mfscli("-SIG")
        metrics.extend(parse_cluster_summary(sig_output))
        metrics.extend(parse_chunk_matrix(sig_output))
        metrics.extend(parse_check_loop(sig_output))
    except Exception as e:
        log.error("Failed to collect cluster summary: %s", e)

    # -SCS gives chunkserver info
    try:
        scs_output = run_mfscli("-SCS")
        metrics.extend(parse_chunkservers(scs_output))
    except Exception as e:
        log.error("Failed to collect chunkserver info: %s", e)

    # -SHD gives disk info
    try:
        shd_output = run_mfscli("-SHD")
        metrics.extend(parse_disks(shd_output))
    except Exception as e:
        log.error("Failed to collect disk info: %s", e)

    # -SMU gives memory usage
    try:
        smu_output = run_mfscli("-SMU")
        metrics.extend(parse_memory(smu_output))
    except Exception as e:
        log.error("Failed to collect memory info: %s", e)

    return metrics


def main():
    log.info(
        "MooseFS poller starting — master=%s endpoint=%s interval=%ds",
        MFSCLI_HOST, OTEL_ENDPOINT, POLL_INTERVAL,
    )

    while True:
        try:
            start = time.monotonic()
            metrics = collect_all()
            timestamp_ns = int(time.time() * 1e9)

            if metrics:
                payload = build_otlp_payload(metrics, timestamp_ns)
                ok = push_otlp(payload)
                elapsed = time.monotonic() - start
                log.info(
                    "Collected %d metrics, push %s (%.1fs)",
                    len(metrics), "OK" if ok else "FAILED", elapsed,
                )
            else:
                log.warning("No metrics collected")

        except Exception as e:
            log.error("Collection cycle failed: %s", e)

        time.sleep(POLL_INTERVAL)


if __name__ == "__main__":
    main()
