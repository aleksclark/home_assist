#!/usr/bin/env python3
"""
Fleet Service Health Monitor
Performs authenticated API checks against fleet services and reports results
as OpenTelemetry metrics to SigNoz.
"""

import json
import time
import urllib.request
import urllib.error
import ssl
import sys
import os
import socket

# SigNoz OTLP endpoint
OTEL_ENDPOINT = os.environ.get("OTEL_ENDPOINT", "http://otel-collector.fleet.clark.team:4318")

# Service checks configuration
SERVICES = [
    {
        "name": "nomad",
        "description": "Nomad Cluster Leader",
        "url": "http://node-1.fleet.clark.team:4646/v1/status/leader",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "4647",
    },
    {
        "name": "nomad_jobs",
        "description": "Nomad Job List API",
        "url": "http://node-1.fleet.clark.team:4646/v1/jobs",
        "method": "GET",
        "expect_status": 200,
    },
    {
        "name": "moosefs_master",
        "description": "MooseFS Master Client Port",
        "check_type": "tcp",
        "host": "moosefs-master.fleet.clark.team",
        "port": 9421,
    },
    {
        "name": "moosefs_chunkserver_node1",
        "description": "MooseFS Chunkserver Node-1",
        "check_type": "tcp",
        "host": "node-1.fleet.clark.team",
        "port": 9422,
    },
    {
        "name": "moosefs_chunkserver_node2",
        "description": "MooseFS Chunkserver Node-2",
        "check_type": "tcp",
        "host": "node-2.fleet.clark.team",
        "port": 9422,
    },
    {
        "name": "moosefs_chunkserver_node3",
        "description": "MooseFS Chunkserver Node-3",
        "check_type": "tcp",
        "host": "node-3.fleet.clark.team",
        "port": 9422,
    },
    {
        "name": "nomad_node1_health",
        "description": "Nomad Node-1 Agent",
        "url": "http://node-1.fleet.clark.team:4646/v1/agent/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "ok",
    },
    {
        "name": "nomad_node2_health",
        "description": "Nomad Node-2 Agent",
        "url": "http://node-2.fleet.clark.team:4646/v1/agent/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "ok",
    },
    {
        "name": "nomad_node3_health",
        "description": "Nomad Node-3 Agent",
        "url": "http://node-3.fleet.clark.team:4646/v1/agent/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "ok",
    },
    {
        "name": "nomad_node4_health",
        "description": "Nomad Node-4 Agent",
        "url": "http://node-4.fleet.clark.team:4646/v1/agent/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "ok",
    },
    {
        "name": "signoz",
        "description": "SigNoz Health",
        "url": "http://signoz.fleet.clark.team:8080/api/v1/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "ok",
    },
    {
        "name": "clickhouse",
        "description": "ClickHouse Ping",
        "url": "http://clickhouse.fleet.clark.team:8123/ping",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "Ok",
    },
    {
        "name": "omada",
        "description": "Omada Controller API Info",
        "url": "https://omada.fleet.clark.team:8043/api/info",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "omadacId",
        "skip_tls_verify": True,
    },
    {
        "name": "omada_openapi",
        "description": "Omada OpenAPI Auth",
        "check_type": "omada_openapi",
        "base_url": "https://omada.fleet.clark.team:8043",
        "omadac_id": "e6d76fd594d5127815bc23a2c1adb0d0",
        "client_id": "67863a1fa043421284537d4e9e0f972f",
        "client_secret": "63ebbede0e714a80844fde80d4c3c2a5",
    },
    {
        "name": "omada_network",
        "description": "Omada Network Metrics (devices, WAN, APs, clients)",
        "check_type": "omada_network",
        "base_url": "https://omada.fleet.clark.team:8043",
        "omadac_id": "e6d76fd594d5127815bc23a2c1adb0d0",
        "client_id": "67863a1fa043421284537d4e9e0f972f",
        "client_secret": "63ebbede0e714a80844fde80d4c3c2a5",
    },
    {
        "name": "homeassistant",
        "description": "Home Assistant API",
        "url": "http://homeassistant.fleet.clark.team:8123/api/",
        "method": "GET",
        "expect_status": 401,  # Returns 401 without token = API is alive
    },
    {
        "name": "traefik_node1",
        "description": "Traefik Node-1 Metrics",
        "url": "http://node-1.fleet.clark.team:8082/metrics",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "traefik_",
    },
    {
        "name": "traefik_node3",
        "description": "Traefik Node-3 Metrics",
        "url": "http://node-3.fleet.clark.team:8082/metrics",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "traefik_",
    },
    {
        "name": "traefik_node4",
        "description": "Traefik Node-4 Metrics",
        "url": "http://node-4.fleet.clark.team:8082/metrics",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "traefik_",
    },
    {
        "name": "coredns",
        "description": "CoreDNS TCP Port",
        "check_type": "tcp",
        "host": "node-1.fleet.clark.team",
        "port": 53,
    },
    {
        "name": "emqx",
        "description": "EMQX MQTT VIP TCP",
        "check_type": "tcp",
        "host": "mqtt.fleet.clark.team",
        "port": 1883,
    },
    # --- Application Services (deep health checks) ---
    {
        "name": "jellyfin",
        "description": "Jellyfin Media Server",
        "url": "http://jellyfin.fleet.clark.team:8096/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "Healthy",
    },
    {
        "name": "radarr",
        "description": "Radarr Movie Manager",
        "url": "http://radarr.fleet.clark.team:7878/ping",
        "method": "GET",
        "expect_status": 500,
        "expect_body_contains": "status",
    },
    {
        "name": "prowlarr",
        "description": "Prowlarr Indexer Manager",
        "url": "http://prowlarr.fleet.clark.team:9696/ping",
        "method": "GET",
        "expect_status": 500,
        "expect_body_contains": "status",
    },
    {
        "name": "readarr",
        "description": "Readarr Book Manager",
        "url": "http://readarr.fleet.clark.team:8788/ping",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "OK",
    },
    {
        "name": "photoprism",
        "description": "PhotoPrism Photo Library",
        "url": "http://photoprism.fleet.clark.team:2342/api/v1/status",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "operational",
    },
    {
        "name": "syncthing",
        "description": "Syncthing File Sync",
        "url": "http://syncthing.fleet.clark.team:8384/rest/noauth/health",
        "method": "GET",
        "expect_status": 200,
        "expect_body_contains": "OK",
    },
    # --- VPN-tunneled services (check Nomad allocation is running) ---
    {
        "name": "sonarr",
        "description": "Sonarr TV Manager (VPN)",
        "check_type": "nomad_job",
        "job_name": "sonarr",
        "nomad_url": "http://node-1.fleet.clark.team:4646",
    },
    {
        "name": "lidarr",
        "description": "Lidarr Music Manager (VPN)",
        "check_type": "nomad_job",
        "job_name": "lidarr",
        "nomad_url": "http://node-1.fleet.clark.team:4646",
    },
    {
        "name": "bazarr",
        "description": "Bazarr Subtitle Manager (VPN)",
        "check_type": "nomad_job",
        "job_name": "bazarr",
        "nomad_url": "http://node-1.fleet.clark.team:4646",
    },
    {
        "name": "qbittorrent",
        "description": "qBittorrent Download Client (VPN)",
        "check_type": "nomad_job",
        "job_name": "qbittorrent",
        "nomad_url": "http://node-1.fleet.clark.team:4646",
    },
    {
        "name": "speakarr",
        "description": "Speakarr Audiobook Manager (VPN)",
        "check_type": "nomad_job",
        "job_name": "speakarr",
        "nomad_url": "http://node-1.fleet.clark.team:4646",
    },
    {
        "name": "cloudflared",
        "description": "Cloudflare Tunnel",
        "check_type": "nomad_job",
        "job_name": "cloudflared",
        "nomad_url": "http://node-1.fleet.clark.team:4646",
    },
]

