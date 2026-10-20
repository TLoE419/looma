#!/usr/bin/env python3
"""Check the reviewed source inventory; optionally test patched crates in isolation."""

import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parent.parent


def verify():
    inventory = json.loads((ROOT / "vendor/manifest.json").read_text())
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    overrides = manifest["patch"]["crates-io"]
    assert set(overrides) == set(inventory["crates"]), "Unexpected source override"
    packages = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
    for name, crate in inventory["crates"].items():
        assert overrides[name] == {"path": f"vendor/{name}"}, name
        matches = [p for p in packages if p["name"] == name]
        assert len(matches) == 1 and matches[0]["version"] == crate["version"], name
        assert "source" not in matches[0], f"Override not active: {name}"
        folder = ROOT / "vendor" / name
        actual = {}
        for path in folder.rglob("*"):
            assert not path.is_symlink(), f"Symlink in vendored source: {path}"
            if path.is_file():
                actual[path.relative_to(folder).as_posix()] = hashlib.sha256(path.read_bytes()).hexdigest()
        assert actual == crate["files"], f"Unreviewed source change: {name}"
        patch = ROOT / "vendor/patches" / f"{name}.patch"
        assert hashlib.sha256(patch.read_bytes()).hexdigest() == crate["patch_sha256"], name
    forbidden = {"paste", "fxhash", "proc-macro-error", "proc-macro-error2"}
    assert not forbidden.intersection(p["name"] for p in packages), "Retired dependency returned"
    assert not any(p["name"] == "rand" and p["version"].startswith("0.7.") for p in packages)
    print("Dependency patch inventory and active overrides verified.", flush=True)


def verify_upstream():
    inventory = json.loads((ROOT / "vendor/manifest.json").read_text())
    for name, crate in inventory["crates"].items():
        request = urllib.request.Request(crate["registry_archive"], headers={"User-Agent": "Looma dependency patch verification"})
        with urllib.request.urlopen(request, timeout=60) as response:
            archive = response.read()
        assert hashlib.sha256(archive).hexdigest() == crate["registry_sha256"], f"Archive checksum: {name}"
        with tempfile.TemporaryDirectory(prefix="looma-patch-source-") as directory:
            folder = Path(directory)
            with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as source:
                members = {m.name: m for m in source.getmembers()}
                for relative in crate["files"]:
                    member = members.get(f"{name}-{crate['version']}/{relative}")
                    path = folder / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    if member:
                        assert member.isfile(), f"Nonregular source: {name}/{relative}"
                        path.write_bytes(source.extractfile(member).read())
                    else:
                        path.write_bytes(b"")
            subprocess.run(["git", "apply", str(ROOT / "vendor/patches" / f"{name}.patch")], cwd=folder, check=True)
            for relative, expected in crate["files"].items():
                assert hashlib.sha256((folder / relative).read_bytes()).hexdigest() == expected, f"Patch reconstruction: {name}/{relative}"
        print(f"Official archive plus recorded patch reproduces {name}.", flush=True)


def test_patches(update_lock):
    # A separate lockfile keeps upstream test-only dependencies out of the application.
    with tempfile.TemporaryDirectory(prefix="looma-patch-tests-") as directory:
        workspace = Path(directory)
        for name in ("glib", "glib-macros", "gtk3-macros"):
            shutil.copytree(ROOT / "vendor" / name, workspace / name)
        shutil.copyfile(ROOT / "tests/dependency-patches/Cargo.toml", workspace / "Cargo.toml")
        lock = ROOT / "tests/dependency-patches/Cargo.lock"
        if update_lock:
            subprocess.run(["cargo", "generate-lockfile"], cwd=workspace, check=True)
            shutil.copyfile(workspace / "Cargo.lock", lock)
            return
        shutil.copyfile(lock, workspace / "Cargo.lock")
        env = os.environ.copy()
        env.setdefault("CARGO_TARGET_DIR", str(ROOT / "target/dependency-patches"))
        commands = [
            ["cargo", "test", "--locked", "--release", "-p", "glib", "--lib"],
            ["cargo", "test", "--locked", "--release", "-p", "glib-macros", "--lib", "--tests"],
        ]
        commands.append(["cargo", "test", "--locked", "--release", "-p", "gtk3-macros", "--lib"])
        for command in commands:
            subprocess.run(command, cwd=workspace, env=env, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test", action="store_true", help="Run upstream and regression tests (GTK development libraries required)")
    parser.add_argument("--verify-upstream", action="store_true", help="Reconstruct each patch from checksum-verified official archives")
    parser.add_argument("--update-test-lock", action="store_true", help="Explicitly refresh the separate upstream-test lockfile")
    args = parser.parse_args()
    verify()
    if args.verify_upstream:
        verify_upstream()
    if args.test or args.update_test_lock:
        test_patches(args.update_test_lock)


if __name__ == "__main__":
    main()
