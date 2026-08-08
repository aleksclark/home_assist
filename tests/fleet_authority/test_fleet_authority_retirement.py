"""Task 9: retire duplicate fleet Ansible/Nomad authority from home_assist.

TDD contracts:
- exhaustive value-free classification of every pre-migration fleet/** path
- removed paths absent from the working tree / git index
- MOVED pointer README is correct and non-executable
- retained artifacts cannot execute Ansible/Nomad authority
- minisplit / project / device sources remain
- CI guard fails closed on reintroduction
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
PRE_MIGRATION_REF = os.environ.get("FLEET_AUTHORITY_BASE_REF", "origin/master")
MANIFEST_PATH = REPO_ROOT / "fleet" / "MIGRATION_MANIFEST.json"
README_PATH = REPO_ROOT / "fleet" / "README.md"
GUARD_SCRIPT = REPO_ROOT / "tools" / "fleet_authority_guard" / "check_no_fleet_authority.py"

ALLOWED_ACTIONS = frozenset({"migrate", "retire", "retain-non-authoritative"})

# Paths that must never remain as active fleet control-plane authority.
FORBIDDEN_PREFIXES = (
    "fleet/ansible.cfg",
    "fleet/inventory/",
    "fleet/group_vars/",
    "fleet/host_vars/",
    "fleet/roles/",
    "fleet/playbooks/",
    "fleet/nomad/",
)

# Alternate shapes the guard must also reject.
FORBIDDEN_BASENAME_PATTERNS = (
    re.compile(r"(^|/)ansible\.cfg$", re.I),
    re.compile(r"(^|/)hosts\.ya?ml$", re.I),
    re.compile(r"(^|/)site\.ya?ml$", re.I),
    re.compile(r"(^|/).*\\.nomad(\\.hcl)?$", re.I),
)

MINISPLIT_REQUIRED = (
    "services/minisplit-otel-poller/deploy/nomad/jobs/minisplit-otel-poller.nomad.hcl",
    "services/minisplit-otel-poller/main.go",
    "devices/della-minisplits/README.md",
)

POINTER_NEEDLES = (
    "MOVED",
    "aleksclark/fleet-iac",
    "platform/ansible",
    "jobs/",
)


def _git(*args: str, cwd: Path = REPO_ROOT) -> str:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    proc = subprocess.run(
        ["git", "-C", str(cwd), *args],
        capture_output=True,
        text=True,
        check=True,
        env=env,
    )
    return proc.stdout


def _pre_migration_fleet_paths() -> list[str]:
    out = _git("ls-tree", "-r", "--name-only", PRE_MIGRATION_REF, "--", "fleet/")
    paths = [p for p in out.splitlines() if p]
    if not paths:
        raise AssertionError(f"no fleet/** paths on {PRE_MIGRATION_REF}")
    return sorted(paths)


def _tracked_paths() -> set[str]:
    out = _git("ls-files", "-z")
    return {p for p in out.split("\0") if p}


def _load_manifest() -> dict:
    assert MANIFEST_PATH.is_file(), f"missing migration manifest: {MANIFEST_PATH}"
    data = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    assert isinstance(data, dict)
    return data


class TestMigrationManifestExhaustive(unittest.TestCase):
    def test_manifest_exists_and_schema(self):
        data = _load_manifest()
        self.assertEqual(data.get("schema_version"), 1)
        self.assertEqual(data.get("task"), "9")
        self.assertEqual(data.get("source_repo"), "aleksclark/home_assist")
        self.assertEqual(data.get("canonical_repo"), "aleksclark/fleet-iac")
        self.assertIn("source_commit", data)
        self.assertIn("canonical_commit", data)
        self.assertIn("entries", data)
        self.assertIsInstance(data["entries"], list)
        # value-free: no secret-shaped assignment values in JSON text
        raw = MANIFEST_PATH.read_text(encoding="utf-8")
        self.assertIsNone(
            re.search(
                r"(password|secret|token|api_key)\s*[:=]\s*['\"][^'\"]{8,}",
                raw,
                re.I,
            )
        )

    def test_classification_exhaustive_over_pre_migration_tree(self):
        pre = _pre_migration_fleet_paths()
        data = _load_manifest()
        entries = data["entries"]
        by_path = {}
        for e in entries:
            self.assertIn("path", e)
            self.assertIn("action", e)
            self.assertIn(e["action"], ALLOWED_ACTIONS)
            self.assertNotIn(e["path"], by_path, f"duplicate entry for {e['path']}")
            by_path[e["path"]] = e

        self.assertEqual(
            sorted(by_path),
            pre,
            "manifest must classify every tracked fleet/** path on pre-migration ref",
        )

        # Explicit class buckets must cover archiso + monitoring + README
        actions_by_prefix = {
            "fleet/archiso/": set(),
            "fleet/monitoring/": set(),
            "fleet/README.md": set(),
        }
        for path, e in by_path.items():
            for pref in actions_by_prefix:
                if path == pref.rstrip("/") or path.startswith(pref):
                    actions_by_prefix[pref].add(e["action"])
        for pref, acts in actions_by_prefix.items():
            self.assertTrue(acts, f"no classification for {pref}")

    def test_authority_paths_are_not_retain(self):
        data = _load_manifest()
        for e in data["entries"]:
            path = e["path"]
            if path == "fleet/ansible.cfg" or any(
                path.startswith(p) for p in FORBIDDEN_PREFIXES if p.endswith("/")
            ):
                self.assertIn(
                    e["action"],
                    {"migrate", "retire"},
                    f"active authority path must not be retain-non-authoritative: {path}",
                )


class TestRemovedAuthorityAbsent(unittest.TestCase):
    def test_forbidden_authority_paths_absent_from_worktree_and_index(self):
        tracked = _tracked_paths()
        offenders = []
        for p in sorted(tracked):
            if p in {
                "fleet/README.md",
                "fleet/MIGRATION_MANIFEST.json",
            } or p.startswith("fleet/MIGRATION_MANIFEST"):
                continue
            if p == "fleet/ansible.cfg" or any(
                p == pref.rstrip("/") or p.startswith(pref)
                for pref in FORBIDDEN_PREFIXES
            ):
                offenders.append(p)
            # alternate inventory/playbook shapes under fleet/
            if p.startswith("fleet/") and (
                p.endswith("ansible.cfg")
                or "/inventory/" in f"/{p}"
                or "/group_vars/" in f"/{p}"
                or "/host_vars/" in f"/{p}"
                or "/roles/" in f"/{p}"
                or "/playbooks/" in f"/{p}"
                or p.endswith(".nomad.hcl")
                or p.endswith(".nomad")
            ):
                if p not in offenders:
                    offenders.append(p)
        self.assertEqual(offenders, [], f"authority paths still present: {offenders}")

        # filesystem check (not only index)
        for pref in (
            "ansible.cfg",
            "inventory",
            "group_vars",
            "host_vars",
            "roles",
            "playbooks",
            "nomad",
        ):
            p = REPO_ROOT / "fleet" / pref
            self.assertFalse(p.exists(), f"path still on disk: {p}")


class TestPointerReadme(unittest.TestCase):
    def test_readme_is_moved_pointer(self):
        self.assertTrue(README_PATH.is_file())
        text = README_PATH.read_text(encoding="utf-8")
        for needle in POINTER_NEEDLES:
            self.assertIn(needle, text)
        # no executable fallback instructions
        banned = [
            r"ansible-playbook",
            r"nomad\s+job\s+run",
            r"cd\s+fleet\b",
            r"\./build\.sh",
            r"Quick Start",
        ]
        for pat in banned:
            self.assertIsNone(
                re.search(pat, text, re.I),
                f"README must not contain executable fallback ({pat})",
            )


class TestRetainedNonAuthoritative(unittest.TestCase):
    def test_retained_entries_cannot_execute_ansible_or_nomad_authority(self):
        data = _load_manifest()
        tracked = _tracked_paths()
        retained = [e for e in data["entries"] if e["action"] == "retain-non-authoritative"]
        for e in retained:
            path = e["path"]
            self.assertIn(path, tracked, f"retain entry missing from tree: {path}")
            # must not be ansible/nomad control-plane shapes
            self.assertFalse(path.endswith("ansible.cfg"))
            self.assertNotIn("/inventory/", f"/{path}")
            self.assertNotIn("/group_vars/", f"/{path}")
            self.assertNotIn("/host_vars/", f"/{path}")
            self.assertNotIn("/roles/", f"/{path}")
            self.assertNotIn("/playbooks/", f"/{path}")
            self.assertFalse(path.endswith(".nomad.hcl"))
            self.assertFalse(path.endswith(".nomad"))
            # retained file must not itself be an ansible playbook/inventory
            if path.endswith((".yml", ".yaml")):
                body = (REPO_ROOT / path).read_text(encoding="utf-8", errors="replace")
                self.assertNotRegex(body, r"(?m)^\s*-\s*hosts\s*:")
                self.assertNotRegex(body, r"(?m)^\s*roles\s*:")


class TestProjectSourcesRemain(unittest.TestCase):
    def test_minisplit_and_device_sources_remain(self):
        tracked = _tracked_paths()
        for p in MINISPLIT_REQUIRED:
            self.assertIn(p, tracked)
            self.assertTrue((REPO_ROOT / p).is_file())
        # HA application config stays
        self.assertTrue((REPO_ROOT / "home-assistant").is_dir())
        self.assertTrue((REPO_ROOT / "devices").is_dir())
        self.assertTrue((REPO_ROOT / "esphome").is_dir())


class TestGuardScript(unittest.TestCase):
    def test_guard_script_exists_and_passes_on_repo(self):
        self.assertTrue(GUARD_SCRIPT.is_file())
        proc = subprocess.run(
            [sys.executable, str(GUARD_SCRIPT), "--root", str(REPO_ROOT)],
            capture_output=True,
            text=True,
        )
        self.assertEqual(proc.returncode, 0, proc.stdout + proc.stderr)

    def test_guard_fails_closed_on_reintroduced_inventory(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # minimal allowed layout
            (root / "fleet").mkdir()
            (root / "fleet" / "README.md").write_text(
                "MOVED\naleksclark/fleet-iac\nplatform/ansible\njobs/\n",
                encoding="utf-8",
            )
            (root / "fleet" / "MIGRATION_MANIFEST.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "task": "9",
                        "entries": [],
                        "allowlist_path_prefixes": [
                            "fleet/README.md",
                            "fleet/MIGRATION_MANIFEST.json",
                        ],
                    }
                ),
                encoding="utf-8",
            )
            # seed git repo so guard can ls-files; also test filesystem walk mode
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "add", "fleet"], cwd=root, check=True, capture_output=True)

            ok = subprocess.run(
                [sys.executable, str(GUARD_SCRIPT), "--root", str(root)],
                capture_output=True,
                text=True,
            )
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)

            # reintroduce forbidden inventory
            inv = root / "fleet" / "inventory"
            inv.mkdir()
            (inv / "hosts.yml").write_text("all: {}\n", encoding="utf-8")
            subprocess.run(["git", "add", "fleet/inventory"], cwd=root, check=True, capture_output=True)
            bad = subprocess.run(
                [sys.executable, str(GUARD_SCRIPT), "--root", str(root)],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn("inventory", (bad.stdout + bad.stderr).lower())

    def test_guard_detects_alternate_ansible_cfg_and_nomad_shapes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "fleet").mkdir()
            (root / "fleet" / "README.md").write_text(
                "MOVED\naleksclark/fleet-iac\nplatform/ansible\njobs/\n", encoding="utf-8"
            )
            (root / "fleet" / "MIGRATION_MANIFEST.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "task": "9",
                        "entries": [],
                        "allowlist_path_prefixes": [
                            "fleet/README.md",
                            "fleet/MIGRATION_MANIFEST.json",
                        ],
                    }
                ),
                encoding="utf-8",
            )
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            # alternate names
            (root / "fleet" / "Ansible.cfg").write_text("[defaults]\n", encoding="utf-8")
            (root / "fleet" / "jobs").mkdir()
            (root / "fleet" / "jobs" / "traefik.nomad").write_text('job "t" {}\n', encoding="utf-8")
            subprocess.run(["git", "add", "fleet"], cwd=root, check=True, capture_output=True)
            bad = subprocess.run(
                [sys.executable, str(GUARD_SCRIPT), "--root", str(root)],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(bad.returncode, 0)
            combined = bad.stdout + bad.stderr
            self.assertTrue(
                "ansible.cfg" in combined.lower() or "nomad" in combined.lower(),
                combined,
            )


class TestWorkflowPresent(unittest.TestCase):
    def test_sha_pinned_workflow_exists(self):
        wf = REPO_ROOT / ".github" / "workflows" / "ci-fleet-authority-guard.yml"
        self.assertTrue(wf.is_file())
        text = wf.read_text(encoding="utf-8")
        self.assertIn("permissions:", text)
        self.assertIn("contents: read", text)
        # SHA-pinned actions (40 hex) rather than floating major tags alone
        self.assertRegex(
            text,
            r"actions/checkout@[0-9a-f]{40}",
        )
        self.assertRegex(
            text,
            r"actions/setup-python@[0-9a-f]{40}",
        )
        self.assertIn("check_no_fleet_authority.py", text)
        self.assertIn("test_fleet_authority_retirement.py", text)


if __name__ == "__main__":
    unittest.main()
