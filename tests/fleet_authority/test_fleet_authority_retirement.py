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
MANIFEST_PATH = REPO_ROOT / "fleet" / "MIGRATION_MANIFEST.json"
README_PATH = REPO_ROOT / "fleet" / "README.md"
GUARD_SCRIPT = REPO_ROOT / "tools" / "fleet_authority_guard" / "check_no_fleet_authority.py"
MERGE_ORDER_SCRIPT = (
    REPO_ROOT / "tools" / "fleet_authority_guard" / "check_fleet_iac_merge_order.py"
)

# Expected provenance pins (stable across post-merge HEAD movement).
EXPECTED_SOURCE_COMMIT = "8d23d803377d9b0434b4543825be5ae57a65253b"
EXPECTED_CANONICAL_COMMIT = "234115bfb1afbf01838656bb48dc27c2a008acd8"

ALLOWED_ACTIONS = frozenset({"migrate", "retire", "retain-non-authoritative"})


def _manifest_source_commit() -> str:
    data = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    commit = data.get("source_commit")
    if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise AssertionError(f"manifest source_commit missing/invalid: {commit!r}")
    return commit


def _pre_migration_ref() -> str:
    """Git object used for pre-migration fleet/** provenance.

    Default is the pinned manifest source_commit (not floating origin/master).
    FLEET_AUTHORITY_BASE_REF may override for tooling, but provenance tests that
    need the pre-retirement tree always resolve via source_commit when the
    override points at a post-retirement tip (HEAD / origin/master after merge).
    """
    override = os.environ.get("FLEET_AUTHORITY_BASE_REF")
    if override and override not in {"HEAD", "origin/master", "master"}:
        return override
    return _manifest_source_commit()


# Back-compat name used throughout this module.
PRE_MIGRATION_REF = None  # resolved lazily via _resolve_pre_migration_ref()


