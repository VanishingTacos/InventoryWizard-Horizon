#!/usr/bin/env python3
"""Packages the mod as target/<id>-<version>.hnmod for the Horizon mod registry.

    python3 package.py

The same deterministic file as horizon-mods' `native/tools/native.py package`: a zip of horizon-native.json and the
bundle's .rs and .toml files, with fixed dates, so the same source always gives the same SHA-256. Needs no rsync or
WSL, so it runs on Windows too.
"""
import hashlib
import json
import sys
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def main():
    compat = json.loads((ROOT / "compatibility.json").read_text())
    bundle = next(p for p in (ROOT / "mixins").iterdir() if (p / "horizon-mixin.toml").is_file())
    manifest = tomllib.loads((bundle / "horizon-mixin.toml").read_text())
    mod_id, version = manifest["mod"]["id"], manifest["mod"]["version"]
    pinned = manifest.get("compatibility", {}).get("horizon_revision")
    if pinned != compat["horizon_revision"]:
        sys.exit(f"{mod_id}: horizon-mixin.toml pins {pinned}, compatibility.json {compat['horizon_revision']}")
    native = {"schema": 1, "id": mod_id, "version": version, "game": compat["game"],
              "horizon": compat["horizon_revision"], "mixins": compat["horizon_mixins_revision"],
              "hooks": compat["hooks"]}
    files = {"horizon-native.json": (json.dumps(native, indent=2, sort_keys=True) + "\n").encode()}
    for path in sorted(bundle.iterdir()):
        if path.suffix in (".rs", ".toml"):
            # Line endings as committed, whatever the checkout's: the hash must not depend on the OS
            files[f"mixins/{mod_id}/{path.name}"] = path.read_bytes().replace(b"\r\n", b"\n")
    out = ROOT / "target" / f"{mod_id}-{version}.hnmod"
    out.parent.mkdir(exist_ok=True)
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in sorted(files.items()):
            info = zipfile.ZipInfo(name, (2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, content)
    print(json.dumps({"file": str(out), "size": out.stat().st_size,
                      "sha256": hashlib.sha256(out.read_bytes()).hexdigest()}, indent=2))


if __name__ == "__main__":
    main()
