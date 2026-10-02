"""Independently check pilot receipts and retain every original evidence byte.

This receipt checker never executes payloads. It compares the already recorded
production verifier results; cryptographic replay uses Joy separately.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import tarfile


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(path):
    require(path.is_file() and not path.is_symlink(), f"regular file: {path}")
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"bytes": path.stat().st_size, "sha256": digest}


def read(path):
    return json.loads(path.read_text())


def exact(value):
    return {k: value[k] for k in ("bytes", "sha256")}


def relative(name):
    path = Path(name)
    require(not path.is_absolute() and ".." not in path.parts, f"safe path: {name}")
    return path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    source = args.source.resolve()
    package = source / "audit-package-v2"
    manifest = read(package / "manifest.json")
    package_files = read(package / "package-files.json")
    summary = read(package / "summary.json")
    files = {}
    identities = {}
    for name, expected in manifest.items():
        path = source / relative(name)
        require(path.resolve() == Path(expected["external_path"]).resolve(),
                f"original locator: {name}")
        require(identity(path) == exact(expected), f"original identity: {name}")
        if expected["storage"] == "package":
            require(identity(package / name) == exact(expected), f"copy identity: {name}")
        key = "evidence/" + name
        files[key], identities[key] = path, exact(expected)
    for name, expected in package_files.items():
        path = package / relative(name)
        require(identity(path) == expected, f"package identity: {name}")
    for path in sorted(package.rglob("*")):
        if not path.is_file():
            continue
        name = "evidence/" + str(path.relative_to(package))
        actual = identity(path)
        require(name not in identities or actual == identities[name], f"collision: {name}")
        files[name], identities[name] = path, actual

    expected_binary = exact(summary["binary"])
    binary = Path(summary["binary"]["path"])
    require(identity(binary) == expected_binary, "installed production binary")
    files["tools/joy-macos-arm64"] = binary
    identities["tools/joy-macos-arm64"] = expected_binary

    def command_receipt(name, exit_code):
        receipt_path = source / name
        r = read(receipt_path)
        require(r["exit_code"] == exit_code, f"exit: {name}")
        require(r["status"] == ("passed" if exit_code == 0 else "rejected"),
                f"status: {name}")
        require(exact(r["binary"]) == r["binary_end"] == expected_binary,
                f"binary: {name}")
        require(r["argument_files_start"] == r["argument_files_end"], f"arguments: {name}")
        require(r["pilot_sources_start"] == r["pilot_sources_end"], f"sources: {name}")
        for filename, expected in r["files"].items():
            require(identity(receipt_path.parent / filename) == expected,
                    f"command output: {name}/{filename}")
        return r

    positive = []
    for pilot in summary["pilots"]:
        name = pilot["name"]
        pr = command_receipt(pilot["prover_receipt"], 0)
        command_receipt(pilot["verifier_receipt"], 0)
        prove_dir = source / Path(pilot["prover_receipt"]).parent
        verify_dir = source / Path(pilot["verifier_receipt"]).parent
        pv = read(prove_dir / "stdout")["verification"]
        vv = read(verify_dir / "stdout")["verification"]
        require("prover_observations" not in vv, f"fresh verifier observations: {name}")
        for key, value in pilot["verified"].items():
            require(pv[key] == vv[key] == value, f"verified field {name}/{key}")
        proof = identity(prove_dir / "proof.joysc")
        require(proof == exact(pilot["proof"]) == pr["files"]["proof.joysc"], f"proof: {name}")
        require(proof["bytes"] == vv["transport"]["wire_bytes"], f"wire size: {name}")
        if name != "compile-error":
            command_receipt(f"attempts/verify-{name}-program-1/receipt.json", 0)
            command_receipt(f"attempts/execute-{name}-1/receipt.json", 0)
        positive.append({"name": name, "proof": proof})

    accepted, rejected = [], []
    aggregate = next(p for p in positive if p["name"] == "aggregate")["proof"]
    for filename in ("adversarial-results.json", "adversarial-valid-output-results.json"):
        cases = read(source / filename)
        for case in cases["results"]:
            code = case["expected_exit"]
            require(code in (0, 1) and case["actual_exit"] == code, f"case exit: {case['name']}")
            r = command_receipt(case["receipt"], code)
            directory = source / Path(case["receipt"]).parent
            require((directory / "stderr").read_text() == case["stderr"], f"error: {case['name']}")
            if code:
                require((directory / "stdout").stat().st_size == 0, f"success output: {case['name']}")
                destination = r["argv"][r["argv"].index("--output") + 1]
                require(case["destination_preserved"] and identity(Path(destination)) ==
                        r["argument_files_start"][destination], f"protected output: {case['name']}")
                rejected.append(case["name"])
            else:
                require(case["proof"] == aggregate, f"byte-identical control: {case['name']}")
                accepted.append(case["name"])
    require(len(positive) == 5 and len(accepted) == 3 and len(rejected) == 24,
            "complete pilot matrix")

    args.output.mkdir(parents=True, exist_ok=False)
    archive = args.output / "sh7-native-c2-complete-evidence-20261002.tar.gz"
    with tarfile.open(archive, "w:gz", compresslevel=1) as tar:
        for name, path in sorted(files.items()):
            require(identity(path) == identities[name], f"changed before archive: {name}")
            info = tar.gettarinfo(str(path), arcname=name)
            info.uid = info.gid = info.mtime = 0
            info.uname = info.gname = ""
            with path.open("rb") as stream:
                tar.addfile(info, stream)
    seen = set()
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar:
            require(member.isfile() and member.name in identities and member.name not in seen,
                    f"archive member: {member.name}")
            seen.add(member.name)
            with tar.extractfile(member) as stream:
                actual = {"bytes": member.size,
                          "sha256": hashlib.file_digest(stream, "sha256").hexdigest()}
            require(actual == identities[member.name], f"archived bytes: {member.name}")
    require(seen == set(identities), "complete archive")
    result = {"schema": "trident/sh7-evidence-retention/v1", "status": "passed",
              "command": [sys.executable, *sys.argv], "checker": identity(Path(__file__)),
              "source_revisions": summary["source_revisions"], "binary": expected_binary,
              "manifest": identity(package / "manifest.json"),
              "summary": identity(package / "summary.json"),
              "original_manifest_files": len(manifest), "package_files": len(package_files),
              "archive_files": len(files), "archive": {"path": str(archive), **identity(archive)},
              "pilots": positive, "accepted_controls": accepted, "rejected_attacks": rejected,
              "scope": "Independent receipt/byte validation and durable full evidence assembly; no new execution-proof verification."}
    (args.output / "root-review.json").write_text(json.dumps(result, indent=2) + "\n")
    (args.output / "archive-files.json").write_text(json.dumps(identities, indent=2) + "\n")
    print(json.dumps(result))


if __name__ == "__main__":
    main()
