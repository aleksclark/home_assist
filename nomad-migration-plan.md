# NAS → Nomad Fleet Migration Plan

## Current State

### NAS Hardware (192.168.0.3 — "stash")
- Xeon E5-1620 v4 (4C/8T @ 3.5GHz), 32GB RAM
- ZFS pool (`pool0`) → `/storage` with datasets: `/storage/media`, `/storage/family`, `/storage/tmp`
- OS on 405GB `/dev/sda3`
- Running: Docker (docker-compose), NFS server, ddclient, iDrive backup

### Fleet Nodes (Nomad targets)
| Node | IP | CPU | RAM | Disks | MooseFS Role |
|------|-----|-----|-----|-------|-------------|
| node-1 | 192.168.0.23 | Ivy Bridge | 8GB | 1x 3.6TB | master + chunkserver |
| node-2 | 192.168.0.24 | Ivy Bridge | 16GB | 2x 3.6TB | chunkserver + metalogger |
| node-3 | 192.168.0.89 | Ivy Bridge | 8GB | 1x 3.6TB + 2x 1.8TB | chunkserver |

### Distributed Storage
- MooseFS 4.58.4 mounted at `/mnt/moosefs` via FUSE
- Directories: `/family` (2CP), `/media` (1CP), `/tmp` (1CP)
- Data migration from NAS ZFS → MooseFS in progress

---

## Service Inventory & Dependencies

### Tier 1: Infrastructure (migrate first)

| Service | Image | Network Mode | Storage | Dependencies | Notes |
|---------|-------|-------------|---------|-------------|-------|
| **mosquitto** | eclipse-mosquitto:2 | bridge (1883, 9001) | config+data+log dirs | none | MQTT broker, depended on by HA |
| **cloudflared** | cloudflare/cloudflared | bridge | config.yml | external (Cloudflare) | Tunnel for aero.clark.team → SSH |
| **omada-controller** | mbentley/omada-controller:5.13 | host | docker volumes (data+logs) | none | TP-Link AP management; host network for L2 discovery |

### Tier 2: Home Automation

| Service | Image | Network Mode | Storage | Dependencies | Notes |
|---------|-------|-------------|---------|-------------|-------|
| **homeassistant** | ghcr.io/home-assistant/home-assistant:stable | host | config dir, /run/dbus | mosquitto, matter-server | Privileged, needs dbus for BT, host network for mDNS/SSDP |
| **matter-server** | ghcr.io/home-assistant-libs/python-matter-server:stable | host | docker volume | none | Host network required for Matter protocol |

### Tier 3: Media Stack

| Service | Image | Network Mode | Storage | Dependencies | Notes |
|---------|-------|-------------|---------|-------------|-------|
| **jellyfin** | ghcr.io/hotio/jellyfin | bridge (8096) | config dir, /media, /photos | moosefs media+family | Needs /dev/dri for HW transcode (Intel QSV on NAS — Ivy Bridge fleet has no iGPU) |
| **qbittorrent** | ghcr.io/hotio/qbittorrent | bridge (8080,8118,7971) | config dir, /media | moosefs media | VPN (WireGuard), NET_ADMIN cap, sysctl tweaks |
| **prowlarr** | ghcr.io/hotio/prowlarr | bridge (9696) | config dir, /media | none | Indexer manager |
| **sonarr** | ghcr.io/hotio/sonarr | bridge (8989) | config dir, /media | prowlarr, qbittorrent | TV show management |
| **radarr** | ghcr.io/hotio/radarr | bridge (7878) | config dir, /media | prowlarr, qbittorrent | Movie management |
| **lidarr** | ghcr.io/hotio/lidarr | bridge (8686) | config dir, /media | prowlarr, qbittorrent | Music management |
| **bazarr** | cr.hotio.dev/hotio/bazarr | bridge (6767) | config dir, /media | sonarr, radarr | Subtitle management |
| **readarr** | ghcr.io/hotio/readarr | bridge (8787) | config dir, /media | prowlarr, qbittorrent | Ebook management (not running) |
| **speakarr** | ghcr.io/hotio/readarr | bridge (8788) | config dir, /media | prowlarr, qbittorrent | Audiobook management (readarr fork) |
| **sabnzbd** | cr.hotio.dev/hotio/sabnzbd | bridge (8282) | config dir, /media | none | Usenet downloader (not running) |

