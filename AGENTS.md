# AGENTS.md — Home Automation & Fleet

This repository holds all configuration, firmware, fleet management, and documentation for a Home Assistant-based home automation system backed by an Ansible-managed compute fleet.

## Infrastructure

| Component                | Details                                                              |
|--------------------------|----------------------------------------------------------------------|
| **Home Assistant**       | Docker on `192.168.0.3` (migrating to Nomad fleet — see migration plan) |
| **MQTT Broker**          | Mosquitto (Nomad job on fleet)                                       |
| **ESPHome**              | CLI for compiling/flashing ESP32 devices                             |
| **BLE Proxy Network**    | 6× ESP32-C3 nodes providing Bluetooth coverage to HA                |
| **Compute Fleet**        | 3× Arch Linux nodes managed by Ansible, running Nomad + Consul      |
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
├── fleet/                              # Ansible-managed compute fleet
│   ├── README.md                       # architecture, stack, quick start
│   ├── ansible.cfg
│   ├── inventory/
│   │   └── hosts.yml
│   ├── group_vars/
│   │   ├── all.yml                     # users, SSH, base packages, mirrors
│   │   ├── storage.yml                 # blockyard / MooseFS disk config
│   │   └── compute.yml                 # Docker workload nodes
│   ├── host_vars/
│   │   ├── node-1.yml                  # Dell Inspiron 660 — Nomad server + MooseFS master
│   │   ├── node-2.yml                  # Dell OptiPlex 9010 — heavy compute + metalogger
│   │   └── node-3.yml                  # Dell OptiPlex 7020 — compute + chunkserver
│   ├── roles/
│   │   ├── base/                       # Arch baseline: pacman, snapper, users, SSH
│   │   ├── nomad/
│   │   ├── consul/
│   │   ├── docker/
│   │   └── blockyard/                  # distributed block storage daemon
│   ├── playbooks/
│   │   ├── site.yml                    # full converge
│   │   ├── upgrade.yml                 # rolling upgrade with snapshot/rollback
│   │   ├── deploy-blockyard.yml
│   │   ├── blockyard-restart.yml
│   │   └── blockyard-wipe-raft.yml
│   ├── nomad/                          # Nomad job definitions (HCL)
│   │   ├── infrastructure/             # mosquitto, cloudflared, omada, traefik,
│   │   │                               # coredns, ddclient, otel-agent, signoz, idrive
│   │   ├── home-automation/            # homeassistant, matter-server
│   │   └── media/                      # jellyfin, qbittorrent, *arr suite,
│   │                                   # photoprism, readarr, speakarr
│   ├── monitoring/
│   │   └── moosefs-poller.py           # MooseFS → OTLP metrics exporter
│   └── archiso/                        # custom Arch ISO for USB bootstrap
│       ├── build.sh                    # --inject-key to bake in SSH pubkey
│       ├── profiledef.sh
│       ├── packages.x86_64
│       └── airootfs/
│
└── bt_track/                           # BK7231 flasher (legacy copy)
    └── flasher/
```

## Conventions

### Directory Organization

- **`devices/<project>/`** — Each device type or project gets its own directory. ESPHome YAML configs and Rust firmware live alongside the project they belong to.
- **`esphome/common/`** — Shared YAML packages (`!include` targets) for WiFi, API, OTA to reduce duplication across device configs.
- **`libs/`** — Shared Rust libraries and KiCad assets used by multiple device projects.
- **`fleet/`** — Ansible inventory, roles, playbooks, Nomad job definitions, and fleet tooling.
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

```bash
# Full converge (all nodes, all roles)
cd fleet && ansible-playbook -i inventory/hosts.yml playbooks/site.yml

# Rolling upgrade with btrfs snapshot + rollback
ansible-playbook -i inventory/hosts.yml playbooks/upgrade.yml

# Deploy blockyard to storage nodes
ansible-playbook -i inventory/hosts.yml playbooks/deploy-blockyard.yml --limit storage

# Build custom Arch ISO for bootstrapping new nodes
cd fleet/archiso && ./build.sh --inject-key ~/.ssh/id_ed25519.pub

# Submit a Nomad job
nomad job run fleet/nomad/media/jellyfin.nomad.hcl
```

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

### Infrastructure (`fleet/nomad/infrastructure/`)

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

### Home Automation (`fleet/nomad/home-automation/`)

| Job               | Purpose                                           |
|-------------------|---------------------------------------------------|
| homeassistant     | Home Assistant (host network, privileged)          |
| matter-server     | Matter protocol server (host network)              |

### Media (`fleet/nomad/media/`)

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
