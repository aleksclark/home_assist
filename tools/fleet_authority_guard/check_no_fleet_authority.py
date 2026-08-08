#!/usr/bin/env python3
"""Fail-closed guard: prevent reintroduction of active fleet authority in home_assist.

Blocks tracked (and optionally untracked) paths under fleet/ that look like
Ansible inventory/control-plane or fleet-owned Nomad jobspecs.

Allowlist is limited to migration pointer artifacts and explicitly retained
non-authoritative paths declared in fleet/MIGRATION_MANIFEST.json.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Iterable

TOOL_NAME = "check_no_fleet_authority"
TOOL_VERSION = "1.0.0"

DEFAULT_MANIFEST_REL = Path("fleet/MIGRATION_MANIFEST.json")

# Directory/file prefixes that constitute active fleet control-plane authority.
FORBIDDEN_EXACT = frozenset(
    {
        "fleet/ansible.cfg",
    }
)

FORBIDDEN_DIR_PREFIXES = (
    "fleet/inventory/",
    "fleet/group_vars/",
    "fleet/host_vars/",
    "fleet/roles/",
    "fleet/playbooks/",
    "fleet/nomad/",
    # alternate layout shapes
    "fleet/ansible/",
    "fleet/jobs/",  # fleet-iac uses jobs/ at repo root; must not reappear under fleet/
)

# Basename / path regexes for alternate filenames.
FORBIDDEN_PATH_REGEXES = (
    re.compile(r"(?i)^fleet/.*ansible\.cfg$"),
    re.compile(r"(?i)^fleet/.*/hosts\.ya?ml$"),
    re.compile(r"(?i)^fleet/.*/inventory\.ya?ml$"),
    re.compile(r"(?i)^fleet/.*\bsite\.ya?ml$"),
    re.compile(r"(?i)^fleet/.*\bupgrade\.ya?ml$"),
    re.compile(r"(?i)^fleet/.*deploy-blockyard\.ya?ml$"),
    re.compile(r"(?i)^fleet/.*blockyard-(restart|wipe-raft)\.ya?ml$"),
    re.compile(r"(?i)^fleet/.*\.nomad(\.hcl)?$"),
    re.compile(r"(?i)^fleet/.*/(group_vars|host_vars|playbooks|roles)(/|$)"),
)

# Default allowlist if manifest missing these keys (tests may supply minimal manifest).
DEFAULT_ALLOWLIST_PREFIXES = (
    "fleet/README.md",
    "fleet/MIGRATION_MANIFEST.json",
)


def _git_paths(root: Path, *, include_untracked: bool) -> list[str]:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    if not (root / ".git").exists() and not (root / ".git").is_file():
        # filesystem walk fallback for non-git fixtures
        paths: list[str] = []
        for dirpath, dirnames, filenames in os.walk(root):
            dirnames[:] = [d for d in dirnames if d not in {".git", "__pycache__"}]
            for name in filenames:
                full = Path(dirpath) / name
                rel = full.relative_to(root).as_posix()
                paths.append(rel)
        return sorted(paths)

    tracked = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        capture_output=True,
        check=True,
        env=env,
    )
    paths = [p for p in tracked.stdout.decode("utf-8").split("\0") if p]
    if include_untracked:
        untracked = subprocess.run(
            [
                "git",
                "-C",
                str(root),
                "ls-files",
                "-z",
                "--others",
                "--exclude-standard",
            ],
            capture_output=True,
            check=True,
            env=env,
        )
        paths.extend(p for p in untracked.stdout.decode("utf-8").split("\0") if p)
    return sorted(set(paths))


def _load_allowlist(root: Path, manifest_path: Path) -> tuple[set[str], tuple[str, ...]]:
    """Return (exact allow paths, allow prefixes)."""
    exact: set[str] = set(DEFAULT_ALLOWLIST_PREFIXES)
    prefixes: list[str] = list(DEFAULT_ALLOWLIST_PREFIXES)
    path = manifest_path if manifest_path.is_absolute() else root / manifest_path
    if not path.is_file():
        return exact, tuple(prefixes)
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return exact, tuple(prefixes)

    for item in data.get("allowlist_paths", []) or []:
        exact.add(str(item))
        prefixes.append(str(item))
    for item in data.get("allowlist_path_prefixes", []) or []:
        prefixes.append(str(item))

    # retain-non-authoritative entries are allowlisted as exact paths
    for entry in data.get("entries", []) or []:
        if not isinstance(entry, dict):
            continue
        if entry.get("action") == "retain-non-authoritative" and entry.get("path"):
            exact.add(str(entry["path"]))
            prefixes.append(str(entry["path"]))

    # always allow the manifest and README themselves
    try:
        man_rel = path.relative_to(root).as_posix()
    except ValueError:
        man_rel = DEFAULT_MANIFEST_REL.as_posix()
    exact.add(man_rel)
    exact.add("fleet/README.md")
    prefixes.extend([man_rel, "fleet/README.md"])
    return exact, tuple(sorted(set(prefixes)))


def _is_allowlisted(rel: str, exact: set[str], prefixes: Iterable[str]) -> bool:
    if rel in exact:
        return True
    for pref in prefixes:
        if not pref:
            continue
        if rel == pref.rstrip("/"):
            return True
        pref_n = pref if pref.endswith("/") else pref
        # prefix match only when allowlist entry ends with / or is a directory prefix
        if pref.endswith("/"):
            if rel.startswith(pref):
                return True
        else:
            # exact file or path-equal already handled; also allow children if entry is dir
            if rel.startswith(pref + "/"):
                return True
    return False


def _is_forbidden(rel: str) -> str | None:
    """Return reason string if forbidden, else None."""
    if not rel.startswith("fleet/"):
        return None
    if rel in FORBIDDEN_EXACT:
        return f"forbidden exact path: {rel}"
    for pref in FORBIDDEN_DIR_PREFIXES:
        if rel.startswith(pref) or rel == pref.rstrip("/"):
            return f"forbidden prefix path: {rel} (prefix {pref})"
    for rx in FORBIDDEN_PATH_REGEXES:
        if rx.search(rel):
            return f"forbidden path shape: {rel} (matched {rx.pattern})"
    return None


def check(root: Path, *, manifest: Path, include_untracked: bool = True) -> list[str]:
    exact, prefixes = _load_allowlist(root, manifest)
    errs: list[str] = []
    for rel in _git_paths(root, include_untracked=include_untracked):
        if not rel.startswith("fleet/"):
            continue
        if _is_allowlisted(rel, exact, prefixes):
            continue
        reason = _is_forbidden(rel)
        if reason:
            errs.append(reason)
    return errs


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--version", action="version", version=f"{TOOL_NAME} {TOOL_VERSION}")
    p.add_argument("--root", type=Path, default=Path.cwd())
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST_REL)
    p.add_argument("--json", action="store_true")
    p.add_argument(
        "--tracked-only",
        action="store_true",
        help="Only scan git-tracked paths (default also includes untracked).",
    )
    args = p.parse_args(argv)

    root = args.root.resolve()
    errs = check(root, manifest=args.manifest, include_untracked=not args.tracked_only)
    payload = {
        "tool": TOOL_NAME,
        "version": TOOL_VERSION,
        "ok": not errs,
        "error_count": len(errs),
        "errors": errs,
    }
    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        if errs:
            print(f"{TOOL_NAME}: FAIL", file=sys.stderr)
            for e in errs:
                print(f"  - {e}", file=sys.stderr)
        else:
            print(f"{TOOL_NAME}: ok")
    return 1 if errs else 0


if __name__ == "__main__":
    raise SystemExit(main())