### Tier 4: Music Streaming

| Service | Image | Network Mode | Storage | Dependencies | Notes |
|---------|-------|-------------|---------|-------------|-------|
| **gonic** | ghcr.io/aleksclark/gonic | bridge (4747) | data dir, /music (ro), /playlists | moosefs media | Subsonic-compatible server (not running) |
| **airsonic** | ghcr.io/aleksclark/airsonic-refix | bridge (7979) | none | gonic | Web frontend for gonic |

### Tier 5: Photos & Sync

| Service | Image | Network Mode | Storage | Dependencies | Notes |
|---------|-------|-------------|---------|-------------|-------|
| **photoprism** | photoprism/photoprism | bridge (2342) | /photos (originals), storage dir | mariadb, /dev/dri | TensorFlow for face/image classification; Intel QSV |
| **mariadb** | mariadb:10.9 | bridge (internal 3306) | database dir | none | Only used by photoprism |
| **syncthing** | lscr.io/linuxserver/syncthing | bridge (8384, 22000, 21027) | config dir, /phone_data | moosefs family | Phone photo/file sync |

### Tier 6: Network Services (stay on NAS or dedicated)

| Service | Notes |
|---------|-------|
| **ddclient** | Dynamic DNS updater — systemd service on NAS |
| **iDrive** | Backup agent — systemd service on NAS |
| **NFS server** | Currently exports /storage — replaced by MooseFS |

---

## Migration Strategy

### Phase 0: Prerequisites
1. **Install Nomad** on all 3 fleet nodes + NAS (NAS as a client initially for gradual migration)
2. **Install Consul** for service discovery (or use Nomad's built-in service discovery)
3. **MooseFS mount on all Nomad clients** — each node mounts `/mnt/moosefs` via FUSE
4. **Config data migration** — copy all `*_config` dirs from NAS `/home/aleks/` to MooseFS `/configs/`
5. **Create a Nomad volume definition** for MooseFS paths using host_volume stanzas

### Phase 1: Infrastructure Services
**Target: node-1 (master node, most stable)**

1. **mosquitto** — straightforward, no special requirements
   - Config volume: `/mnt/moosefs/configs/mosquitto/`
   - Constraint: pin to single node (MQTT clients expect stable IP) OR use Consul DNS
   
2. **cloudflared** — stateless tunnel
   - Config: single YAML file
   - Can run anywhere, restart-safe

3. **omada-controller** — MUST run on a node that can do L2 broadcast to APs
   - Host network mode required
   - Constraint: pin to node on same L2 segment as TP-Link APs
   - MongoDB embedded, needs persistent volume

### Phase 2: Home Automation
**Target: node-2 (most RAM at 16GB)**

4. **matter-server** — host network, relatively lightweight
   - Must be reachable by HA on the same host or via IP
   
5. **homeassistant** — the big one
   - Host network (mDNS, SSDP, BLE)
   - Needs dbus access for Bluetooth (only works if the node has a BT adapter)
   - Privileged container
   - Constraint: must be co-located with or network-reachable to mosquitto and matter-server
   - **BLE dependency**: HA currently uses BLE proxy nodes (ESP32-C3), so physical BT adapter on the Nomad node is NOT required
   - Config volume: ~200MB, must be persistent

### Phase 3: Media Stack (Arr Suite)
**Target: spread across fleet, all need MooseFS /media mount**

6. **qbittorrent** — most complex due to VPN
   - NET_ADMIN capability, sysctl modifications
   - WireGuard VPN (TorGuard) — wg0 interface inside container
   - Constraint: needs `cap_add: NET_ADMIN` and sysctl support in Nomad
   - Pin to single node to avoid VPN IP churn
   
7. **prowlarr** → any node, lightweight
8. **sonarr** → any node, needs /media
9. **radarr** → any node, needs /media
10. **lidarr** → any node, needs /media
11. **bazarr** → any node, needs /media
12. **speakarr** → any node, needs /media

All *arr services need:
- MooseFS `/media` mount (read-write)
- Persistent config volume per service
- Inter-service communication (sonarr↔prowlarr, sonarr↔qbittorrent, etc.) — use Consul DNS or static IPs

### Phase 4: Media Servers
**Target: node-2 (most RAM for transcoding)**

13. **jellyfin** — media streaming
    - **HW transcoding issue**: NAS has Intel QSV (Broadwell Xeon iGPU). Fleet Ivy Bridge desktop CPUs have NO iGPU (they use discrete GPUs or none). Jellyfin will fall back to software transcoding.
    - Needs MooseFS `/media` + `/family/photos` mounts
    - RAM: can use 2-4GB for transcoding cache
    - Consider keeping on NAS if QSV transcoding matters

14. **gonic + airsonic** — music streaming
    - Lightweight, can run anywhere with MooseFS /media

### Phase 5: Photos & Sync

15. **mariadb** — pin to single node, persistent storage critical
    - InnoDB buffer pool = 512MB configured
    - Constraint: stable storage, not moved frequently
    
16. **photoprism** — photo management
    - **HW transcoding issue**: same as Jellyfin — uses /dev/dri for Intel QSV
    - TensorFlow for face detection — CPU-intensive
    - Needs MooseFS `/family/photos` mount
    - Consider keeping on NAS if QSV matters

17. **syncthing** — phone sync
    - Needs stable ports (22000/tcp, 22000/udp, 21027/udp)
    - MooseFS `/family/phones` mount
    - Pin to single node for stable sync targets

---

## Key Challenges

### 1. Hardware Transcoding (HW Accel)
The NAS Xeon E5-1620 v4 has Intel Quick Sync Video (QSV) via its iGPU. The fleet nodes are desktop Ivy Bridge CPUs which typically lack iGPU when paired with discrete graphics or on server boards. **Jellyfin and PhotoPrism will lose HW transcoding** on the fleet.

**Options:**
- a) Keep Jellyfin + PhotoPrism on the NAS (hybrid approach)
- b) Accept software transcoding (Ivy Bridge cores are decent, 3-4 simultaneous 1080p streams possible)
- c) Add a GPU to a fleet node (old Quadro P400 or Intel Arc A310 for QSV)