# Create SSL context that skips verification
ssl_ctx_noverify = ssl.create_default_context()
ssl_ctx_noverify.check_hostname = False
ssl_ctx_noverify.verify_mode = ssl.CERT_NONE


def check_http(service):
    """Perform HTTP health check. Returns (healthy: bool, latency_ms: float, detail: str)"""
    url = service["url"]
    method = service.get("method", "GET")
    headers = service.get("headers", {})
    body = service.get("body")
    timeout = service.get("timeout", 5)
    expect_status = service.get("expect_status", 200)
    expect_body = service.get("expect_body_contains")
    skip_tls = service.get("skip_tls_verify", False)

    start = time.time()
    try:
        data = body.encode() if body else None
        req = urllib.request.Request(url, data=data, method=method)
        for k, v in headers.items():
            req.add_header(k, v)

        ctx = ssl_ctx_noverify if skip_tls else None
        resp = urllib.request.urlopen(req, timeout=timeout, context=ctx)
        latency = (time.time() - start) * 1000
        status = resp.status
        resp_body = resp.read().decode("utf-8", errors="replace")

        if status != expect_status:
            return False, latency, f"status {status} != expected {expect_status}"

        if expect_body and expect_body not in resp_body:
            return False, latency, f"body missing '{expect_body}'"

        return True, latency, "ok"

    except urllib.error.HTTPError as e:
        latency = (time.time() - start) * 1000
        if e.code == expect_status:
            return True, latency, "ok (expected error code)"
        return False, latency, f"HTTP {e.code}: {e.reason}"
    except Exception as e:
        latency = (time.time() - start) * 1000
        return False, latency, str(e)[:100]


