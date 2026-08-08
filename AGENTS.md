# AGENTS.md — Home Automation & Fleet

This repository holds Home Assistant configuration, firmware, device sources, and project jobspecs. Fleet Ansible/Nomad control-plane authority has moved to aleksclark/fleet-iac (see fleet/README.md).

## Infrastructure

| Component                | Details                                                              |
|--------------------------|----------------------------------------------------------------------|
| **Home Assistant**       | Docker on `192.168.0.3` (migrating to Nomad fleet — see migration plan) |
| **MQTT Broker**          | Mosquitto (Nomad job on fleet)                                       |
| **ESPHome**              | CLI for compiling/flashing ESP32 devices                             |
| **BLE Proxy Network**    | 6× ESP32-C3 nodes providing Bluetooth coverage to HA                |
| **Compute Fleet**        | Arch Linux nodes managed via `aleksclark/fleet-iac` (Ansible + Nomad) |
| **Distributed Storage**  | MooseFS 4.58.4 across fleet nodes (~26 TB raw)                      |
| **Reverse Proxy**        | Traefik (Nomad system job) with Let's Encrypt wildcard via Cloudflare DNS-01 |
| **Monitoring**           | SigNoz + OpenTelemetry agent (fleet-wide), MooseFS poller           |
| **DNS**                  | CoreDNS (Nomad system job) for `fleet.clark.team`                   |

## Repository Structure

```
├── AGENTS.md                           # this file
├── nomad-migration-plan.md             # NAS → Nomad fleet migration plan
├── .gitignore
├── .tool-versions                      # asdf: python 3.13
│
├── esphome/                            # shared ESPHome resources
│   ├── .gitignore
│   ├── secrets.yaml                    # git-ignored (WiFi, API, OTA creds)
│   └── common/
│       ├── base.yaml                   # logger, api, ota defaults
│       ├── wifi.yaml                   # standard WiFi config
│       └── wifi_stable.yaml            # WiFi with ESP32-C3 stability workaround
│
├── devices/                            # one subdirectory per device type / project
│   ├── ble-scanners/                   # ESP32-C3 BLE proxy nodes
│   │   ├── README.md
│   │   └── ble-scanner-*.yaml          # per-location ESPHome configs (6 nodes)
│   ├── ble-thermometers/               # Xiaomi Mijia BLE temp/humidity sensors
│   │   ├── README.md
│   │   └── devices.txt                 # sensor MACs, BLE keys, tokens
│   ├── della-minisplits/               # Della AC units (ESPHome + TCL protocol)
│   │   ├── README.md
│   │   ├── SETUP.md                    # OpenBeken flashing guide (legacy)
│   │   ├── ESPHOME_SETUP.md            # ESP32-C3 SuperMini replacement guide
│   │   ├── della-ac.yaml               # generic ESPHome config
│   │   ├── amos-minisplit.yaml         # Amos bedroom unit config
│   │   └── components/tcl_climate/     # custom ESPHome TCL protocol component
│   ├── central_air/                    # central HVAC unit docs (PDFs)
│   ├── status-display/                 # ESP32-2432S028 CYD wall-mounted TFT (active)
│   │   ├── README.md
│   │   └── firmware/                   # Rust (esp-idf) firmware
│   │       ├── Cargo.toml              # deps: esp-idf-*, ha-display-kit, mipidsi
│   │       ├── ota_flash.py            # OTA upload script
│   │       └── src/
│   ├── resphome-test/                  # test device for the resphome library
│   │   └── firmware/                   # Rust (esp-idf) example using libs/resphome
│   └── irrigation/                     # moisture sensing & irrigation (planned)
│       ├── README.md
│       └── outdoor/                    # 4-solenoid drip irrigation
│           ├── irrigation-outdoor.yaml # ESPHome config
│           ├── WIRING.md
│           └── board/                  # KiCad PCB design
│
├── libs/                               # shared libraries
│   ├── resphome/                       # Rust ESPHome core (native API, WiFi, BLE, OTA)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── ha-display-kit/                 # embedded-graphics layout/theme toolkit
│   │   ├── Cargo.toml
│   │   └── src/
│   └── CORE-ESP32/                     # KiCad 9 symbol + footprint for CORE-ESP32-C3
│       ├── README.md                   # full pinout, specs, wiring reference
│       ├── CORE-ESP32-C3.kicad_sym
│       └── CORE-ESP32-C3.pretty/
│
├── tools/
│   ├── flasher/
│   │   └── BK7231GUIFlashTool-main/    # BK7231/RTL GUI flash tool
│   ├── firmware/
│   │   ├── OpenBK7231N_1.18.236.rbl
│   │   └── OpenRTL87X0C_1.18.236.bin
│   └── bb-status-reporter/
│       └── bb-status-reporter.sh       # workstation status → HA (away/working/playing)
│
├── home-assistant/                     # HA config, automations, and scripts
│   ├── README.md                       # HVAC zone control architecture & docs
│   ├── mqtt.yaml                       # MQTT climate entities (3 Della units)
│   ├── automations_hvac.yaml           # 5 HVAC automations (schedule + tracking)
│   ├── automations_irrigation.yaml     # irrigation automations
│   ├── input_helpers.yaml              # input_boolean / input_number definitions
│   ├── input_text_bb_status.yaml       # BB status entity
│   ├── zones.yaml                      # house zone topology & thermal coupling
│   ├── pyscript/
│   │   └── hvac_control.py             # pyscript HVAC logic
│   ├── ha-download.sh                  # pull live HA config to snapshot dir
│   ├── ha-upload.sh                    # push snapshot config to HA
│   └── ha-config-snapshot/             # point-in-time HA config backup
│       ├── configuration.yaml
│       ├── automations.yaml
│       ├── mqtt.yaml / mqtt_climate.yaml
│       ├── input_*.yaml / scenes.yaml / scripts.yaml
│       ├── custom_components/
│       └── blueprints/
│
├── fleet/                              # RETIRED authority pointer (see fleet/README.md)
│   ├── README.md                       # MOVED → aleksclark/fleet-iac platform/ansible + jobs/
│   └── MIGRATION_MANIFEST.json         # value-free path classification / provenance
│
└── bt_track/                           # BK7231 flasher (legacy copy)
    └── flasher/
```

