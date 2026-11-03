#!/usr/bin/env python3
"""Select Cargo app packages affected by a list of changed repository paths."""

from __future__ import annotations

import json
import sys
from pathlib import Path

ALL_PACKAGES = ("crm", "runner", "yasweb", "tasker", "crm_updater", "wxcc_handler")
SHARED_ROOT_FILES = {"Cargo.toml", "Cargo.lock", "deny.toml"}


def affected_packages(changed_paths: list[str]) -> list[str]:
    """Return affected packages in stable order; fail closed for unknown code paths."""
    packages: set[str] = set()
    shared = False

    for raw_path in changed_paths:
        path = raw_path.strip().replace("\\", "/")
        while path.startswith("./"):
            path = path[2:]
        if not path:
            continue

        parts = path.split("/")
        if path in SHARED_ROOT_FILES or path.startswith(
            ("src/", "tests/", ".github/")
        ):
            shared = True
        elif parts[0] == "apps":
            if len(parts) > 1 and parts[1] in ALL_PACKAGES:
                packages.add(parts[1])
            else:
                shared = True
        elif parts[0] != "md":
            # Unknown root-level code/config paths are shared until explicitly classified.
            shared = True

    if shared:
        return list(ALL_PACKAGES)
    return [package for package in ALL_PACKAGES if package in packages]


def main() -> int:
    if len(sys.argv) != 2:
        print("Usage: detect_affected_packages.py <changed-files.txt>", file=sys.stderr)
        return 2

    changed_paths = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
    print(json.dumps(affected_packages(changed_paths), separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