def check_tcp(service):
    """TCP port check. Returns (healthy, latency_ms, detail)"""
    host = service["host"]
    port = service["port"]
    timeout = service.get("timeout", 5)

    start = time.time()
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.settimeout(timeout)
        s.connect((host, port))
        s.close()
        latency = (time.time() - start) * 1000
        return True, latency, "ok"
    except Exception as e:
        latency = (time.time() - start) * 1000
        return False, latency, str(e)[:100]


def check_omada_openapi(service):
    """Authenticate to Omada OpenAPI and verify token. Returns (healthy, latency_ms, detail)"""
    base_url = service["base_url"]
    omadac_id = service["omadac_id"]
    client_id = service["client_id"]
    client_secret = service["client_secret"]

    url = f"{base_url}/openapi/authorize/token?grant_type=client_credentials"
    payload = json.dumps({
        "omadacId": omadac_id,
        "client_id": client_id,
        "client_secret": client_secret,
    }).encode()

    start = time.time()
    try:
        req = urllib.request.Request(url, data=payload, method="POST")
        req.add_header("Content-Type", "application/json")
        resp = urllib.request.urlopen(req, timeout=10, context=ssl_ctx_noverify)
        latency = (time.time() - start) * 1000
        body = json.loads(resp.read().decode())

        if body.get("errorCode") == 0 and body.get("result", {}).get("accessToken"):
            return True, latency, "token acquired"
        return False, latency, f"errorCode={body.get('errorCode')}: {body.get('msg', 'unknown')}"
    except Exception as e:
        latency = (time.time() - start) * 1000
        return False, latency, str(e)[:100]


def get_omada_token(base_url, omadac_id, client_id, client_secret):
    """Get Omada OpenAPI access token. Returns token string or None."""
    url = f"{base_url}/openapi/authorize/token?grant_type=client_credentials"
    payload = json.dumps({
        "omadacId": omadac_id,
        "client_id": client_id,
        "client_secret": client_secret,
    }).encode()
    try:
        req = urllib.request.Request(url, data=payload, method="POST")
        req.add_header("Content-Type", "application/json")
        resp = urllib.request.urlopen(req, timeout=10, context=ssl_ctx_noverify)
        body = json.loads(resp.read().decode())
        if body.get("errorCode") == 0:
            return body["result"]["accessToken"]
    except Exception:
        pass
    return None


def omada_api_get(base_url, omada_id, path, token, params=None):
    """Make authenticated GET request to Omada OpenAPI. Returns parsed result or None."""
    url = f"{base_url}/openapi/v1/{omada_id}{path}"
    if params:
        url += "?" + "&".join(f"{k}={v}" for k, v in params.items())
    try:
        req = urllib.request.Request(url)
        req.add_header("Authorization", f"AccessToken={token}")
        resp = urllib.request.urlopen(req, timeout=10, context=ssl_ctx_noverify)
        body = json.loads(resp.read().decode())
        if body.get("errorCode") == 0:
            return body.get("result")
    except Exception:
        pass
    return None


