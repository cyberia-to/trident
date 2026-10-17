"""Retain reviewed SH7 evidence on the existing unpublished rehearsal draft."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import traceback

REPO = "cyberia-to/trisha"
RELEASE = 389977897
TAG = "candidate-20260916.1"


def identity(path):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"bytes": path.stat().st_size, "sha256": digest}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--retention", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    reviewed = json.loads((args.retention / "root-review.json").read_text())
    archive = Path(reviewed["archive"]["path"])
    expected = {k: reviewed["archive"][k] for k in ("bytes", "sha256")}
    if reviewed["status"] != "passed" or identity(archive) != expected:
        raise ValueError("reviewed archive identity required")
    args.output.mkdir(parents=True, exist_ok=False)
    receipt = {"schema": "trident/sh7-draft-evidence-transport/v1", "status": "running",
               "command": [sys.executable, *sys.argv], "driver": identity(Path(__file__)),
               "review": identity(args.retention / "root-review.json"),
               "scope": "Evidence retention on an existing draft; no release publication, tag or binary promotion.",
               "repository": REPO, "release_id": RELEASE, "archive": expected, "commands": []}

    def save():
        (args.output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")

    def run(name, command, expected_exit=0, binary_output=None):
        stdout = binary_output or (args.output / (name + ".stdout"))
        with stdout.open("xb") as stream:
            result = subprocess.run(command, stdout=stream, stderr=subprocess.PIPE, timeout=1800)
        (args.output / (name + ".stderr")).write_bytes(result.stderr)
        receipt["commands"].append({"name": name, "command": command, "exit_code": result.returncode,
                                    "stdout": identity(stdout),
                                    "stderr": identity(args.output / (name + ".stderr"))})
        save()
        if result.returncode != expected_exit:
            raise RuntimeError(f"{name}: unexpected exit {result.returncode}")
        return stdout

    def draft(name):
        path = run(name, ["gh", "api", f"repos/{REPO}/releases/{RELEASE}"])
        release = json.loads(path.read_text())
        if not release["draft"] or release["tag_name"] != TAG:
            raise ValueError("unpublished existing draft required")
        absent = json.loads(run(name + "-tag", ["gh", "api", f"repos/{REPO}/git/ref/tags/{TAG}"], 1).read_text())
        if str(absent.get("status")) != "404":
            raise ValueError("draft tag must remain absent")
        return release

    save()
    try:
        before = draft("draft-before")
        if archive.name in {a["name"] for a in before["assets"]}:
            raise ValueError("unique evidence asset name required")
        run("upload", ["gh", "release", "upload", TAG, str(archive), "--repo", REPO])
        after = draft("draft-after")
        assets = [a for a in after["assets"] if a["name"] == archive.name]
        if len(assets) != 1:
            raise ValueError("exactly one uploaded asset required")
        asset = assets[0]
        if asset["size"] != expected["bytes"] or asset.get("digest") != "sha256:" + expected["sha256"]:
            raise ValueError("server asset identity mismatch")
        receipt["asset"] = {k: asset[k] for k in ("id", "name", "size", "digest", "url", "browser_download_url")}
        save()
        downloaded = args.output / archive.name
        run("download", ["gh", "api", asset["url"], "-H", "Accept: application/octet-stream"],
            binary_output=downloaded)
        if identity(downloaded) != expected:
            raise ValueError("independently downloaded bytes differ")
        draft("draft-final")
        receipt.update(status="passed", downloaded=identity(downloaded))
    except BaseException:
        receipt.update(status="failed", error=traceback.format_exc())
        raise
    finally:
        save()
    print(json.dumps({"status": receipt["status"], "asset": receipt["asset"]}))


if __name__ == "__main__":
    main()
