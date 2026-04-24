# NAS → Nomad Fleet Migration Plan

## Current State

### NAS Hardware (192.168.0.3 — "stash") → future node-4
- Xeon E5-1620 v4 (4C/8T @ 3.5GHz), 32GB RAM
- Intel QSV capable (Broadwell Xeon iGPU)
- 1x 476GB SSD (OS)
- 4x 5.5TB HDD (ZFS raidz1 pool — 10.9TB raw, ~10.5TB used)
- ZFS pool (`pool0`) → `/storage` with datasets: `/storage/media`, `/storage/family`, `/storage/tmp`
- Running: Docker (docker-compose), ddclient, iDrive backup

### Fleet Nodes (Nomad targets)
| Node | IP | CPU | RAM | Disks | Network | MooseFS Role |
|------|-----|-----|-----|-------|---------|-------------|
| node-1 | 192.168.0.23 | Ivy Bridge | 8GB | 1x 3.6TB + 2x 1.8TB | bond0 (1G+2.5G ALB) | master + chunkserver |
| node-2 | 192.168.0.24 | Ivy Bridge | 16GB | 2x 3.6TB + 1x 1.8TB | bond0 (2.5G ALB) | chunkserver + metalogger |
| node-3 | 192.168.0.89 | Ivy Bridge | 8GB | 1x 3.6TB + 2x 1.8TB | bond0 (2.5G ALB) | chunkserver |
| **node-4** | **192.168.0.3** | **Broadwell Xeon** | **32GB** | **4x 5.5TB** | **TBD** | **chunkserver (future)** |

### Distributed Storage
- MooseFS 4.58.4 mounted at `/mnt/moosefs` via FUSE
- Directories: `/family` (2CP), `/media` (1CP), `/tmp` (1CP)
- ✅ Data migration from NAS ZFS → MooseFS complete (family, media, tmp)
- Current cluster: ~26TB raw, ~10TB available
- After node-4 joins: total raw capacity ~48TB (current ~26TB + node-4 ~22TB)

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
| **jellyfin** | ghcr.io/hotio/jellyfin | bridge (8096) | config dir, /media, /photos | moosefs media+family | Needs /dev/dri for HW transcode |
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

### Systemd Services on NAS

| Service | Notes | Migration |
|---------|-------|-----------|
| **ddclient** | Dynamic DNS updater | Move to any fleet node as systemd unit |
| **iDrive** | Backup agent | Replace with MooseFS-aware backup (or drop) |
| **NFS server** | Exports /storage | Already replaced by MooseFS — decommission |

---

## Migration Execution Plan

The key constraint: the NAS must be wiped and reinstalled as node-4, but it currently
runs all services AND is the source of truth for data still being rsynced to MooseFS.
The plan minimizes downtime by migrating services in waves to the fleet BEFORE touching
the NAS, then converting the NAS last.

### Phase 0: Foundation (no disruption)

**Goal**: Get Nomad running on the fleet, get all data onto MooseFS, copy all configs.

1. Install Nomad on all 3 fleet nodes (node-1 as Nomad server, node-2/3 as clients)
2. MooseFS FUSE mount on all fleet nodes at `/mnt/moosefs`
3. ✅ Complete the rsync from NAS ZFS → MooseFS (family, media, tmp) — DONE
4. Verify data integrity: spot-check file counts and sizes match
5. Create `/mnt/moosefs/configs/` directory with 2CP storage class
6. Copy ALL service config dirs from NAS to MooseFS:
   ```
   rsync -a /home/aleks/*_config /mnt/moosefs/configs/
   rsync -a /home/aleks/mosquitto /mnt/moosefs/configs/
   rsync -a /home/aleks/cloudflared_config /mnt/moosefs/configs/
   ```
7. Copy docker volumes (omada-data, omada-logs, matter-data) to MooseFS:
   ```
   docker cp omada-controller:/opt/tplink/EAPController/data /mnt/moosefs/configs/omada/data
   docker cp omada-controller:/opt/tplink/EAPController/logs /mnt/moosefs/configs/omada/logs
   ```
8. Install ddclient on a fleet node (node-1), configure identically, but don't start yet