## Conventions

### Directory Organization

- **`devices/<project>/`** — Each device type or project gets its own directory. ESPHome YAML configs and Rust firmware live alongside the project they belong to.
- **`esphome/common/`** — Shared YAML packages (`!include` targets) for WiFi, API, OTA to reduce duplication across device configs.
- **`libs/`** — Shared Rust libraries and KiCad assets used by multiple device projects.
- **`fleet/`** — Retired control-plane pointer + migration manifest only. Active fleet IaC: `aleksclark/fleet-iac`.
- **`tools/`** — Flashing utilities, firmware binaries, and helper scripts.
- **`home-assistant/`** — HA-side configuration: automations, MQTT entities, input helpers, pyscript, and config snapshots.

### Adding a New Device Type

1. Create `devices/<device-name>/`
2. Add a `README.md` documenting hardware, purpose, and status
3. Place ESPHome YAML configs or Rust firmware in that directory
4. Reference shared packages from `esphome/common/` via `!include` (ESPHome) or `libs/` via path deps (Rust)
5. Update this file's structure diagram

### Naming

- ESPHome device configs: `<function>-<location>.yaml` (e.g., `ble-scanner-kitchen.yaml`)
- Directories: lowercase, hyphen-separated (e.g., `della-minisplits`, `status-display`)

### Secrets

All ESPHome secrets are managed via `esphome/secrets.yaml` (git-ignored). Configs reference them with `!secret <key>`.