def _resolve_pre_migration_ref() -> str:
    global PRE_MIGRATION_REF
    if PRE_MIGRATION_REF is None:
        PRE_MIGRATION_REF = _pre_migration_ref()
    return PRE_MIGRATION_REF

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
    ref = _resolve_pre_migration_ref()
    out = _git("ls-tree", "-r", "--name-only", ref, "--", "fleet/")
    paths = [p for p in out.splitlines() if p]
    if not paths:
        raise AssertionError(f"no fleet/** paths on {ref}")
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
        self.assertEqual(data.get("source_commit"), EXPECTED_SOURCE_COMMIT)
        self.assertEqual(data.get("canonical_commit"), EXPECTED_CANONICAL_COMMIT)
        self.assertIn("entries", data)
        self.assertIsInstance(data["entries"], list)
        # merge-order sequencing: HA PR merge-ready only after canonical on fleet-iac mainline
        seq = data.get("merge_sequencing") or data.get("release_prerequisites")
        self.assertIsInstance(seq, dict, "manifest must declare merge_sequencing")
        self.assertEqual(seq.get("canonical_commit"), EXPECTED_CANONICAL_COMMIT)
        self.assertIn(seq.get("canonical_mainline_status"), {"pending", "merged"})
        self.assertTrue(seq.get("home_assist_merge_blocked_until_canonical_on_mainline"))
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
        # Provenance must pin/fetch manifest source_commit fail-hard (no || true).
        self.assertIn("source_commit", text)
        self.assertNotRegex(text, r"git\s+fetch[^\n]*\|\|\s*true")
        self.assertNotIn("FLEET_AUTHORITY_BASE_REF: origin/master", text)
        # Merge-order gate is a hard fleet-first release/merge prerequisite.
        self.assertIn("check_fleet_iac_merge_order.py", text)
        self.assertRegex(
            text,
            re.compile(
                r"merge.order|merge_order|merge-ready|release.?prerequisite|merge.?prerequisite",
                re.I,
            ),
        )

    def test_workflow_materializes_private_fleet_iac_with_read_token(self):
        """Hosted runners cannot read private fleet-iac via GITHUB_TOKEN.

        Require a second SHA-pinned checkout of aleksclark/fleet-iac into
        fleet-iac-canonical using secrets.FLEET_IAC_READ_TOKEN, fetch-depth 0,
        with explicit preflight failure when the secret is missing.
        """
        wf = REPO_ROOT / ".github" / "workflows" / "ci-fleet-authority-guard.yml"
        text = wf.read_text(encoding="utf-8")
        checkouts = re.findall(
            r"uses:\s*actions/checkout@([0-9a-f]{40})",
            text,
        )
        self.assertGreaterEqual(
            len(checkouts),
            2,
            "workflow must SHA-pin a second actions/checkout for private fleet-iac",
        )
        self.assertTrue(
            all(re.fullmatch(r"[0-9a-f]{40}", c) for c in checkouts),
            checkouts,
        )
        self.assertIn("aleksclark/fleet-iac", text)
        self.assertIn("fleet-iac-canonical", text)
        self.assertIn("secrets.FLEET_IAC_READ_TOKEN", text)
        self.assertRegex(
            text,
            re.compile(
                r"repository:\s*aleksclark/fleet-iac",
            ),
        )
        self.assertRegex(
            text,
            re.compile(
                r"path:\s*fleet-iac-canonical",
            ),
        )
        # Second checkout must request full history for merge-base ancestry.
        # Prefer the checkout step that sets repository: aleksclark/fleet-iac.
        repo_m = re.search(
            r"repository:\s*aleksclark/fleet-iac",
            text,
        )
        self.assertIsNotNone(repo_m, "missing repository: aleksclark/fleet-iac checkout")
        assert repo_m is not None  # for type checkers
        idx = repo_m.start()
        fi_block = text[max(0, idx - 500) : idx + 500]
        self.assertRegex(
            fi_block,
            re.compile(r"fetch-depth:\s*0"),
            "fleet-iac checkout must use fetch-depth: 0",
        )
        self.assertRegex(
            fi_block,
            re.compile(
                r"token:\s*\$\{\{\s*secrets\.FLEET_IAC_READ_TOKEN\s*\}\}"
            ),
        )
        self.assertRegex(
            fi_block,
            re.compile(r"path:\s*fleet-iac-canonical"),
        )
        # Fail-closed preflight when the read credential is unavailable.
        self.assertTrue(
            "FLEET_IAC_READ_TOKEN" in text
            and (
                "missing" in text.lower()
                or "required" in text.lower()
                or "unavailable" in text.lower()
                or "not set" in text.lower()
                or "empty" in text.lower()
            ),
            "workflow must preflight-fail clearly when FLEET_IAC_READ_TOKEN is unavailable",
        )
        # Never echo/print the token value (name in error text is OK).
        self.assertNotRegex(
            text,
            re.compile(
                r"(echo|printf|cat)\s+[\"']?\$\{?FLEET_IAC_READ_TOKEN",
                re.I,
            ),
        )
        self.assertNotRegex(
            text,
            re.compile(
                r"(echo|printf)\s+.*\$\{\{\s*secrets\.FLEET_IAC_READ_TOKEN",
                re.I,
            ),
        )
        # No repository-variable path assumption for the private checkout.
        self.assertNotIn("vars.FLEET_IAC_REPO", text)
        self.assertNotRegex(text, r"FLEET_IAC_REPO:\s*\$\{\{\s*vars\.")



FLEET_IAC_PATHS_FIXTURE = (
    Path(__file__).resolve().parent / "fixtures" / "fleet_iac_234115b_paths.txt"
)

# Stock archiso enablement symlinks intentionally retired (no unique fleet-iac source).
RETIRED_ARCHISO_ENABLEMENT_SYMLINKS = (
    "fleet/archiso/airootfs/etc/systemd/system-generators/systemd-gpt-auto-generator",
    "fleet/archiso/airootfs/etc/systemd/system/dbus-org.freedesktop.ModemManager1.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/ModemManager.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/hv_fcopy_daemon.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/hv_kvp_daemon.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/hv_vss_daemon.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/iwd.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/livecd-talk.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/vboxservice.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/vmtoolsd.service",
    "fleet/archiso/airootfs/etc/systemd/system/multi-user.target.wants/vmware-vmblock-fuse.service",
)

HARD_FLEET_ALLOWLIST = frozenset(
    {
        "fleet/README.md",
        "fleet/MIGRATION_MANIFEST.json",
    }
)