**Verification checkpoint**: All data and configs exist on MooseFS. NAS still runs everything.

### Phase 1: Migrate stateless/infrastructure services (~5 min downtime per service)

**Services**: mosquitto, cloudflared, omada-controller, ddclient

For each service:
1. Write Nomad job file
2. `nomad job run <service>.nomad.hcl` — start on fleet
3. Verify fleet copy is healthy
4. `docker stop <service>` on NAS
5. Update any DNS/references

**Order & placement**:
- **mosquitto** → node-1 — start first, everything depends on it. Stop NAS copy once fleet MQTT is accepting connections. Update HA config to point to node-1 IP (or use DNS).
- **cloudflared** → node-1 — update tunnel config to point at fleet SSH
- **omada-controller** → whichever fleet node is on the same L2 as APs (host network). Copy omada data volume first.
- **ddclient** → node-1 — enable systemd unit

**Disruption**: MQTT clients (HA, ESPHome devices) briefly disconnect when mosquitto moves. ESPHome devices auto-reconnect. HA needs config update for new MQTT broker IP.

### Phase 2: Migrate Home Automation (~10 min downtime)

**Services**: matter-server, homeassistant

1. Final rsync of HA config: `rsync -a /home/aleks/homeassistant_config/ /mnt/moosefs/configs/homeassistant_config/`
2. Update HA `configuration.yaml` on MooseFS: set MQTT broker to mosquitto's new fleet IP
3. Start **matter-server** on node-2 via Nomad (host network)
4. Start **homeassistant** on node-2 via Nomad (host network, privileged)
5. Verify: HA dashboard loads, automations fire, MQTT entities report, Matter devices connect
6. `docker stop homeassistant matter-server` on NAS

**Disruption**: Home automation offline for ~10 min during cutover. Lights/climate still work (local ESPHome control), but automations and dashboard are down. Matter devices need to re-discover the server at its new IP.

### Phase 3: Migrate Media Stack (~15 min downtime for arr suite)

**Services**: qbittorrent, prowlarr, sonarr, radarr, lidarr, bazarr, speakarr

This is a batch migration. All *arr services talk to each other, so moving them individually
creates cross-service communication issues. Move them all at once.

1. Final rsync of all *_config dirs to MooseFS
2. Stop ALL *arr services + qbittorrent on NAS: `docker stop sonarr radarr lidarr prowlarr bazarr speakarr qbittorrent`
3. Update config files on MooseFS: each *arr service's config.xml has connection URLs for qbittorrent and prowlarr — update these to use fleet IPs or Nomad service addresses
4. Start all via Nomad:
   - **qbittorrent** → node-2 (VPN, needs NET_ADMIN, pin to this node)
   - **prowlarr** → node-1
   - **sonarr** → node-3
   - **radarr** → node-3
   - **lidarr** → node-1
   - **bazarr** → node-1
   - **speakarr** → node-3
5. Verify: prowlarr indexes, sonarr/radarr can reach qbittorrent, downloads work

**Disruption**: No new downloads for ~15 min. Existing media playback unaffected (Jellyfin still on NAS).

### Phase 4: Migrate Media Servers & Photos (~10 min downtime)

**Services**: jellyfin, photoprism, mariadb, syncthing, gonic, airsonic

1. Final rsync of jellyfin_config, photoprism storage/database, syncthing_config
2. **MariaDB** first: start on node-2 with local disk volume (NOT MooseFS — InnoDB needs low-latency I/O). Copy database dump instead:
   ```
   docker exec aleks-mariadb-1 mysqldump -u root -p photoprism > /mnt/moosefs/configs/mariadb/photoprism.sql
   ```
   Then restore on fleet MariaDB.
3. Start **photoprism** on node-2 (co-locate with MariaDB, has most RAM for TensorFlow)
4. Start **jellyfin** on node-2 (most RAM for transcoding; loses QSV, use software transcoding initially)
5. Start **syncthing** on node-3 (pin for stable sync endpoints)
6. Start **gonic + airsonic** on node-1 (lightweight)
7. Stop NAS copies of all these services

**Disruption**: Jellyfin streaming down ~10 min. PhotoPrism down ~10 min. Phone sync pauses.

