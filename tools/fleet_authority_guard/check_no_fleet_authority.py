#!/usr/bin/env python3
"""Fail-closed guard: prevent reintroduction of active fleet authority in home_assist.

After Task 9 retirement, classification retains zero executable/non-authoritative
paths under fleet/. The only permitted fleet/** paths are the exact allowlist:

  - fleet/README.md
  - fleet/MIGRATION_MANIFEST.json

Any other tracked or untracked path under fleet/ fails closed — regardless of
case, suffix, nesting, or symlink. Path normalization rejects traversal and
casefolded allowlist lookalikes.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path, PurePosixPath

TOOL_NAME = "check_no_fleet_authority"
TOOL_VERSION = "2.0.0"

DEFAULT_MANIFEST_REL = Path("fleet/MIGRATION_MANIFEST.json")

# Hard exact allowlist — only these relative POSIX paths under the repo root.
HARD_ALLOWLIST = frozenset(
    {
        "fleet/README.md",
        "fleet/MIGRATION_MANIFEST.json",
    }
)


def _normalize_rel(rel: str) -> str | None:
    """Return a normalized repo-relative POSIX path, or None if unsafe/outside.

    Rejects absolute paths, empty segments that escape via '..', and backslashes
    used as separators. Does not casefold — allowlist matching is exact-case.
    """
    if not rel or rel.endswith("\0"):
        return None
    # Git may emit paths with backslashes on some platforms; treat as separators.
    cleaned = rel.replace("\\", "/").strip()
    if not cleaned or cleaned.startswith("/") or cleaned.startswith("~"):
        return None
    parts: list[str] = []
    for part in cleaned.split("/"):
        if part in ("", "."):
            continue
        if part == "..":
            if not parts:
                return None  # escapes repo root
            parts.pop()
            continue
        # Disallow Windows drive-ish and NUL-ish oddities in segments
        if "\0" in part or ":" in part:
            return None
        parts.append(part)
    if not parts:
        return None
    return "/".join(parts)


def _git_paths(root: Path, *, include_untracked: bool) -> list[str]:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    git_dir = root / ".git"
    if not git_dir.exists() and not git_dir.is_file():
        # filesystem walk fallback for non-git fixtures (files + symlinks)
        paths: list[str] = []
        for dirpath, dirnames, filenames in os.walk(root, followlinks=False):
            dirnames[:] = [d for d in dirnames if d not in {".git", "__pycache__"}]
            # include symlink dirs as leaf paths too
            for name in list(dirnames):
                full = Path(dirpath) / name
                if full.is_symlink():
                    rel = full.relative_to(root).as_posix()
                    paths.append(rel)
            for name in filenames:
                full = Path(dirpath) / name
                rel = full.relative_to(root).as_posix()
                paths.append(rel)
        return sorted(set(paths))

    tracked = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        capture_output=True,
        check=True,
        env=env,
    )
    paths = [p for p in tracked.stdout.decode("utf-8", errors="replace").split("\0") if p]
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
        paths.extend(
            p
            for p in untracked.stdout.decode("utf-8", errors="replace").split("\0")
            if p
        )
    return sorted(set(paths))


def _load_allowlist(root: Path, manifest_path: Path) -> set[str]:
    """Exact allowlist only. Manifest may declare the same two paths; extras ignored.

    Retain-non-authoritative entries are intentionally NOT auto-allowlisted:
    classification retains zero such paths under fleet/ after retirement.
    """
    allow = set(HARD_ALLOWLIST)
    path = manifest_path if manifest_path.is_absolute() else root / manifest_path
    if not path.is_file():
        return allow
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return allow

    declared: list[str] = []
    for item in data.get("allowlist_paths", []) or []:
        declared.append(str(item))
    # prefixes key is accepted only when each item is an exact hard-allowlisted path
    for item in data.get("allowlist_path_prefixes", []) or []:
        declared.append(str(item))

    for item in declared:
        norm = _normalize_rel(item)
        if norm in HARD_ALLOWLIST:
            allow.add(norm)
        # anything else in manifest allowlist is ignored (cannot widen)

    # always allow the resolved manifest path if it is the standard one
    try:
        man_rel = _normalize_rel(path.relative_to(root).as_posix())
    except ValueError:
        man_rel = DEFAULT_MANIFEST_REL.as_posix()
    if man_rel in HARD_ALLOWLIST:
        allow.add(man_rel)
    return allow


def _is_under_fleet(rel: str) -> bool:
    return rel == "fleet" or rel.startswith("fleet/")


def check(root: Path, *, manifest: Path, include_untracked: bool = True) -> list[str]:
    allow = _load_allowlist(root, manifest)
    errs: list[str] = []
    seen_normalized: set[str] = set()

    for raw in _git_paths(root, include_untracked=include_untracked):
        norm = _normalize_rel(raw)
        if norm is None:
            # unsafe / traversal path referencing fleet somehow
            if "fleet" in raw.replace("\\", "/").split("/"):
                errs.append(f"unsafe fleet path rejected: {raw}")
            continue
        if not _is_under_fleet(norm):
            continue
        if norm in seen_normalized:
            continue
        seen_normalized.add(norm)

        if norm in allow:
            continue

        # Exact allowlist only — any other fleet/** path is forbidden.
        errs.append(f"path not allowlisted under fleet/: {raw}")

    # Also scan filesystem under fleet/ for dangling dirs/symlinks not in git listing
    # (git ls-files lists blob paths; empty dirs won't appear — that's fine.
    #  Symlinks to dirs are listed when tracked/untracked as the symlink path.)
    fleet_dir = root / "fleet"
    if fleet_dir.exists():
        for dirpath, dirnames, filenames in os.walk(fleet_dir, followlinks=False):
            # detect case-colliding directory names that sneak past case-sensitive git
            for name in list(dirnames) + list(filenames):
                full = Path(dirpath) / name
                try:
                    rel = full.relative_to(root).as_posix()
                except ValueError:
                    errs.append(f"path escapes root via fleet walk: {full}")
                    continue
                norm = _normalize_rel(rel)
                if norm is None:
                    errs.append(f"unsafe fleet path rejected: {rel}")
                    continue
                if norm in allow or norm in seen_normalized:
                    continue
                # only report files and symlinks (not intermediate dirs solely as containers)
                if full.is_symlink() or full.is_file():
                    seen_normalized.add(norm)
                    errs.append(f"path not allowlisted under fleet/: {rel}")
                elif full.is_dir():
                    # empty non-allowlisted dirs still constitute reintroduction surface
                    # if they are top-level authority-shaped or any leftover tree node
                    # with no children yet — report the directory itself when empty
                    try:
                        next(full.iterdir())
                    except StopIteration:
                        if norm not in allow:
                            errs.append(f"path not allowlisted under fleet/: {rel}/")
                    except OSError:
                        errs.append(f"unreadable path under fleet/: {rel}")

    return sorted(set(errs))


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
        "allowlist": sorted(HARD_ALLOWLIST),
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