def omada_api_post(base_url, omada_id, path, token, data, params=None, api_version="v1"):
    """Make authenticated POST request to Omada OpenAPI. Returns parsed result or None."""
    url = f"{base_url}/openapi/{api_version}/{omada_id}{path}"
    if params:
        url += "?" + "&".join(f"{k}={v}" for k, v in params.items())
    try:
        req = urllib.request.Request(url, data=json.dumps(data).encode(), method="POST")
        req.add_header("Authorization", f"AccessToken={token}")
        req.add_header("Content-Type", "application/json")
        resp = urllib.request.urlopen(req, timeout=10, context=ssl_ctx_noverify)
        body = json.loads(resp.read().decode())
        if body.get("errorCode") == 0:
            return body.get("result")
    except Exception:
        pass
    return None


def parse_omada_uptime(uptime_str):
    """Parse Omada uptime string like '14day(s) 7h 12m 38s' to seconds."""
    if not uptime_str:
        return None
    import re
    total = 0
    # Handle "14day(s) 7h 12m 38s" or "14 days 07:12:38"
    days = re.search(r'(\d+)\s*day', uptime_str)
    if days:
        total += int(days.group(1)) * 86400

    # Try HH:MM:SS format
    hms = re.search(r'(\d+):(\d+):(\d+)', uptime_str)
    if hms:
        total += int(hms.group(1)) * 3600 + int(hms.group(2)) * 60 + int(hms.group(3))
    else:
        # Try "7h 12m 38s" format
        hours = re.search(r'(\d+)h', uptime_str)
        if hours:
            total += int(hours.group(1)) * 3600
        mins = re.search(r'(\d+)m', uptime_str)
        if mins:
            total += int(mins.group(1)) * 60
        secs = re.search(r'(\d+)s', uptime_str)
        if secs:
            total += int(secs.group(1))

    return total if total > 0 else None


