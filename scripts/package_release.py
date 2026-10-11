"""Package a native release build without including local simulation history."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
from pathlib import Path
import platform
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PLATFORMS = {
    ("Windows", "amd64"): "windows-x86_64",
    ("Windows", "x86_64"): "windows-x86_64",
    ("Linux", "x86_64"): "linux-x86_64",
    ("Darwin", "arm64"): "macos-arm64",
}


def dependency_notices() -> bytes:
    # Build/cache dependencies first; this command never downloads missing packages.
    host = next(
        line.removeprefix("host: ") for line in subprocess.check_output(
            ["rustc", "-vV"], cwd=ROOT, text=True, encoding="utf-8"
        ).splitlines() if line.startswith("host: ")
    )
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1",
         "--filter-platform", host],
        cwd=ROOT, text=True, encoding="utf-8",
    ))
    sections = ["Third-party crate licenses and notices"]
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["name"] == "martingale_sim":
            continue
        directory = Path(package["manifest_path"]).parent
        candidates = {
            p for p in directory.iterdir()
            if p.is_file() and p.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE"))
        }
        if package.get("license_file"):
            candidates.add(directory / package["license_file"])
        sections.append(
            f"\n{'=' * 72}\n{package['name']} {package['version']}\n"
            f"License: {package.get('license') or 'See included license file'}\n"
            f"Repository: {package.get('repository') or ''}"
        )
        for candidate in sorted(candidates):
            sections.append(f"\n--- {candidate.name} ---\n" + candidate.read_text(encoding="utf-8", errors="replace"))
    return ("\n".join(sections) + "\n").encode("utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=sorted(set(PLATFORMS.values())))
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output-dir", type=Path, default=ROOT / "dist")
    args = parser.parse_args()
    detected = PLATFORMS.get((platform.system(), platform.machine().lower()))
    target = args.platform or detected
    if not target:
        parser.error("Unsupported native platform; specify --platform and a matching --binary")
    binary_name = "martingale_sim.exe" if target.startswith("windows") else "martingale_sim"
    binary = args.binary or ROOT / "target" / "release" / binary_name
    if not binary.is_file():
        parser.error(f"Missing binary {binary}; run cargo build --release --locked first")
    launcher = "start_martingale.bat" if target.startswith("windows") else "start_martingale.sh"
    # An explicit allowlist keeps private CSV files and unrelated workspace content out.
    files = [ROOT / name for name in [
        "README.md", "LICENSE", "THIRD_PARTY_NOTICES.md", "CONTRIBUTING.md", "CHANGELOG.md", launcher
    ]] + sorted((ROOT / "docs").glob("*.md")) + sorted((ROOT / "examples").glob("*.json"))
    entries = [(binary_name, binary.read_bytes())] + [
        (file.relative_to(ROOT).as_posix(), file.read_bytes()) for file in files
    ]
    entries.append(("THIRD_PARTY_LICENSES.txt", dependency_notices()))
    args.output_dir.mkdir(parents=True, exist_ok=True)
    stem = f"martingale-{target}"
    folder = stem + "/"
    if target.startswith("windows"):
        archive = args.output_dir / (stem + ".zip")
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as package:
            for name, data in entries:
                package.writestr(folder + name, data)
    else:
        archive = args.output_dir / (stem + ".tar.gz")
        with tarfile.open(archive, "w:gz") as package:
            for name, data in entries:
                info = tarfile.TarInfo(folder + name)
                info.size = len(data)
                info.mode = 0o755 if name in [binary_name, launcher] else 0o644
                package.addfile(info, io.BytesIO(data))
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_name(archive.name + ".sha256").write_text(
        f"{digest}  {archive.name}\n", encoding="utf-8"
    )
    print(f"Created {archive}")


if __name__ == "__main__":
    main()