def _seed_pointer_repo(root: Path) -> None:
    (root / "fleet").mkdir(parents=True, exist_ok=True)
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
                "allowlist_paths": sorted(HARD_FLEET_ALLOWLIST),
            }
        ),
        encoding="utf-8",
    )
    subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
    subprocess.run(
        ["git", "config", "user.email", "test@example.com"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    subprocess.run(
        ["git", "config", "user.name", "test"],
        cwd=root,
        check=True,
        capture_output=True,
    )
    subprocess.run(["git", "add", "fleet"], cwd=root, check=True, capture_output=True)


def _run_guard(root: Path, *extra: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(GUARD_SCRIPT), "--root", str(root), *extra],
        capture_output=True,
        text=True,
    )


def _load_fleet_iac_paths() -> set[str]:
    assert FLEET_IAC_PATHS_FIXTURE.is_file(), f"missing fixture {FLEET_IAC_PATHS_FIXTURE}"
    return {
        ln.strip()
        for ln in FLEET_IAC_PATHS_FIXTURE.read_text(encoding="utf-8").splitlines()
        if ln.strip()
    }


class TestGuardExactAllowlist(unittest.TestCase):
    """Only fleet/README.md and fleet/MIGRATION_MANIFEST.json may exist under fleet/."""

    def test_guard_rejects_case_alternate_top_level_authority_names(self):
        cases = [
            ("Roles/x/tasks/main.yml", "roles"),
            ("Group_vars/all.yml", "group_vars"),
            ("Host_vars/node.yml", "host_vars"),
            ("Inventory/hosts.yml", "inventory"),
            ("Nomad/job.nomad.hcl", "nomad"),
            ("jobs/traefik.nomad", "jobs"),
            ("playbooks/site.yml", "playbooks"),
            ("ansible.cfg", "ansible"),
            ("Ansible.cfg", "ansible"),
            ("ANSIBLE.CFG", "ansible"),
        ]
        for rel, needle in cases:
            with self.subTest(rel=rel):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    _seed_pointer_repo(root)
                    target = root / "fleet" / rel
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_text("x\n", encoding="utf-8")
                    subprocess.run(
                        ["git", "add", "-A"], cwd=root, check=True, capture_output=True
                    )
                    bad = _run_guard(root)
                    self.assertNotEqual(
                        bad.returncode, 0, rel + " " + bad.stdout + bad.stderr
                    )
                    combined = (bad.stdout + bad.stderr).lower()
                    self.assertTrue(
                        needle in combined
                        or rel.lower().split("/")[0] in combined
                        or "not allowlisted" in combined
                        or "fleet/" in combined,
                        combined,
                    )

    def test_guard_rejects_extensionless_hcl_mixed_case_and_nested(self):
        samples = [
            "fleet/nomad/infra/job",  # extensionless
            "fleet/NoMaD/Traefik.HCL",
            "fleet/roles/nested/deep/tasks/main.yml",
            "fleet/something_else.txt",
            "fleet/archiso/build.sh",
            "fleet/monitoring/rules.yml",
        ]
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            for rel in samples:
                p = root / rel
                p.parent.mkdir(parents=True, exist_ok=True)
                p.write_text("x\n", encoding="utf-8")
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )
            bad = _run_guard(root)
            self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)
            combined = bad.stdout + bad.stderr
            for rel in samples:
                self.assertIn(rel, combined)

    def test_guard_rejects_symlink_and_untracked_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            target = root / "outside_secret.yml"
            target.write_text("secret: x\n", encoding="utf-8")
            link = root / "fleet" / "roles_link"
            link.symlink_to(target)
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )
            bad = _run_guard(root)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn("roles_link", bad.stdout + bad.stderr)

            link.unlink()
            subprocess.run(
                ["git", "rm", "-f", "fleet/roles_link"],
                cwd=root,
                check=True,
                capture_output=True,
            )
            untracked = root / "fleet" / "Inventory" / "hosts.yml"
            untracked.parent.mkdir(parents=True, exist_ok=True)
            untracked.write_text("all: {}\n", encoding="utf-8")
            bad2 = _run_guard(root)
            self.assertNotEqual(bad2.returncode, 0, bad2.stdout + bad2.stderr)
            self.assertIn("Inventory", bad2.stdout + bad2.stderr)

    def test_guard_rejects_path_traversal_and_casefolded_allowlist_bypass(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            (root / "fleet" / "readme.md").write_text("nope\n", encoding="utf-8")
            (root / "fleet" / "Migration_Manifest.json").write_text("{}\n", encoding="utf-8")
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )
            bad = _run_guard(root)
            self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)
            combined = bad.stdout + bad.stderr
            self.assertIn("readme.md", combined)
            self.assertIn("Migration_Manifest.json", combined)

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            evil_dir = root / "fleet" / "nested"
            evil_dir.mkdir()
            (evil_dir / "x.yml").write_text("x\n", encoding="utf-8")
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )
            bad = _run_guard(root)
            self.assertNotEqual(bad.returncode, 0)
            self.assertIn("nested", bad.stdout + bad.stderr)

    def test_guard_allows_only_exact_hard_allowlist(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            ok = _run_guard(root)
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)