### 2. VPN for qBittorrent
qBittorrent runs with WireGuard VPN (TorGuard). This requires:
- `cap_add: NET_ADMIN`
- `sysctls: net.ipv4.conf.all.src_valid_mark=1`
- Nomad supports both via the Docker driver, but the VPN config (wg0.conf) must be in the container's config volume

### 3. Host Network Services
homeassistant, matter-server, and omada-controller all use `network_mode: host`. In Nomad, this means:
- They must use the `docker` driver with `network_mode = "host"`
- Port allocation is static (no Nomad port management)
- Cannot run multiple host-network services competing for the same ports on one node

### 4. Service Discovery
The *arr services currently communicate via Docker network DNS (container names). In Nomad:
- Use **Nomad service discovery** (built-in) or **Consul**
- Alternatively, use static IPs + known ports (simpler for a small cluster)
- The *arr configs will need URL updates (e.g., sonarr's qbittorrent URL)

### 5. Config State Migration
All `*_config` directories on the NAS (`/home/aleks/`) contain service state (databases, settings). These must be:
1. Copied to MooseFS (persistent across any node)
2. OR kept as Nomad host_volume paths on specific nodes
3. Using MooseFS with goal=2 for configs ensures redundancy

### 6. Omada Controller L2 Requirement
The TP-Link Omada controller needs L2 adjacency for AP discovery and adoption. It must run on a node directly connected to the same switch/VLAN as the APs.

---

## Storage Layout on MooseFS

```
/mnt/moosefs/
├── family/          (2CP - existing)
│   ├── photos/
│   ├── phones/
│   └── photoprism/
│       ├── storage/
│       └── database/
├── media/           (1CP - existing)
│   ├── tv/
│   ├── movies/
│   ├── music/
│   ├── audiobooks/
│   ├── books/
│   └── playlists/
├── tmp/             (1CP - existing)
└── configs/         (2CP - new, service configs)
    ├── homeassistant/
    ├── mosquitto/
    ├── jellyfin/
    ├── sonarr/
    ├── radarr/
    ├── lidarr/
    ├── prowlarr/
    ├── bazarr/
    ├── speakarr/
    ├── qbittorrent/
    ├── syncthing/
    ├── omada/
    ├── photoprism/
    ├── mariadb/       ← NOTE: MariaDB performance on FUSE may be poor
    └── cloudflared/
```

### MariaDB on MooseFS Warning
MariaDB (used by PhotoPrism) does heavy random I/O. Running it on MooseFS FUSE may have significant latency. Consider:
- Running MariaDB with a local disk volume on a pinned node
- Or switching PhotoPrism to SQLite (supported, simpler)

---

## Nomad Job Structure

```
nomad/
├── infrastructure/
│   ├── mosquitto.nomad.hcl
│   ├── cloudflared.nomad.hcl
│   └── omada.nomad.hcl
├── home-automation/
│   ├── homeassistant.nomad.hcl
│   └── matter-server.nomad.hcl
├── media/
│   ├── jellyfin.nomad.hcl
│   ├── qbittorrent.nomad.hcl
│   ├── prowlarr.nomad.hcl
│   ├── sonarr.nomad.hcl
│   ├── radarr.nomad.hcl
│   ├── lidarr.nomad.hcl
│   ├── bazarr.nomad.hcl
│   ├── speakarr.nomad.hcl
│   └── gonic.nomad.hcl
├── photos/
│   ├── photoprism.nomad.hcl
│   ├── mariadb.nomad.hcl
│   └── syncthing.nomad.hcl
└── music/
    └── airsonic.nomad.hcl
```

---

## Node Placement Plan

| Node | RAM | Services | Rationale |
|------|-----|----------|-----------|
| node-1 (8GB) | mosquitto, cloudflared, prowlarr, bazarr, lidarr | Lightweight services, MooseFS master here |
| node-2 (16GB) | homeassistant, matter-server, jellyfin, photoprism, mariadb, qbittorrent | RAM-heavy services, most RAM available |
| node-3 (8GB) | omada, sonarr, radarr, speakarr, syncthing, gonic, airsonic | Medium services, most storage |

Estimated RAM per node:
- node-1: ~2-3GB for services + MooseFS master (~2GB) = ~5GB / 8GB
- node-2: ~8-10GB for services + MooseFS chunkserver = ~11GB / 16GB
- node-3: ~3-4GB for services + MooseFS chunkserver = ~5GB / 8GB

---

## Services to Keep on NAS

| Service | Reason |
|---------|--------|
| **ddclient** | Trivial, runs as systemd unit, no benefit from migration |
| **iDrive** | Backup agent tied to local ZFS pool, needs direct disk access |
| **NFS server** | Deprecated — replaced by MooseFS. Shut down after migration complete |

---

## Migration Order & Rollback

1. Install Nomad + Consul on fleet
2. Set up MooseFS `/configs/` directory with 2CP
3. Copy all `*_config` dirs from NAS to MooseFS `/configs/`
4. Start Tier 1 services on fleet, verify, stop NAS copies
5. Update cloudflared tunnel / DNS / any external references
6. Start Tier 2 (HA), verify automations work
7. Start Tier 3 (*arr suite), update inter-service URLs
8. Start Tier 4-5 (media servers, photos)
9. Monitor for 1 week
10. Decommission NAS Docker stack

**Rollback**: At any point, `docker compose up` on NAS restores all services (data still on ZFS until fully decommissioned).

---

## Open Questions

1. **Do we want Consul or just Nomad built-in service discovery?** Consul adds complexity but gives health checks + DNS. For 3 nodes, Nomad built-in may suffice.
2. **Keep Jellyfin on NAS for HW transcoding?** Or accept software transcoding on fleet?
3. **MariaDB on local disk or MooseFS?** Performance vs redundancy tradeoff.
4. **Omada controller L2 requirement** — which fleet node is on the same L2 as the APs?
5. **Cloudflared tunnel token** — needs to be updated to point at fleet IP instead of NAS.