def collect_omada_metrics(service):
    """
    Collect comprehensive Omada network metrics via OpenAPI.
    Returns list of (metric_name, value, attributes_dict) tuples.
    """
    base_url = service["base_url"]
    omadac_id = service["omadac_id"]
    client_id = service["client_id"]
    client_secret = service["client_secret"]

    metrics = []

    # Get token
    token = get_omada_token(base_url, omadac_id, client_id, client_secret)
    if not token:
        return metrics

    # Get site ID
    sites_result = omada_api_get(base_url, omadac_id, "/sites", token, {"page": "1", "pageSize": "10"})
    if not sites_result or not sites_result.get("data"):
        return metrics
    site_id = sites_result["data"][0]["siteId"]

    # --- Device Status (APs + Gateway) ---
    devices_result = omada_api_get(base_url, omadac_id, f"/sites/{site_id}/devices", token, {"page": "1", "pageSize": "50"})
    if devices_result and devices_result.get("data"):
        for dev in devices_result["data"]:
            name = dev.get("name", dev.get("mac", "unknown"))
            dev_type = dev.get("type", "unknown")
            mac = dev.get("mac", "")
            # status: 0=disconnected, 1=connected
            status = 1 if dev.get("status") == 1 else 0
            attrs = {"device.name": name, "device.type": dev_type, "device.mac": mac}

            metrics.append(("omada.device.status", status, attrs))
            metrics.append(("omada.device.cpu_util", dev.get("cpuUtil", 0), attrs))
            metrics.append(("omada.device.mem_util", dev.get("memUtil", 0), attrs))

            # Parse uptime to seconds
            uptime_str = dev.get("uptime", "")
            uptime_secs = parse_omada_uptime(uptime_str)
            if uptime_secs is not None:
                metrics.append(("omada.device.uptime_seconds", uptime_secs, attrs))

    # --- Gateway WAN Status (latency, packet loss, traffic) ---
    gateway_mac = None
    if devices_result and devices_result.get("data"):
        for dev in devices_result["data"]:
            if dev.get("type") == "gateway":
                gateway_mac = dev["mac"]
                break

    if gateway_mac:
        wan_result = omada_api_get(base_url, omadac_id, f"/sites/{site_id}/gateways/{gateway_mac}/wan-status", token)
        if wan_result:
            for port in wan_result:
                if port.get("mode") != 0:  # Only WAN-mode ports
                    continue
                port_name = port.get("portDesc") or port.get("name", f"WAN{port.get('port', '?')}")
                wan_attrs = {"wan.port": port_name, "wan.ip": port.get("ip", "")}

                # Internet state: 0=disconnected, 1=connected
                inet_state = 1 if port.get("internetState") == 1 else 0
                metrics.append(("omada.wan.connected", inet_state, wan_attrs))
                metrics.append(("omada.wan.latency_ms", port.get("latency", 0), wan_attrs))
                metrics.append(("omada.wan.packet_loss_pct", port.get("loss", 0.0), wan_attrs))
                metrics.append(("omada.wan.rx_rate_kbps", port.get("rxRate", 0), wan_attrs))
                metrics.append(("omada.wan.tx_rate_kbps", port.get("txRate", 0), wan_attrs))
                metrics.append(("omada.wan.rx_bytes_total", port.get("rx", 0), wan_attrs))
                metrics.append(("omada.wan.tx_bytes_total", port.get("tx", 0), wan_attrs))
                metrics.append(("omada.wan.rx_error_pkts", port.get("rxErrorPkts", 0), wan_attrs))
                metrics.append(("omada.wan.tx_error_pkts", port.get("txErrorPkts", 0), wan_attrs))

    # --- AP Radio Channel Utilization ---
    if devices_result and devices_result.get("data"):
        for dev in devices_result["data"]:
            if dev.get("type") != "ap":
                continue
            ap_mac = dev["mac"]
            ap_name = dev.get("name", ap_mac)
            radios = omada_api_get(base_url, omadac_id, f"/sites/{site_id}/aps/{ap_mac}/radios", token)
            if radios:
                # 2.4GHz radio
                wp2g = radios.get("wp2g", {})
                if wp2g:
                    radio_attrs = {"device.name": ap_name, "radio.band": "2.4GHz", "radio.channel": wp2g.get("actualChannel", "")}
                    metrics.append(("omada.ap.channel_util_pct", wp2g.get("busyUtil", 0), radio_attrs))
                    rt2g = radios.get("radioTraffic2g", {})
                    if rt2g:
                        metrics.append(("omada.ap.rx_error_pkts", rt2g.get("rxErrPkts", 0), radio_attrs))
                        metrics.append(("omada.ap.tx_error_pkts", rt2g.get("txErrPkts", 0), radio_attrs))
                        metrics.append(("omada.ap.rx_retry_pkts", rt2g.get("rxRetryPkts", 0), radio_attrs))

                # 5GHz radio
                wp5g = radios.get("wp5g", {})
                if wp5g:
                    radio_attrs = {"device.name": ap_name, "radio.band": "5GHz", "radio.channel": wp5g.get("actualChannel", "")}
                    metrics.append(("omada.ap.channel_util_pct", wp5g.get("busyUtil", 0), radio_attrs))
                    rt5g = radios.get("radioTraffic5g", {})
                    if rt5g:
                        metrics.append(("omada.ap.rx_error_pkts", rt5g.get("rxErrPkts", 0), radio_attrs))
                        metrics.append(("omada.ap.tx_error_pkts", rt5g.get("txErrPkts", 0), radio_attrs))
                        metrics.append(("omada.ap.rx_retry_pkts", rt5g.get("rxRetryPkts", 0), radio_attrs))

    # --- Client Statistics ---
    clients_result = omada_api_post(base_url, omadac_id, f"/sites/{site_id}/clients", token,
                                     {"page": 1, "pageSize": 300}, api_version="v2")
    if clients_result and clients_result.get("data"):
        clients = clients_result["data"]
        total_clients = len(clients)
        wireless_clients = sum(1 for c in clients if c.get("wireless"))
        wired_clients = total_clients - wireless_clients

        metrics.append(("omada.clients.total", total_clients, {}))
        metrics.append(("omada.clients.wireless", wireless_clients, {}))
        metrics.append(("omada.clients.wired", wired_clients, {}))

        # Clients per AP
        ap_client_count = {}
        for c in clients:
            if c.get("wireless") and c.get("apName"):
                ap_name = c["apName"]
                ap_client_count[ap_name] = ap_client_count.get(ap_name, 0) + 1

        for ap_name, count in ap_client_count.items():
            metrics.append(("omada.ap.client_count", count, {"device.name": ap_name}))

        # Per-client signal quality (only wireless)
        for c in clients:
            if c.get("wireless") and c.get("rssi"):
                client_name = c.get("name") or c.get("hostName") or c.get("mac", "unknown")
                client_attrs = {
                    "client.name": client_name,
                    "client.mac": c.get("mac", ""),
                    "client.ap": c.get("apName", ""),
                    "client.ssid": c.get("ssid", ""),
                }
                metrics.append(("omada.client.rssi", c.get("rssi", 0), client_attrs))
                metrics.append(("omada.client.signal_level", c.get("signalLevel", 0), client_attrs))
                if c.get("activity") is not None:
                    metrics.append(("omada.client.activity_kbps", c.get("activity", 0), client_attrs))

    return metrics