class TestProvenanceContracts(unittest.TestCase):
    def test_retired_archiso_enablement_symlinks_reclassified(self):
        data = _load_manifest()
        by_path = {e["path"]: e for e in data["entries"]}
        for path in RETIRED_ARCHISO_ENABLEMENT_SYMLINKS:
            self.assertIn(path, by_path)
            e = by_path[path]
            self.assertEqual(e["action"], "retire", path)
            self.assertIsNone(e.get("canonical"), path)
            rel = (e.get("relation") or "").lower()
            notes = (e.get("notes") or "").lower()
            self.assertTrue(
                "stock" in rel or "enablement" in rel or "symlink" in rel,
                f"relation should mark stock enablement symlink: {e.get('relation')}",
            )
            self.assertTrue(
                ("stock" in notes or "enablement" in notes)
                and (
                    "retired" in notes
                    or "no unique" in notes
                    or "intentionally" in notes
                ),
                f"notes must explain intentional retirement: {e.get('notes')}",
            )

    def test_action_counts_match_entries_after_archiso_reclass(self):
        data = _load_manifest()
        from collections import Counter

        c = Counter(e["action"] for e in data["entries"])
        derived_total = len(data["entries"])
        # Census is derived from entries/actions — no drift vs hardcoded constants.
        self.assertEqual(data["entry_count"], derived_total)
        self.assertEqual(len(data["entries"]), data["entry_count"])
        self.assertEqual(data["action_counts"].get("retire"), c.get("retire"))
        self.assertEqual(data["action_counts"].get("migrate"), c.get("migrate"))
        self.assertEqual(
            data["action_counts"].get("retain-non-authoritative", 0),
            c.get("retain-non-authoritative", 0),
        )
        self.assertEqual(sum(data["action_counts"].values()), derived_total)
        # Still exhaustive over pre-migration source tree
        pre = _pre_migration_fleet_paths()
        self.assertEqual(derived_total, len(pre))
        self.assertEqual(sorted(e["path"] for e in data["entries"]), pre)
        # Preserve expected source/canonical pins
        self.assertEqual(data["source_commit"], EXPECTED_SOURCE_COMMIT)
        self.assertEqual(data["canonical_commit"], EXPECTED_CANONICAL_COMMIT)
        # Sanity: after archiso reclass we still have both migrate and retire
        self.assertGreater(c.get("migrate", 0), 0)
        self.assertGreater(c.get("retire", 0), 0)
        self.assertEqual(c.get("retain-non-authoritative", 0), 0)

    def test_migrate_canonical_paths_exist_in_fleet_iac_fixture(self):
        data = _load_manifest()
        self.assertEqual(data.get("canonical_commit"), EXPECTED_CANONICAL_COMMIT)
        fleet_paths = _load_fleet_iac_paths()
        missing = []
        for e in data["entries"]:
            if e["action"] != "migrate":
                continue
            canon = e.get("canonical")
            if not isinstance(canon, str) or not canon:
                missing.append((e["path"], canon, "empty-canonical"))
                continue
            if canon not in fleet_paths:
                missing.append((e["path"], canon, "not-in-tree"))
        self.assertEqual(
            missing,
            [],
            f"migrate canonical missing from fleet-iac@{EXPECTED_CANONICAL_COMMIT[:7]}: {missing}",
        )

    def test_retire_entries_have_null_canonical_and_reason(self):
        data = _load_manifest()
        bad = []
        for e in data["entries"]:
            if e["action"] != "retire":
                continue
            if e.get("canonical") is not None:
                bad.append((e["path"], "canonical-not-null", e.get("canonical")))
            relation = e.get("relation")
            notes = e.get("notes") or ""
            if not relation:
                bad.append((e["path"], "missing-relation", None))
            if not notes:
                bad.append((e["path"], "missing-notes", None))
        self.assertEqual(bad, [], f"retire provenance contract violations: {bad}")

    def test_source_sha256_exhaustive_and_stable(self):
        data = _load_manifest()
        pre = _pre_migration_fleet_paths()
        by_path = {e["path"]: e for e in data["entries"]}
        self.assertEqual(sorted(by_path), pre)
        self.assertEqual(data["source_commit"], EXPECTED_SOURCE_COMMIT)
        ref = _resolve_pre_migration_ref()
        # Even if env override is HEAD/post-retirement tip, provenance uses source_commit.
        self.assertEqual(ref, EXPECTED_SOURCE_COMMIT)
        sha_re = re.compile(r"^[0-9a-f]{64}$")
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        import hashlib

        for path in pre:
            e = by_path[path]
            self.assertRegex(e.get("source_sha256", ""), sha_re, path)
            proc = subprocess.run(
                [
                    "git",
                    "-C",
                    str(REPO_ROOT),
                    "cat-file",
                    "-p",
                    f"{ref}:{path}",
                ],
                capture_output=True,
                check=True,
                env=env,
            )
            digest = hashlib.sha256(proc.stdout).hexdigest()
            self.assertEqual(digest, e["source_sha256"], path)

    def test_provenance_defaults_to_manifest_source_commit_not_floating_tip(self):
        """Post-merge simulation: BASE_REF=HEAD still validates against source_commit."""
        self.assertEqual(_manifest_source_commit(), EXPECTED_SOURCE_COMMIT)
        # Clear any cached ref
        global PRE_MIGRATION_REF
        old = os.environ.get("FLEET_AUTHORITY_BASE_REF")
        try:
            for tip in ("HEAD", "origin/master", "master", None):
                PRE_MIGRATION_REF = None
                if tip is None:
                    os.environ.pop("FLEET_AUTHORITY_BASE_REF", None)
                else:
                    os.environ["FLEET_AUTHORITY_BASE_REF"] = tip
                ref = _pre_migration_ref()
                self.assertEqual(
                    ref,
                    EXPECTED_SOURCE_COMMIT,
                    f"tip override {tip!r} must still resolve to source_commit",
                )
                paths = _pre_migration_fleet_paths()
                self.assertGreater(len(paths), 2)
                self.assertIn("fleet/ansible.cfg", paths)
            # Explicit post-retirement tip: current HEAD allowlist-only tree
            # must differ from pinned source_commit pre-migration tree.
            PRE_MIGRATION_REF = None
            os.environ["FLEET_AUTHORITY_BASE_REF"] = "HEAD"
            source_paths = _pre_migration_fleet_paths()
            head_paths = [
                p
                for p in _git(
                    "ls-tree", "-r", "--name-only", "HEAD", "--", "fleet/"
                ).splitlines()
                if p
            ]
            self.assertEqual(
                sorted(head_paths),
                ["fleet/MIGRATION_MANIFEST.json", "fleet/README.md"],
            )
            self.assertNotEqual(sorted(source_paths), sorted(head_paths))
            self.assertIn("fleet/ansible.cfg", source_paths)
        finally:
            PRE_MIGRATION_REF = None
            if old is None:
                os.environ.pop("FLEET_AUTHORITY_BASE_REF", None)
            else:
                os.environ["FLEET_AUTHORITY_BASE_REF"] = old