| Key              | Purpose                              |
|------------------|--------------------------------------|
| `wifi_ssid`      | WiFi network name                    |
| `wifi_password`  | WiFi password                        |
| `api_key`        | Home Assistant native API encryption |
| `ota_password`   | Over-the-air update password         |

## ESPHome Commands

```bash
# Compile a device config
esphome compile devices/ble-scanners/ble-scanner-kitchen.yaml

# Upload firmware over WiFi (OTA)
esphome upload devices/ble-scanners/ble-scanner-kitchen.yaml

# Stream device logs
esphome logs devices/ble-scanners/ble-scanner-kitchen.yaml
```

## Fleet Commands

Fleet Ansible/Nomad authority is **not** in this repository.

Use `aleksclark/fleet-iac`:

- Ansible control plane: `platform/ansible` (inventory, roles, playbooks)
- Fleet-owned Nomad jobs: `jobs/`
- Bootstrap ISO: `platform/archiso`
- Classification of what used to live here: `fleet/MIGRATION_MANIFEST.json`

Project-owned jobspecs that remain in **this** repo (example):

```bash
# Minisplit OTEL poller (project-owned under services/)
nomad job run services/minisplit-otel-poller/deploy/nomad/jobs/minisplit-otel-poller.nomad.hcl
```

### Fleet-first merge-order prerequisite (Task 9)