### Phase 5: Verify everything, drain NAS (1 week soak)

1. All services running on fleet via Nomad
2. NAS Docker stack fully stopped: `docker compose down` on NAS
3. Monitor for 1 week:
   - HA automations running correctly
   - Media downloads completing
   - Jellyfin streaming stable (check software transcoding performance)
   - Phone syncthing reconnected
   - No missing data on MooseFS
4. Keep NAS running (Docker off) as fallback — ZFS data still intact

**Rollback**: `docker compose up` on NAS restores all services instantly. MooseFS configs can be rsynced back.

### Phase 6: Convert NAS to node-4

Once confident the fleet is stable:

1. **Final data verification**: compare NAS ZFS vs MooseFS file counts for family/media/tmp
2. **Export ZFS pool**: `zpool export pool0` (preserves data on the disks just in case)
3. **Reinstall Arch Linux** using fleet-bootstrap process (same archiso as other nodes)
4. **Configure as node-4**:
   - Install Nomad client
   - Install MooseFS chunkserver
   - Format the 4x 5.5TB drives as individual XFS filesystems (no ZFS/RAID — MooseFS handles redundancy)
   - Mount as `/data/disk0` through `/data/disk3`
   - Configure mfshdd.cfg with all 4 disks
   - Start chunkserver, let it register with master
5. **Join Nomad cluster** as a client
6. **Rebalance services**: node-4 is the beefiest node (32GB RAM, Xeon, QSV) — move Jellyfin and PhotoPrism there for HW transcoding via /dev/dri

**Post-conversion capacity**:
- MooseFS gains ~22TB raw from node-4's 4x 5.5TB drives
- Total cluster: ~48TB raw
- With 2CP: ~24TB usable for replicated data
- With 1CP: ~48TB usable for unreplicated data

### Phase 7: Post-conversion optimization

1. **Move Jellyfin → node-4**: gets Intel QSV back via /dev/dri
2. **Move PhotoPrism + MariaDB → node-4**: QSV for video, 32GB RAM for TensorFlow + InnoDB
3. **Promote MooseFS storage class**: with 4 nodes, consider upgrading media from 1CP to 2CP (enough capacity now)
4. **Move MooseFS metalogger → node-4**: most RAM, best metalogger candidate
5. **Consider MooseFS master failover**: Pro edition not needed, but metalogger on node-4 gives fast recovery if node-1 dies
6. **Re-evaluate EC**: with 4 nodes, still can't do EC 4+1 (need 5), but closer to enabling it with one more node
7. **ddclient**: move to node-4 or any stable node
8. **iDrive replacement**: configure MooseFS-level backups (or install iDrive on node-4 backing up /mnt/moosefs)

---

## Revised Node Placement (after node-4 joins)

| Node | RAM | Role | Services |
|------|-----|------|----------|
| node-1 (8GB) | MooseFS master, Nomad server | mosquitto, cloudflared, ddclient, prowlarr, bazarr, lidarr |
| node-2 (16GB) | MooseFS chunkserver | homeassistant, matter-server, qbittorrent |
| node-3 (8GB) | MooseFS chunkserver | omada, sonarr, radarr, speakarr, gonic, airsonic |
| **node-4 (32GB)** | **MooseFS chunkserver + metalogger** | **jellyfin, photoprism, mariadb, syncthing** |

Estimated RAM per node:
- node-1: ~2-3GB services + ~2GB MooseFS master = ~5GB / 8GB
- node-2: ~4-5GB services + chunkserver = ~6GB / 16GB ← room to grow
- node-3: ~3-4GB services + chunkserver = ~5GB / 8GB
- node-4: ~6-8GB services (Jellyfin+PhotoPrism+MariaDB) + chunkserver + metalogger = ~10GB / 32GB ← room to grow

Storage per node (MooseFS chunkserver):
- node-1: 1x 3.6TB + 2x 1.8TB = ~7.2TB raw
- node-2: 2x 3.6TB + 1x 1.8TB = ~9.0TB raw
- node-3: 1x 3.6TB + 2x 1.8TB = ~7.2TB raw
- node-4: 4x 5.5TB = ~22TB raw
- **Total: ~45.4TB raw**