class TestGuardCasefoldTopLevel(unittest.TestCase):
    """Reject casefold-equivalent top-level fleet trees (FLEET/, Fleet/)."""

    def test_guard_rejects_casefold_top_level_fleet_trees_and_contents(self):
        variants = ["FLEET", "Fleet", "FlEeT"]
        for top in variants:
            with self.subTest(top=top):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    _seed_pointer_repo(root)
                    # tracked nested authority under casefold top-level
                    tracked = root / top / "roles" / "x.yml"
                    tracked.parent.mkdir(parents=True, exist_ok=True)
                    tracked.write_text("x\n", encoding="utf-8")
                    subprocess.run(
                        ["git", "add", "-A"], cwd=root, check=True, capture_output=True
                    )
                    # untracked file under same tree
                    untracked = root / top / "inventory" / "hosts.yml"
                    untracked.parent.mkdir(parents=True, exist_ok=True)
                    untracked.write_text("all: {}\n", encoding="utf-8")
                    # symlink under casefold tree
                    target = root / "outside.yml"
                    target.write_text("secret: 1\n", encoding="utf-8")
                    link = root / top / "playbooks_link"
                    link.symlink_to(target)
                    # empty dir under casefold tree
                    empty = root / top / "group_vars"
                    empty.mkdir(parents=True, exist_ok=True)

                    bad = _run_guard(root)
                    self.assertNotEqual(
                        bad.returncode, 0, top + " " + bad.stdout + bad.stderr
                    )
                    combined = bad.stdout + bad.stderr
                    self.assertTrue(
                        top in combined or top.lower() in combined.lower(),
                        combined,
                    )
                    # contents should surface too
                    self.assertTrue(
                        "roles" in combined
                        or "inventory" in combined
                        or "playbooks_link" in combined
                        or "group_vars" in combined
                        or "not allowlisted" in combined.lower()
                        or "casefold" in combined.lower(),
                        combined,
                    )

    def test_guard_rejects_mixed_casefold_with_lowercase_fleet_present(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            (root / "FLEET" / "nomad").mkdir(parents=True)
            (root / "FLEET" / "nomad" / "job.hcl").write_text("job {}\n", encoding="utf-8")
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )
            bad = _run_guard(root)
            self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)
            self.assertIn("FLEET", bad.stdout + bad.stderr)