Task 9 retirement merges **fleet-first**: `home_assist` must not merge (PR or master)
until the pinned canonical commit in `fleet/MIGRATION_MANIFEST.json`
(`merge_sequencing.canonical_commit` = `47c45e83…`, fleet-iac master merge of PR #124)
is an ancestor of `aleksclark/fleet-iac` `origin/master`.

- Gate: `tools/fleet_authority_guard/check_fleet_iac_merge_order.py`
- CI workflow: `.github/workflows/ci-fleet-authority-guard.yml` (hard-fail on PR + master;
  no soft/`continue-on-error` path)
- Hosted runners check out private `aleksclark/fleet-iac` into `fleet-iac-canonical`
  with `fetch-depth: 0` using repository secret **`FLEET_IAC_READ_SSH_KEY`**
  (`ssh-key:` on `actions/checkout`; private repo)
- This repo is public; `GITHUB_TOKEN` cannot read private `fleet-iac`. Missing
  `FLEET_IAC_READ_SSH_KEY` fails closed with a clear preflight error (key material is
  never printed)

**Required credential (setup only — do not commit values):** create a **read-only
SSH deploy key** on private `aleksclark/fleet-iac` (GitHub → Settings → Deploy keys →
Allow write access **unchecked**). Add the private key as repository secret
`FLEET_IAC_READ_SSH_KEY` on `aleksclark/home_assist`. No PAT/token and no 1Password
required for this gate. Prefer least privilege; rotate if exposed.

## Current Device Inventory

### Active

| Device                  | Type           | Location(s)                                              | Protocol / Stack         |
|-------------------------|----------------|----------------------------------------------------------|--------------------------|
| BLE Scanners (6×)       | ESP32-C3       | Office, Kitchen, Livingroom, Bedroom, Bathroom, Carport  | ESPHome API              |
| BLE Thermometers (4×)   | Xiaomi MiT2    | Kitchen, A&K BR, Amos, Red Room                          | BLE via proxies          |
| Della Mini Splits (3×)  | ESP32-C3 SuperMini | Kitchen (18k), Amos BR (9k), Livingroom (12k)       | ESPHome + TCL protocol   |
| Hallway Thermostat      | Matter         | Hallway (central HVAC)                                   | Matter                   |
| Status Display          | ESP32-2432S028 | Wall-mounted                                             | Rust (esp-idf) + HA API |

### Planned

| Device             | Type             | Notes                                                    |
|--------------------|------------------|----------------------------------------------------------|
| Indoor Irrigation  | ESP32 + 4 pumps  | Capacitive moisture sensing, peristaltic pumps           |
| Outdoor Irrigation | ESP32 + 4 valves | ESPHome config + KiCad board design in progress          |

## Fleet Inventory

| Node   | Hardware            | IP             | RAM  | Disks                      | Fleet Role                              |
|--------|---------------------|----------------|------|----------------------------|-----------------------------------------|
| node-1 | Dell Inspiron 660   | 192.168.0.23   | 8 GB | 1×3.6 TB + 2×1.8 TB (XFS) | Nomad server, MooseFS master + chunkserver |
| node-2 | Dell OptiPlex 9010  | 192.168.0.24   | 16 GB| 2×3.6 TB + 1×1.8 TB (XFS) | Heavy compute, MooseFS chunkserver + metalogger |
| node-3 | Dell OptiPlex 7020  | 192.168.0.89   | 8 GB | 1×3.6 TB + 2×1.8 TB (XFS) | Compute, MooseFS chunkserver            |

All nodes run Arch Linux, Nomad (client), Consul (agent), Docker, and MooseFS chunkserver.
MooseFS is mounted cluster-wide at `/mnt/moosefs` (~26 TB raw, directories: `/family` 2CP, `/media` 1CP, `/tmp` 1CP).
Network bonding (1G + 2.5G ALB) on all nodes.

A fourth node (the current NAS at `192.168.0.3`, Xeon E5-1620 v4, 32 GB, 4×5.5 TB) is planned — see `nomad-migration-plan.md`.

## Nomad Services

Fleet-owned jobspecs live in `aleksclark/fleet-iac` under `jobs/` (not in this repo).
Project-owned examples that remain here: `services/minisplit-otel-poller/`.

### Infrastructure (fleet-iac `jobs/platform/` + `jobs/home/`)

| Job               | Purpose                                           |
|-------------------|---------------------------------------------------|
| mosquitto         | MQTT broker                                       |
| cloudflared       | Cloudflare tunnel for external SSH                |
| omada             | TP-Link Omada AP management (host network)        |
| traefik           | Reverse proxy, Let's Encrypt wildcard (system job)|
| coredns           | DNS for `fleet.clark.team` (system job)           |
| ddclient          | Dynamic DNS updater                               |
| otel-agent        | OpenTelemetry collector (system job)              |
| signoz            | Monitoring / observability (ClickHouse-backed)    |
| idrive            | Backup agent                                      |
| moosefs-poller    | MooseFS metrics → OTLP                            |

### Home Automation (fleet-iac `jobs/home/`)

| Job               | Purpose                                           |
|-------------------|---------------------------------------------------|
| homeassistant     | Home Assistant (host network, privileged)          |
| matter-server     | Matter protocol server (host network)              |

### Media (fleet-iac `jobs/media/`)

| Job           | Purpose                                               |
|---------------|-------------------------------------------------------|
| jellyfin      | Media server (HW transcode via `/dev/dri` on node-4)  |
| qbittorrent   | Torrent client (WireGuard VPN, NET_ADMIN)              |
| prowlarr      | Indexer manager                                        |
| sonarr        | TV show management                                     |
| radarr        | Movie management                                       |
| lidarr        | Music management                                       |
| bazarr        | Subtitle management                                    |
| readarr       | Ebook management                                       |
| speakarr      | Audiobook management                                   |
| photoprism    | Photo library with AI classification                   |

## Network

| Host / Group       | Address          | Notes                                  |
|--------------------|------------------|----------------------------------------|
| Home Assistant     | 192.168.0.3      | NAS (migrating to fleet)               |
| node-1             | 192.168.0.23     | Nomad server, MooseFS master           |
| node-2             | 192.168.0.24     | Heavy compute                          |
| node-3             | 192.168.0.89     | Compute                                |
| ESPHome devices    | DHCP             | Static leases recommended              |
| Fleet services     | `*.fleet.clark.team` | Traefik-routed via Cloudflare DNS  |
| MQTT broker        | Nomad-scheduled  | Port 1883                              |
