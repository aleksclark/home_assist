#!/usr/bin/env python3
"""Fail-closed merge-order gate for Task 9 home_assist retirement.

Home Assistant's Task 9 PR is merge-ready only after the pinned canonical
fleet-iac commit is an ancestor of fleet-iac mainline (origin/master).

Fleet-first contract:
- home_assist is public; aleksclark/fleet-iac is private.
- Hosted CI must materialize fleet-iac at path fleet-iac-canonical via a
  dedicated read-only SSH deploy key secret (FLEET_IAC_READ_SSH_KEY) —
  never assume GITHUB_TOKEN, a PAT/token checkout, or a repository variable
  path can see the private repo.
- Callers pass --fleet-iac explicitly (recommended). FLEET_IAC_REPO remains
  a local/dev override only.
- Default mode is fail-closed. Soft/report-only exists for local diagnostics
  only; CI must not use it for PR or master merge gates.

Canonical pin tracks the fleet-iac master merge SHA (not a floating PR head).
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

TOOL_NAME = "check_fleet_iac_merge_order"
TOOL_VERSION = "1.2.0"

DEFAULT_MANIFEST = Path("fleet/MIGRATION_MANIFEST.json")
DEFAULT_MAINLINE = "origin/master"
DEFAULT_FLEET_IAC_CHECKOUT = Path("fleet-iac-canonical")


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        env=env,
    )


def _load_manifest(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("manifest root must be object")
    return data


def _canonical_commit(data: dict) -> str:
    seq = data.get("merge_sequencing") or {}
    commit = seq.get("canonical_commit") or data.get("canonical_commit")
    if not isinstance(commit, str) or len(commit) != 40:
        raise ValueError(f"canonical_commit missing/invalid: {commit!r}")
    return commit


def check(
    *,
    root: Path,
    fleet_iac: Path,
    manifest_rel: Path,
    mainline_ref: str,
    soft: bool = False,
) -> tuple[int, list[str]]:
    """Return (exit_code, messages). exit_code 0 = merge-ready."""
    msgs: list[str] = []
    man_path = manifest_rel if manifest_rel.is_absolute() else root / manifest_rel
    if not man_path.is_file():
        msgs.append(f"missing migration manifest: {man_path}")
        return (0 if soft else 1, msgs)

    try:
        data = _load_manifest(man_path)
        canonical = _canonical_commit(data)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        msgs.append(f"manifest error: {exc}")
        return (0 if soft else 1, msgs)

    seq = data.get("merge_sequencing") or {}
    blocked = bool(seq.get("home_assist_merge_blocked_until_canonical_on_mainline", True))
    status = seq.get("canonical_mainline_status", "pending")
    mainline = seq.get("canonical_mainline_ref") or mainline_ref

    if not fleet_iac.exists():
        msgs.append(
            f"fleet-iac repo not found at {fleet_iac}; "
            f"cannot verify canonical {canonical[:7]} is on mainline {mainline}. "
            f"Hosted CI must checkout aleksclark/fleet-iac to fleet-iac-canonical "
            f"using secrets.FLEET_IAC_READ_SSH_KEY (read-only deploy key; fail-closed)."
        )
        # Fail closed for merge-ready checks when repo missing.
        return (0 if soft else 1, msgs)

    # Ensure object exists (or is fetchable — caller must fetch; we fail hard).
    cat = _git(fleet_iac, "cat-file", "-t", canonical)
    if cat.returncode != 0 or cat.stdout.strip() != "commit":
        msgs.append(
            f"canonical commit {canonical} not present in fleet-iac repo "
            f"(fetch required; fail-closed). stderr={cat.stderr.strip()}"
        )
        return (0 if soft else 1, msgs)

    # Resolve mainline tip (prefer fetched origin/master from private checkout).
    rev = _git(fleet_iac, "rev-parse", "--verify", mainline)
    if rev.returncode != 0 and mainline != mainline_ref:
        rev = _git(fleet_iac, "rev-parse", "--verify", mainline_ref)
    if rev.returncode != 0:
        msgs.append(
            f"mainline ref {mainline!r} not resolvable in fleet-iac: {rev.stderr.strip()}"
        )
        return (0 if soft else 1, msgs)
    mainline_sha = rev.stdout.strip()

    anc = _git(fleet_iac, "merge-base", "--is-ancestor", canonical, mainline_sha)
    if anc.returncode != 0:
        msgs.append(
            f"MERGE-ORDER GATE: canonical fleet-iac commit {canonical} is NOT an "
            f"ancestor of mainline {mainline} ({mainline_sha[:12]}). "
            f"Home Assistant Task 9 is NOT merge-ready until the canonical commit "
            f"is on fleet-iac mainline. status={status!r} blocked={blocked}."
        )
        return (0 if soft else 1, msgs)

    msgs.append(
        f"ok: canonical {canonical[:12]} is on fleet-iac mainline {mainline} "
        f"({mainline_sha[:12]}); home_assist Task 9 merge-order prerequisite satisfied."
    )
    return (0, msgs)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--version", action="version", version=f"{TOOL_NAME} {TOOL_VERSION}")
    p.add_argument("--root", type=Path, default=Path.cwd())
    p.add_argument(
        "--fleet-iac",
        type=Path,
        default=None,
        help=(
            "Path to fleet-iac checkout (CI: fleet-iac-canonical). "
            "Optional local override: env FLEET_IAC_REPO. "
            "Default path when unset: fleet-iac-canonical under --root."
        ),
    )
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--mainline-ref", default=DEFAULT_MAINLINE)
    p.add_argument(
        "--soft",
        action="store_true",
        help="Report-only: always exit 0 (local diagnostics only). Default is fail-closed.",
    )
    p.add_argument(
        "--report-only",
        action="store_true",
        help="Alias for --soft (local diagnostics only; CI must not use this).",
    )
    args = p.parse_args(argv)

    root = args.root.resolve()
    fleet_iac = args.fleet_iac
    if fleet_iac is None:
        env_path = os.environ.get("FLEET_IAC_REPO")
        if env_path:
            fleet_iac = Path(env_path)
        else:
            # Prefer the CI-materialized private checkout path under root.
            candidate = root / DEFAULT_FLEET_IAC_CHECKOUT
            fleet_iac = candidate if candidate.exists() else DEFAULT_FLEET_IAC_CHECKOUT

    soft = bool(args.soft or args.report_only)
    code, msgs = check(
        root=root,
        fleet_iac=Path(fleet_iac).resolve(),
        manifest_rel=args.manifest,
        mainline_ref=args.mainline_ref,
        soft=soft,
    )
    prefix = f"{TOOL_NAME}: "
    if code == 0 and not any(m.startswith("MERGE-ORDER") for m in msgs):
        print(prefix + ("SOFT-OK " if soft else "ok ") + "; ".join(msgs))
    else:
        stream = sys.stderr if code else sys.stdout
        label = "SOFT-FAIL" if soft and any("GATE" in m or "not" in m.lower() for m in msgs) else (
            "FAIL" if code else "ok"
        )
        print(f"{prefix}{label}", file=stream)
        for m in msgs:
            print(f"  - {m}", file=stream)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
