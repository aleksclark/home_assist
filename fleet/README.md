# MOVED — Fleet Ansible / Nomad authority retired

**This tree is no longer an executable fleet control plane.**

Sole active authority lives in:

| Concern | Canonical location |
|---------|-------------------|
| Ansible inventory, group/host vars, roles, playbooks | [`aleksclark/fleet-iac`](https://github.com/aleksclark/fleet-iac) → `platform/ansible` |
| Fleet-owned Nomad jobspecs | `aleksclark/fleet-iac` → `jobs/` |
| Credential-free bootstrap ISO | `aleksclark/fleet-iac` → `platform/archiso` |
| Monitoring scripts used by fleet jobs | `aleksclark/fleet-iac` → `platform/scripts` |
| Project-owned jobs (e.g. minisplit-otel-poller) | remain under this repo’s `services/` (not under `fleet/`) |

## What was removed

Duplicate executable authority previously under `fleet/` — `ansible.cfg`, `inventory/`, `group_vars/`, `host_vars/`, active `roles/`, `playbooks/`, and fleet platform `nomad/` jobspecs — has been deleted from `home_assist`.

See the value-free classification/provenance report:

- [`MIGRATION_MANIFEST.json`](./MIGRATION_MANIFEST.json)

## Do not use this directory as a control plane

- Do **not** run Ansible against paths under `home_assist/fleet`.
- Do **not** submit Nomad jobs from `home_assist/fleet` (there are none).
- Do **not** reintroduce inventories, roles, playbooks, or fleet Nomad jobspecs here — CI fails closed on reintroduction (`tools/fleet_authority_guard/`).

Home Assistant application config, ESPHome/device sources, and project jobspecs under `services/` are unchanged and remain in this repository.
