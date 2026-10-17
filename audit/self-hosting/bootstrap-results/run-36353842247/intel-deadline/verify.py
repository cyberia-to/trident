"""Check retained origin failure against the byte-exact artifact and local inputs."""
import gzip
import hashlib
import json
from pathlib import Path
import zipfile


def require(value, message):
    if not value:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    root = Path(__file__).resolve().parent
    files = json.loads((root / "files.json").read_bytes())
    for name, identity in files.items():
        data = (root / name).read_bytes()
        require(len(data) == identity["bytes"] and digest(data) == identity["sha256"], name)
        if "uncompressed_sha256" in identity:
            raw = gzip.decompress(data)
            require(len(raw) == identity["uncompressed_bytes"] and digest(raw) == identity["uncompressed_sha256"], name + " raw")
    api = json.loads((root / "artifact-10943644187.json").read_bytes())
    observed = json.loads((root / "x86_64-apple-darwin-failure.json").read_bytes())
    archive = root / observed["archive"]
    require(api["digest"] == observed["archive_digest"] == "sha256:" + digest(archive.read_bytes()), "GitHub ZIP digest")
    require(api["workflow_run"]["id"] == observed["run_id"] == 36353842247, "run binding")
    require(api["workflow_run"]["head_sha"] == observed["head_sha"], "head binding")
    with zipfile.ZipFile(archive) as bundle:
        receipt = json.loads(bundle.read("receipt.json"))
        step_bytes = bundle.read("repeat-1/c2-step.json")
        step = json.loads(step_bytes)
        require(digest(step_bytes) == observed["receipt_sha256"], "step binding")
        require(receipt["status"] == "failed" and receipt["pins"]["trident"] == observed["head_sha"], "failed pinned run")
        require(receipt["target"] == observed["target"] == "x86_64-apple-darwin", "target")
        command = step["commands"][-1]
        for key, source in (("command", "runtime_command"), ("stderr", "runtime_stderr"),
                            ("exit_code", "runtime_exit"), ("elapsed_nanoseconds", "elapsed_nanoseconds")):
            require(command[key] == observed[source], key)
        require(command["exit_code"] == 1 and "Execution(Cancelled)" in command["stderr"], "runtime rejection")
        require(command["command"][command["command"].index("--time-ms") + 1] == "3600000", "original deadline")
        require("repeat-1/c2.dag" not in bundle.namelist(), "no emitted C2")
        local = json.loads((root.parents[2] / "lexer-bootstrap/c1-to-c2.json").read_bytes())
        for key in ("compiler_sha256", "inventory_sha256", "job_sha256"):
            require(step[key] == local[key] == observed["local_same_input_reference"]["identities"][key], key)
        rust_warnings = []
        builds = [c for c in receipt["commands"] if c["command"][:2] == ["cargo", "build"]]
        require(len(builds) == 2 and all(c["exit_code"] == 0 for c in builds), "native builds")
        for build in builds:
            rust_warnings.extend(line for line in bundle.read(build["stderr"]).decode().splitlines()
                                 if line.startswith("warning:"))
        require(rust_warnings == observed["rust_warning_lines"] == [], "Rust warnings")
    print(json.dumps({"status": "passed", "scope": "original failure retention; SH6 remains open",
                      "run": observed["run_id"], "target": observed["target"]}))


if __name__ == "__main__":
    main()
