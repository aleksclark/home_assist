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

## Merge-order prerequisite (fail-closed, fleet-first)

Home Assistant Task 9 is **not merge-ready** until the pinned canonical fleet-iac commit
`47c45e83b03ed9022d8561fac07a76d356309665` is an ancestor of fleet-iac **mainline**
(`origin/master`). That pin is the fleet-iac **master merge SHA** of PR #124
(post-merge CI green run `31240176403`); `merge_sequencing.canonical_mainline_status`
is `merged`. The gate still hard-checks ancestry (does not trust status alone).

- Gate script: `tools/fleet_authority_guard/check_fleet_iac_merge_order.py`
- Manifest field: `merge_sequencing.canonical_mainline_status` (currently `merged`)
- CI hard-fails on **pull_request and master push** until the canonical commit is on
  fleet-iac `origin/master` (no soft/`continue-on-error` path)
- Hosted runners materialize private `aleksclark/fleet-iac` at `fleet-iac-canonical`
  via repository secret `FLEET_IAC_READ_SSH_KEY` (read-only SSH deploy key on
  `aleksclark/fleet-iac` only — least privilege; no PAT/token, no 1Password).
  `GITHUB_TOKEN` for this public repo cannot read the private sibling — missing
  secret fails closed. Never commit or log the key material.

## Do not use this directory as a control plane

- Do **not** run Ansible against paths under `home_assist/fleet`.
- Do **not** submit Nomad jobs from `home_assist/fleet` (there are none).
- Do **not** reintroduce *any* path under `fleet/` other than this README and `MIGRATION_MANIFEST.json` — CI fails closed on an exact allowlist (`tools/fleet_authority_guard/`), including case variants (`FLEET/`, `Fleet/`), symlinks, untracked files, and non-regular allowlist paths.

Home Assistant application config, ESPHome/device sources, and project jobspecs under `services/` are unchanged and remain in this repository.