def check_nomad_job(service):
    """Check a Nomad job has a healthy running allocation. Returns (healthy, latency_ms, detail)"""
    job_name = service["job_name"]
    nomad_url = service["nomad_url"]
    url = f"{nomad_url}/v1/job/{job_name}/allocations"

    start = time.time()
    try:
        req = urllib.request.Request(url)
        resp = urllib.request.urlopen(req, timeout=5)
        latency = (time.time() - start) * 1000
        allocs = json.loads(resp.read().decode())

        running = [a for a in allocs if a.get("ClientStatus") == "running"]
        if running:
            return True, latency, f"{len(running)} alloc(s) running"
        pending = [a for a in allocs if a.get("ClientStatus") == "pending"]
        if pending:
            return False, latency, f"pending ({len(pending)} alloc(s))"
        return False, latency, f"no running allocs (total={len(allocs)})"
    except Exception as e:
        latency = (time.time() - start) * 1000
        return False, latency, str(e)[:100]


def check_dns(service):
    """DNS resolution check. Returns (healthy, latency_ms, detail)"""
    import subprocess
    query = service["query"]
    server = service["server"]

    start = time.time()
    try:
        result = subprocess.run(
            ["dig", f"@{server}", query, "+short", "+time=3", "+tries=1"],
            capture_output=True, text=True, timeout=5
        )
        latency = (time.time() - start) * 1000
        output = result.stdout.strip()
        if result.returncode == 0 and output:
            return True, latency, f"resolved: {output}"
        return False, latency, f"no answer (rc={result.returncode})"
    except Exception as e:
        latency = (time.time() - start) * 1000
        return False, latency, str(e)[:100]


def build_otlp_metrics(results):
    """Build OTLP metrics payload for all check results."""
    now_ns = int(time.time() * 1e9)
    
    gauge_points_health = []
    gauge_points_latency = []
    
    for name, healthy, latency_ms, detail in results:
        attrs = [
            {"key": "service.name", "value": {"stringValue": name}},
            {"key": "check.detail", "value": {"stringValue": detail}},
        ]
        
        gauge_points_health.append({
            "attributes": attrs,
            "timeUnixNano": str(now_ns),
            "asInt": "1" if healthy else "0",
        })
        
        gauge_points_latency.append({
            "attributes": [{"key": "service.name", "value": {"stringValue": name}}],
            "timeUnixNano": str(now_ns),
            "asDouble": latency_ms,
        })
    
    payload = {
        "resourceMetrics": [{
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": "fleet-health-monitor"}},
                    {"key": "service.namespace", "value": {"stringValue": "infrastructure"}},
                ]
            },
            "scopeMetrics": [{
                "scope": {"name": "fleet.health.monitor", "version": "1.0.0"},
                "metrics": [
                    {
                        "name": "fleet.service.health",
                        "description": "Service health status (1=healthy, 0=unhealthy)",
                        "unit": "1",
                        "gauge": {"dataPoints": gauge_points_health}
                    },
                    {
                        "name": "fleet.service.latency",
                        "description": "Service check response latency",
                        "unit": "ms",
                        "gauge": {"dataPoints": gauge_points_latency}
                    }
                ]
            }]
        }]
    }
    return payload


def send_to_otel(payload):
    """Send metrics to SigNoz OTLP HTTP endpoint."""
    url = f"{OTEL_ENDPOINT}/v1/metrics"
    data = json.dumps(payload).encode()
    req = urllib.request.Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")
    
    try:
        resp = urllib.request.urlopen(req, timeout=10)
        return resp.status == 200
    except Exception as e:
        print(f"Failed to send metrics to OTLP: {e}", file=sys.stderr)
        return False


