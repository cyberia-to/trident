"""Check the committed small SH7 retention package without network or proof replay."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import tarfile

ROOT = Path(__file__).resolve().parent
ARCHIVE = dict(bytes=602191451,
    sha256="fa0f99785b4f499067683949a4126a1e4837558fbd421c0c4d61fedc39000d61")
PACKAGE = dict(bytes=138492,
    sha256="9db8e0f08be770423748ca5d376a02be261a22c62d209902a4b2eb92db81de47")
BASE = "bbcd7e455af1a92c6f2bb971dd3470325fad922f"


def require(value, message):
    if not value:
        raise ValueError(message)


def identity(path):
    require(path.is_file() and not path.is_symlink(), "regular retained file")
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return dict(bytes=path.stat().st_size, sha256=digest)


def load(name):
    return json.loads((ROOT / name).read_text())


def main():
    provenance = load("provenance.json")
    require(provenance["base_commit"] == BASE, "acceptance revision")
    names = set()
    for item in provenance["files"]:
        path = PurePosixPath(item["path"])
        require(not path.is_absolute() and ".." not in path.parts and item["path"] not in names,
                "unique local provenance path")
        names.add(item["path"])
        require(identity(ROOT / path) == {k: item[k] for k in ("bytes", "sha256")}, "retained original differs")
    archive = ROOT / "transport-audit.tar.gz"
    require(identity(archive) == PACKAGE, "exact reviewed compact archive")
    package = load("transport-audit-manifest.json")
    require({k: package["archive"][k] for k in ("bytes", "sha256")} == PACKAGE, "package identity")
    expected = package["files"]
    require(len(expected) == 235, "complete compact file inventory")
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        require(len(members) == len(expected) and {m.name for m in members} == set(expected), "exact archive members")
        for member in members:
            path = PurePosixPath(member.name)
            require(member.isfile() and not path.is_absolute() and ".." not in path.parts,
                    "regular relative archived file")
            stream = tar.extractfile(member)
            require(stream is not None, "archived payload exists")
            actual = dict(bytes=member.size, sha256=hashlib.file_digest(stream, "sha256").hexdigest())
            require(actual == expected[member.name], "archived member differs: " + member.name)

    transport = load("transport-receipt.json")
    review = load("transport-review.json")
    root = load("root-review/receipt.json")
    require(transport["status"] == review["status"] == root["status"] == "passed", "completed transport and reviews")
    require(transport["archive"] == transport["reassembled"] == review["archive"] == root["archive"] == ARCHIVE,
            "complete original archive identity")
    require(review["commands"] == root["commands"] == 64 and review["checked_logs"] == root["checked_logs"] == 128,
            "independent complete command/log replays")
    require(review["transport_receipt"] == root["transport_receipt"] == identity(ROOT / "transport-receipt.json"),
            "both reviewers bind same transport receipt")
    require(load("root-review/command.json")["exit_code"] == 0, "actual root replay passed")
    require(load("root-review/initial-invocation.json")["exit_code"] == 1, "initial wrong-path invocation retained")
    remote = load("remote-parts-manifest.json")
    require(remote["repository"] == "cyberia-to/trisha" and remote["release_id"] == 389977897
            and remote["tag"] == "candidate-20260916.1", "existing draft destination")
    require({k: remote["archive"][k] for k in ("bytes", "sha256")} == ARCHIVE, "remote complete archive binding")
    require(len(remote["parts"]) == len(remote["part_assets"]) == 5, "all five parts retained")
    offset = 0
    for number, (part, asset) in enumerate(zip(remote["parts"], remote["part_assets"]), 1):
        require(part["number"] == number and part["offset"] == offset
                and part["bytes"] == min(134217728, ARCHIVE["bytes"] - offset), "ordered exact part boundaries")
        require(asset["name"] == part["name"] and asset["size"] == part["bytes"]
                and asset["digest"] == "sha256:" + part["sha256"] and asset["state"] == "uploaded", "server part identity")
        require(asset["downloaded"] == {k: part[k] for k in ("bytes", "sha256")}, "downloaded part identity")
        offset += part["bytes"]
    require(offset == ARCHIVE["bytes"], "complete ordered length")
    require(transport["remote_manifest"]["downloaded"] == identity(ROOT / "remote-parts-manifest.json"),
            "actual uploaded and downloaded manifest bytes")

    first = load("postinstall/attempt-1/receipt.json")
    second = load("postinstall/attempt-2/receipt.json")
    require(first["commit"] == second["commit"] == BASE, "actual acceptance-commit installs")
    require(first["status"] == "failed" and first["exit_code"] == 0, "initial successful Cargo rejected by warning gate")
    require("warning: be sure to add" in (ROOT / "postinstall/attempt-1/install.stderr").read_text(), "original PATH warning")
    require(second["status"] == "passed" and second["exit_code"] == 0 and second["warning_lines"] == [], "corrected clean install")
    require(first["binary"] == second["binaries"]["trident"], "unchanged installed compiler bytes")
    for record in (first, second):
        require(record["clean_before"] == record["status_after"] == "", "actual clean source checkout")
        require(record["rustc"].startswith("rustc 1.89.0 ") and "host: aarch64-apple-darwin" in record["rustc"], "observed Rust toolchain")
    for name, expected_log in second["logs"].items():
        require(identity(ROOT / "postinstall/attempt-2" / name) == expected_log, "retained corrected install log")
    print(json.dumps(dict(status="passed", copied_files=len(names), compact_archive_files=len(expected),
        transport_commands=64, transport_logs=128, install_attempts=2,
        scope="Committed small package and original receipts; no new network download or proof replay.")))


if __name__ == "__main__":
    main()
