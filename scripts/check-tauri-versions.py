"""Fail if a Tauri crate and its npm package disagree on major.minor.

`tauri build` rejects these mismatches, so this catches them in CI instead
of in the release pipeline.
"""

import json
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO_LOCK = ROOT / "Cargo.lock"
PACKAGE_LOCK = ROOT / "app" / "package-lock.json"


def npm_name(crate: str) -> str | None:
    if crate == "tauri":
        return "@tauri-apps/api"
    if crate.startswith("tauri-plugin-"):
        return "@tauri-apps/plugin-" + crate.removeprefix("tauri-plugin-")
    return None


def major_minor(version: str) -> tuple[str, str]:
    major, minor, *_ = version.split(".")
    return major, minor


def main() -> int:
    cargo = tomllib.loads(CARGO_LOCK.read_text(encoding="utf-8"))
    npm = json.loads(PACKAGE_LOCK.read_text(encoding="utf-8"))["packages"]

    checked = 0
    mismatches = []
    for package in cargo["package"]:
        name = npm_name(package["name"])
        npm_entry = npm.get(f"node_modules/{name}") if name else None
        if npm_entry is None:
            continue
        checked += 1
        crate_version = package["version"]
        npm_version = npm_entry["version"]
        status = "ok"
        if major_minor(crate_version) != major_minor(npm_version):
            status = "MISMATCH"
            mismatches.append(package["name"])
        print(f"{status:8} {package['name']} {crate_version}  <->  {name} {npm_version}")

    if checked == 0:
        print("error: no Tauri crate/npm pairs found; check the lockfile paths")
        return 1
    if mismatches:
        print(
            f"error: {len(mismatches)} pair(s) differ in major.minor. "
            "Update the lagging side so both halves match, e.g. "
            "`cargo update -p <crate>` or `npm install <package>@<version>` in app/."
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