def run_checks():
    """Run all service checks and return results plus omada metrics."""
    results = []
    omada_metrics = []
    
    for svc in SERVICES:
        name = svc["name"]
        check_type = svc.get("check_type", "http")
        
        if check_type == "omada_network":
            # Collect detailed network metrics (separate from health checks)
            try:
                omada_metrics = collect_omada_metrics(svc)
                print(f"  ◆ omada_network: collected {len(omada_metrics)} metrics")
            except Exception as e:
                print(f"  ✗ omada_network: error collecting metrics: {e}")
            continue
        elif check_type == "http":
            healthy, latency, detail = check_http(svc)
        elif check_type == "tcp":
            healthy, latency, detail = check_tcp(svc)
        elif check_type == "dns":
            healthy, latency, detail = check_dns(svc)
        elif check_type == "omada_openapi":
            healthy, latency, detail = check_omada_openapi(svc)
        elif check_type == "nomad_job":
            healthy, latency, detail = check_nomad_job(svc)
        else:
            healthy, latency, detail = False, 0, f"unknown check type: {check_type}"
        
        results.append((name, healthy, latency, detail))
        status = "✓" if healthy else "✗"
        print(f"  {status} {name}: {detail} ({latency:.0f}ms)")
    
    return results, omada_metrics


def build_omada_otlp_metrics(omada_metrics):
    """Build OTLP metrics payload for Omada network metrics."""
    if not omada_metrics:
        return None

    now_ns = str(int(time.time() * 1e9))

    # Group metrics by name to create one metric definition per unique name
    metrics_by_name = {}
    for metric_name, value, attrs in omada_metrics:
        if metric_name not in metrics_by_name:
            metrics_by_name[metric_name] = []
        
        otel_attrs = [{"key": k, "value": {"stringValue": str(v)}} for k, v in attrs.items()]
        
        # Determine value type
        if isinstance(value, float):
            point = {"attributes": otel_attrs, "timeUnixNano": now_ns, "asDouble": value}
        else:
            point = {"attributes": otel_attrs, "timeUnixNano": now_ns, "asInt": str(int(value))}
        
        metrics_by_name[metric_name].append(point)

    # Build OTLP structure
    metric_defs = []
    for name, points in metrics_by_name.items():
        metric_defs.append({
            "name": name,
            "description": name.replace(".", " ").replace("_", " ").title(),
            "unit": "1",
            "gauge": {"dataPoints": points}
        })

    payload = {
        "resourceMetrics": [{
            "resource": {
                "attributes": [
                    {"key": "service.name", "value": {"stringValue": "omada-network-monitor"}},
                    {"key": "service.namespace", "value": {"stringValue": "infrastructure"}},
                ]
            },
            "scopeMetrics": [{
                "scope": {"name": "omada.network.monitor", "version": "1.0.0"},
                "metrics": metric_defs
            }]
        }]
    }
    return payload


def main():
    interval = int(os.environ.get("CHECK_INTERVAL", "60"))
    
    print(f"Fleet Health Monitor starting (interval={interval}s)")
    print(f"OTLP endpoint: {OTEL_ENDPOINT}")
    print(f"Monitoring {len(SERVICES)} services")
    print()
    
    while True:
        print(f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] Running checks...")
        results, omada_metrics = run_checks()
        
        healthy_count = sum(1 for _, h, _, _ in results if h)
        total = len(results)
        print(f"  → {healthy_count}/{total} services healthy")
        
        # Send health check metrics to OTLP
        payload = build_otlp_metrics(results)
        if send_to_otel(payload):
            print("  → Health metrics sent to SigNoz")
        else:
            print("  → WARNING: Failed to send health metrics")
        
        # Send Omada network metrics to OTLP
        if omada_metrics:
            omada_payload = build_omada_otlp_metrics(omada_metrics)
            if omada_payload and send_to_otel(omada_payload):
                print(f"  → Omada network metrics sent ({len(omada_metrics)} data points)")
            else:
                print("  → WARNING: Failed to send Omada metrics")
        
        print()
        time.sleep(interval)


if __name__ == "__main__":
    main()
