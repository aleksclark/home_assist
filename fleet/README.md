# Fleet Management

Ansible-based management for a heterogeneous compute fleet running Arch Linux.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     CONTROL PLANE                            │
│                                                              │
│  This machine (Aleks's workstation)                          │
│  ├── Ansible controller                                      │
│  ├── Consul server (single-node for now)                     │
│  └── Nomad server (single-node for now)                      │
└──────────────┬───────────────────────────────────────────────┘
               │ SSH + Consul gossip + Nomad RPC
    ┌──────────┼──────────┐
    │          │          │
    ▼          ▼          ▼
┌────────┐ ┌────────┐ ┌────────┐
│ Node 1 │ │ Node 2 │ │ Node N │   Old desktops, varying specs
│        │ │        │ │        │
│ Consul │ │ Consul │ │ Consul │   Agent mode
│ Nomad  │ │ Nomad  │ │ Nomad  │   Client mode
│        │ │        │ │        │
│ Roles: │ │ Roles: │ │ Roles: │   Per-node via host_vars
│ storage│ │ docker │ │ both   │
│ byard  │ │        │ │        │
└────────┘ └────────┘ └────────┘
```

## Stack

| Component | Purpose | Package Source |
|-----------|---------|---------------|
| **Ansible** | Configuration management | Control machine only (pip/pacman) |
| **Nomad** | Workload scheduling | `pacman -S nomad` (official Extra repo) |
| **Consul** | Service discovery + health | `pacman -S consul` (official Extra repo) |
| **Docker** | Container runtime for Nomad | `pacman -S docker` |
| **Blockyard** | Distributed block storage | Built from source, deployed as binary |
| **Snapper + snap-pac** | Btrfs snapshot rollback | `pacman -S snapper snap-pac grub-btrfs` |

## Disk Layout (per node)

```
/dev/sda (or nvme0n1)           — OS disk
  ├── /dev/sda1  EFI (512MB, FAT32)
  └── /dev/sda2  Root (rest, btrfs)
       ├── @           → /
       ├── @home       → /home
       ├── @snapshots  → /.snapshots
       └── @var_log    → /var/log

/dev/sdb, /dev/sdc, ...        — Data disks (XFS, for blockyard)
  └── mounted at /data/disk0, /data/disk1, ...
```

## Quick Start

### 1. Bootstrap a new machine

Flash `fleet/archiso/` to USB, boot the target machine from it.
The ISO auto-starts SSH so an agent can connect and run the install.

```bash
# Build the ISO
cd fleet/archiso && ./build.sh --inject-key ~/.ssh/id_ed25519.pub

# Write to USB
sudo dd bs=4M if=out/fleet-bootstrap-*.iso of=/dev/sdX status=progress oflag=sync

# Boot the target machine, then tell your AI agent:
# "New machine is up at 192.168.0.15, bootstrap it as node1"
#
# The agent will:
#   1. SSH in and inspect hardware (disks, CPU, RAM, GPU, boot mode)
#   2. Propose a disk layout and ask you to confirm
#   3. Partition, format, pacstrap, configure
#   4. Reboot into the installed system
#   5. Add to inventory and run Ansible converge
#
# See: hermes skill fleet-bootstrap
```

### 2. Deploy to existing fleet

```bash
# Full converge
ansible-playbook -i inventory/hosts.yml playbooks/site.yml

# Rolling upgrade with snapshot + rollback
ansible-playbook -i inventory/hosts.yml playbooks/upgrade.yml
```

### 3. Deploy blockyard

```bash
ansible-playbook -i inventory/hosts.yml playbooks/deploy-blockyard.yml --limit storage
```

## Directory Structure

```
fleet/
├── README.md
├── ansible.cfg
├── inventory/
│   └── hosts.yml           # All machines, grouped by role
├── group_vars/
│   ├── all.yml             # Common: users, SSH, base packages, mirrors
│   ├── storage.yml         # Blockyard nodes: XFS disks, data paths
│   └── compute.yml         # Docker/container workload nodes
├── host_vars/
│   └── example-node.yml    # Per-machine overrides (disk layout, specs)
├── roles/
│   ├── base/               # Arch baseline: pacman, snapper, users, SSH, sysctl
│   ├── nomad/              # Nomad client agent
│   ├── consul/             # Consul client agent
│   ├── blockyard/          # Blockyard node daemon
│   └── docker/             # Docker + Nomad docker driver
├── playbooks/
│   ├── site.yml            # Full converge (all roles)
│   ├── upgrade.yml         # Rolling upgrade with snapshot/rollback
│   └── deploy-blockyard.yml
└── archiso/                # Custom Arch ISO for USB bootstrap
    ├── build.sh            # Build script (--inject-key to add SSH pubkey)
    ├── profiledef.sh
    ├── packages.x86_64
    └── airootfs/           # Files baked into the ISO (SSH enabled, root:fleet)
```