class TestGuardAllowlistRegularFiles(unittest.TestCase):
    def test_guard_rejects_symlinked_allowlist_readme_and_manifest(self):
        for name in ("README.md", "MIGRATION_MANIFEST.json"):
            with self.subTest(name=name):
                with tempfile.TemporaryDirectory() as tmp:
                    root = Path(tmp)
                    _seed_pointer_repo(root)
                    target = root / f"outside_{name}"
                    if name.endswith(".json"):
                        target.write_text(
                            json.dumps(
                                {
                                    "schema_version": 1,
                                    "task": "9",
                                    "entries": [],
                                    "allowlist_paths": sorted(HARD_FLEET_ALLOWLIST),
                                }
                            ),
                            encoding="utf-8",
                        )
                    else:
                        target.write_text(
                            "MOVED\naleksclark/fleet-iac\nplatform/ansible\njobs/\n",
                            encoding="utf-8",
                        )
                    path = root / "fleet" / name
                    path.unlink()
                    path.symlink_to(target)
                    subprocess.run(
                        ["git", "add", "-A"], cwd=root, check=True, capture_output=True
                    )
                    bad = _run_guard(root)
                    self.assertNotEqual(
                        bad.returncode, 0, name + " " + bad.stdout + bad.stderr
                    )
                    combined = (bad.stdout + bad.stderr).lower()
                    self.assertTrue(
                        "symlink" in combined
                        or "regular file" in combined
                        or name.lower() in combined,
                        combined,
                    )

    def test_guard_requires_regular_files_at_exact_allowlist_paths(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _seed_pointer_repo(root)
            # replace README with a directory
            readme = root / "fleet" / "README.md"
            readme.unlink()
            readme.mkdir()
            (readme / "nested.txt").write_text("x\n", encoding="utf-8")
            subprocess.run(
                ["git", "add", "-A"], cwd=root, check=True, capture_output=True
            )
            bad = _run_guard(root)
            self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)


def _write_merge_order_manifest(root: Path, canonical: str, *, mainline_ref: str = "origin/master", status: str = "pending") -> None:
    (root / "fleet").mkdir(parents=True, exist_ok=True)
    manifest = {
        "schema_version": 1,
        "task": "9",
        "source_commit": EXPECTED_SOURCE_COMMIT,
        "canonical_repo": "aleksclark/fleet-iac",
        "canonical_commit": canonical,
        "merge_sequencing": {
            "canonical_commit": canonical,
            "canonical_mainline_ref": mainline_ref,
            "canonical_mainline_status": status,
            "home_assist_merge_blocked_until_canonical_on_mainline": True,
            "note": "Do not merge home_assist Task 9 until canonical is on fleet-iac mainline.",
        },
        "entries": [],
    }
    (root / "fleet" / "MIGRATION_MANIFEST.json").write_text(
        json.dumps(manifest), encoding="utf-8"
    )