---

## Key Challenges

### 1. Hardware Transcoding (HW Accel)
The NAS Xeon E5-1620 v4 has Intel Quick Sync Video (QSV) via its iGPU. The Ivy Bridge fleet nodes lack iGPU.

**Resolution**: Jellyfin and PhotoPrism run on fleet (software transcoding) during Phase 3-5, then move to node-4 after conversion to regain QSV. Temporary degradation of ~1 week.

### 2. VPN for qBittorrent
qBittorrent runs with WireGuard VPN (TorGuard). Requires `cap_add: NET_ADMIN`, sysctl modifications. Nomad Docker driver supports both.

### 3. Host Network Services
homeassistant, matter-server, and omada-controller use `network_mode: host`. In Nomad, use Docker driver with `network_mode = "host"` and static port allocation.

### 4. Service Discovery
The *arr services currently communicate via Docker network DNS. In Nomad:
- Use static IPs + known ports (simplest for a 4-node cluster)
- Update each *arr service's config to point to the correct fleet node IP
- Alternatively, use Nomad service discovery with template stanzas

### 5. Data Completeness Before NAS Wipe
**CRITICAL**: Before Phase 6, verify:
- `find /storage/family -type f | wc -l` on NAS vs `find /mnt/moosefs/family -type f | wc -l`
- Same for media and tmp
- Spot-check large files (checksums on a sample)
- All service configs present in `/mnt/moosefs/configs/`

### 6. ZFS → Individual Disks
The NAS currently uses ZFS raidz1 across 4x 5.5TB. Converting to node-4 means:
- Breaking the ZFS pool (export first, drives retain data for emergency recovery)
- Formatting each drive individually as XFS
- MooseFS handles the redundancy instead of ZFS
- Net capacity gain: ZFS raidz1 gives ~16.5TB usable; MooseFS with 4 individual disks gives ~22TB raw (all contributed to the cluster pool)

### 7. Omada Controller L2 Requirement
Must run on a node directly connected to the same switch/VLAN as the TP-Link APs. Verify which fleet node satisfies this before migration.

### 8. MariaDB Storage
MariaDB (PhotoPrism's database) needs low-latency I/O. Options:
- a) Run on node-4's SSD (476GB OS disk has plenty of room) ← recommended
- b) Run on MooseFS (higher latency, but simpler)
- c) Switch PhotoPrism to SQLite (simplest, acceptable for single-user)

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
├── media/           (1CP → consider 2CP after node-4 joins)
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
    ├── mariadb/
    └── cloudflared/
```

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

## Disruption Summary

| Phase | Duration | What's Down | What Still Works |
|-------|----------|-------------|-----------------|
| Phase 0 | 0 | Nothing | Everything |
| Phase 1 | ~5 min | MQTT briefly, AP management briefly | HA (reconnects), media, all else |
| Phase 2 | ~10 min | HA dashboard & automations | Lights (local ESPHome), media, downloads |
| Phase 3 | ~15 min | New downloads | Existing media playback, HA, everything else |
| Phase 4 | ~10 min | Jellyfin, PhotoPrism, Syncthing | HA, downloads, everything else |
| Phase 5 | 0 | Nothing (soak period) | Everything on fleet |
| Phase 6 | ~2 hrs | Nothing (NAS already drained) | Everything on fleet |
| Phase 7 | ~30 min | Brief service restarts as they move to node-4 | Most services (rolling) |

**Total user-facing downtime**: ~40 minutes spread across Phases 1-4, never all services at once.

---

## Open Questions

1. **Consul or Nomad built-in service discovery?** For 4 nodes, Nomad built-in + static IPs is likely sufficient.
2. **MariaDB on SSD or MooseFS?** Recommend node-4 SSD after conversion.
3. **Omada controller L2 requirement** — which fleet node is on the same L2 as the APs?
4. **Media storage class after node-4**: upgrade from 1CP to 2CP? With ~48TB raw, 2CP for everything gives ~24TB usable vs current ~10TB available.
5. **iDrive replacement**: MooseFS-level backup strategy? Or reinstall iDrive on node-4?
6. **Cloudflared tunnel token** — needs to be regenerated/updated for fleet.