def _init_git_repo(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    subprocess.run(["git", "init"], cwd=path, check=True, capture_output=True)
    subprocess.run(
        ["git", "config", "user.email", "t@example.com"],
        cwd=path,
        check=True,
        capture_output=True,
    )
    subprocess.run(
        ["git", "config", "user.name", "t"],
        cwd=path,
        check=True,
        capture_output=True,
    )


def _commit_file(repo: Path, rel: str, content: str, message: str) -> str:
    target = repo / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")
    subprocess.run(["git", "add", "."], cwd=repo, check=True, capture_output=True)
    subprocess.run(
        ["git", "commit", "-m", message],
        cwd=repo,
        check=True,
        capture_output=True,
    )
    return subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def _run_merge_order(
    root: Path,
    fleet_iac: Path,
    *extra: str,
    mainline_ref: str = "origin/master",
) -> subprocess.CompletedProcess[str]:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    # Prefer explicit --fleet-iac; do not rely on FLEET_IAC_REPO path assumptions.
    env.pop("FLEET_IAC_REPO", None)
    cmd = [
        sys.executable,
        str(MERGE_ORDER_SCRIPT),
        "--root",
        str(root),
        "--fleet-iac",
        str(fleet_iac),
        "--mainline-ref",
        mainline_ref,
        *extra,
    ]
    return subprocess.run(cmd, capture_output=True, text=True, env=env)


class TestMergeOrderSequencingGate(unittest.TestCase):
    """Fail-closed fleet-first merge-order gate against a private fleet-iac checkout.

    home_assist is public; fleet-iac is private. Hosted CI must materialize
    fleet-iac-canonical with FLEET_IAC_READ_TOKEN and hard-fail PR + master until
    the pinned canonical commit is an ancestor of fleet-iac origin/master.
    """

    def test_merge_order_script_exists_and_documents_gate(self):
        self.assertTrue(MERGE_ORDER_SCRIPT.is_file())
        text = MERGE_ORDER_SCRIPT.read_text(encoding="utf-8")
        self.assertIn("canonical_commit", text)
        self.assertIn("mainline", text.lower())
        self.assertIn("merge", text.lower())
        self.assertIn("merge-base", text)
        self.assertIn("--is-ancestor", text)

    def test_readme_documents_merge_order_prerequisite(self):
        text = README_PATH.read_text(encoding="utf-8")
        self.assertRegex(
            text,
            re.compile(
                r"merge.?order|merge-ready|canonical.*mainline|fleet-iac.*main",
                re.I,
            ),
        )
        self.assertIn(EXPECTED_CANONICAL_COMMIT[:7], text)
        self.assertTrue(
            "prerequisite" in text.lower()
            or "merge-ready" in text.lower()
            or "must land" in text.lower()
            or "before merging" in text.lower()
            or "mainline" in text.lower()
        )

    def test_agents_md_documents_merge_order_prerequisite_and_read_token(self):
        agents = (REPO_ROOT / "AGENTS.md").read_text(encoding="utf-8")
        lowered = agents.lower()
        self.assertTrue(
            "merge-order" in lowered
            or "merge order" in lowered
            or "fleet-first" in lowered,
            "AGENTS.md must document the fleet-first merge-order prerequisite",
        )
        self.assertIn("FLEET_IAC_READ_TOKEN", agents)
        self.assertIn("fleet-iac-canonical", agents)
        # Document credential setup without embedding secret values or 1Password.
        self.assertNotRegex(agents, re.compile(r"1password|op://", re.I))
        self.assertTrue(
            "fine-grained" in lowered
            or "read-only" in lowered
            or "deploy" in lowered
            or "contents: read" in lowered
            or "repository secret" in lowered,
            "AGENTS.md must document required read-only token/deploy credential setup",
        )

    def test_merge_order_gate_fails_closed_when_canonical_object_missing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fi = root / "fleet-iac-canonical"
            _init_git_repo(fi)
            _commit_file(fi, "README.md", "fi\n", "init")
            subprocess.run(
                ["git", "branch", "-M", "master"], cwd=fi, check=True, capture_output=True
            )
            # Simulate a fetched origin/master without the canonical object.
            subprocess.run(
                ["git", "update-ref", "refs/remotes/origin/master", "HEAD"],
                cwd=fi,
                check=True,
                capture_output=True,
            )
            _write_merge_order_manifest(root, EXPECTED_CANONICAL_COMMIT)
            bad = _run_merge_order(root, fi, mainline_ref="origin/master")
            self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)
            combined = (bad.stdout + bad.stderr).lower()
            self.assertTrue(
                "not present" in combined
                or "missing" in combined
                or "cat-file" in combined
                or EXPECTED_CANONICAL_COMMIT[:7] in combined
                or "fail-closed" in combined,
                combined,
            )

    def test_merge_order_gate_fails_closed_when_canonical_not_ancestor(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fi = root / "fleet-iac-canonical"
            _init_git_repo(fi)
            _commit_file(fi, "README.md", "fi\n", "init")
            # Canonical commit exists on a side branch, not on origin/master.
            subprocess.run(
                ["git", "checkout", "-b", "feature"],
                cwd=fi,
                check=True,
                capture_output=True,
            )
            canon = _commit_file(fi, "canon.txt", "c\n", "canonical")
            subprocess.run(
                ["git", "checkout", "master"],
                cwd=fi,
                check=True,
                capture_output=True,
            )
            # Divergent mainline tip (does not contain canonical).
            _commit_file(fi, "mainline.txt", "m\n", "mainline-only")
            subprocess.run(
                ["git", "branch", "-M", "master"], cwd=fi, check=True, capture_output=True
            )
            subprocess.run(
                ["git", "update-ref", "refs/remotes/origin/master", "HEAD"],
                cwd=fi,
                check=True,
                capture_output=True,
            )
            _write_merge_order_manifest(root, canon, status="pending")
            bad = _run_merge_order(root, fi, mainline_ref="origin/master")
            self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)
            combined = (bad.stdout + bad.stderr).lower()
            self.assertTrue(
                "not an ancestor" in combined
                or "merge-order" in combined
                or "mainline" in combined
                or "gate" in combined,
                combined,
            )

    def test_merge_order_gate_passes_when_canonical_is_ancestor_of_origin_master(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fi = root / "fleet-iac-canonical"
            _init_git_repo(fi)
            _commit_file(fi, "README.md", "fi\n", "init")
            canon = _commit_file(fi, "canon.txt", "c\n", "canonical")
            # Later mainline commit still has canonical as ancestor.
            _commit_file(fi, "later.txt", "l\n", "later")
            subprocess.run(
                ["git", "branch", "-M", "master"], cwd=fi, check=True, capture_output=True
            )
            subprocess.run(
                ["git", "update-ref", "refs/remotes/origin/master", "HEAD"],
                cwd=fi,
                check=True,
                capture_output=True,
            )
            _write_merge_order_manifest(root, canon, status="merged")
            ok = _run_merge_order(root, fi, mainline_ref="origin/master")
            self.assertEqual(ok.returncode, 0, ok.stdout + ok.stderr)
            self.assertIn("ok", (ok.stdout + ok.stderr).lower())

    def test_workflow_hard_fails_merge_order_on_pr_and_master(self):
        """PR sequencing and master pushes both hard-fail until canonical is on mainline."""
        wf = (REPO_ROOT / ".github" / "workflows" / "ci-fleet-authority-guard.yml").read_text(
            encoding="utf-8"
        )
        self.assertIn("check_fleet_iac_merge_order.py", wf)
        self.assertIn("fleet-iac-canonical", wf)
        self.assertIn("--fleet-iac", wf)
        self.assertIn("origin/master", wf)
        # No soft/advisory path: remove continue-on-error and --soft/--report-only.
        self.assertNotIn("continue-on-error", wf)
        self.assertNotIn("--soft", wf)
        self.assertNotIn("--report-only", wf)
        # Must invoke against the materialized private checkout path.
        self.assertRegex(
            wf,
            re.compile(
                r"check_fleet_iac_merge_order\.py[\s\S]{0,400}fleet-iac-canonical"
            ),
        )
        self.assertRegex(
            wf,
            re.compile(r"--mainline-ref\s+origin/master"),
        )
        # Must not depend on repository variable path assumptions.
        self.assertNotIn("vars.FLEET_IAC_REPO", wf)


if __name__ == "__main__":
    unittest.main()
