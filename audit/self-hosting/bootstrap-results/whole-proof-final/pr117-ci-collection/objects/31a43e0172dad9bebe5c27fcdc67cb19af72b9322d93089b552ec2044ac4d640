"""Native bootstrap producer, supplied-C2/C3 corpus jobs, and strict 36-phase replay.

See reference/self-hosting.md (SH6). Every phase gets fresh work/evidence paths.
The original v2 monolithic runner remains independently usable.
"""
import argparse
import json
import os
from pathlib import Path
import platform
import re
import sys

sys.dont_write_bytecode = True
import bootstrap_phase_checks as C

B = C.B


def separate(*paths):
    paths = [p.resolve() for p in paths]
    for i, a in enumerate(paths):
        for b in paths[i + 1:]:
            B.require(not a.is_relative_to(b) and not b.is_relative_to(a), "input/work/output directories must be separate")


def copy_exact(source, destination, expected):
    source, destination = Path(source), Path(destination)
    B.require(source.is_file() and not source.is_symlink(), "ordinary source file required")
    size = source.stat().st_size
    B.require(size <= B.MAX_EVIDENCE_BYTES and B.sha(source) == expected, "copy source identity/size")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with source.open("rb") as incoming, destination.open("xb") as outgoing:
        remaining = size
        while remaining:
            chunk = incoming.read(min(remaining, 1 << 20))
            B.require(chunk, "copy source truncated")
            outgoing.write(chunk)
            remaining -= len(chunk)
        B.require(not incoming.read(1), "copy source grew")
    B.require(destination.stat().st_size == size and B.sha(destination) == expected, "copied file identity")
    return destination


def seal(audit):
    files = B.evidence_files(audit.output)
    with (audit.output / "files.json").open("x", encoding="utf-8", newline="\n") as output:
        json.dump(files, output, sort_keys=True)
    audit.report["files"] = audit.identity(audit.output / "files.json")


def helpers_match(sources):
    B.require(C.implementation(sources / "trident/audit/self-hosting") == C.implementation(),
              "phase implementation differs from pinned Trident checkout")


def produce(audit, args, source_pins):
    B.require(args.producer is None and args.generation is None, "producer does not consume another phase")
    work = args.work.resolve() / f"repeat-{args.repeat}"
    audit.report["repetitions"] = []
    row = B.repetition(audit, work, args.repeat, args.target, source_pins,
                       producer_only=True, prepared=helpers_match)
    name = "joy.exe" if os.name == "nt" else "joy"
    binary = work / "target" / args.target / "release" / name
    exported = copy_exact(binary, audit.output / f"repeat-{args.repeat}" / name, row["tools"]["joy"])
    audit.report["joy"] = audit.identity(exported)
    audit.report["status"] = "produced"


def consume(audit, args, source_pins):
    B.require(args.producer is not None and args.generation in (2, 3), "corpus requires --producer and --generation")
    original_root = args.producer.resolve()
    separate(args.work, audit.output, original_root)
    original_path = original_root / "receipt.json"
    original = B.load(original_path)
    C.producer(original_root, original)
    for name in ("target", "repeat", "pins", "rust_version", "profile", "ci_origin", "implementation"):
        B.require(original[name] == audit.report[name], f"producer/corpus {name} differs")
    generation = args.generation
    selected = original["repetitions"][0][f"c{generation}"]
    producer_sha = B.sha(original_path)
    original_manifest = B.retained(original_root, original["files"])
    receipt_copy = copy_exact(original_path, audit.output / "producer-receipt.json", producer_sha)
    manifest_copy = copy_exact(original_manifest, audit.output / "producer-files.json", original["files"]["sha256"])
    compiler = copy_exact(B.retained(original_root, selected), audit.output / "inputs" / f"c{generation}.dag", selected["sha256"])
    name = "joy.exe" if os.name == "nt" else "joy"
    joy = copy_exact(B.retained(original_root, original["joy"]), audit.output / "inputs" / name, original["joy"]["sha256"])
    if os.name != "nt":
        joy.chmod(joy.stat().st_mode | 0o100)
    B.require(B.sha(joy) == original["joy"]["sha256"], "executable mode restoration changed bytes")
    audit.report.update(generation=generation, compiler=audit.identity(compiler), joy=audit.identity(joy),
                        producer_receipt=audit.identity(receipt_copy), producer_manifest=audit.identity(manifest_copy),
                        producer=dict(receipt_sha256=producer_sha, files_sha256=original["files"]["sha256"],
                                      generation=generation, compiler=selected, joy=original["joy"]),
                        execution=dict(compiler=str(compiler), joy=str(joy)), corpora={},
                        fixture_repositories={name: source_pins[name] for name in ("trident", "joy")})
    audit.flush()
    sources = B.checkout(audit, args.work.resolve(), source_pins, ("trident", "joy"))
    helpers_match(sources)
    audit.env["CARGO_NET_OFFLINE"] = "true"
    evidence = audit.output / "corpora"
    evidence.mkdir()
    B.generation_corpora(audit, evidence, sources / "trident/audit/self-hosting", compiler, joy, generation, audit.report)
    for repo in ("trident", "joy"):
        B.clean(audit, sources / repo, source_pins[repo])
    B.require(B.sha(original_path) == producer_sha, "original producer receipt changed")
    C.producer(original_root, B.load(original_path))
    B.require(B.sha(joy) == original["joy"]["sha256"] and B.sha(compiler) == selected["sha256"], "phase inputs changed")
    audit.report["status"] = "passed"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--phase", choices=("producer", "corpus", "matrix"), required=True)
    for name in ("work", "output", "producer", "matrix"):
        parser.add_argument("--" + name, type=Path)
    parser.add_argument("--target", choices=B.TARGETS)
    parser.add_argument("--repeat", type=int, choices=(1, 2))
    parser.add_argument("--generation", type=int, choices=(2, 3))
    parser.add_argument("--pins-json", default=os.environ.get("BOOTSTRAP_PINS"))
    parser.add_argument("--rust-version", default="1.95.0")
    args = parser.parse_args()
    B.require(args.output is not None, "fresh --output directory required")
    separate(*[path for path in (args.output, args.work, args.producer, args.matrix) if path is not None])
    audit = B.Audit(args.output, dict(phase=args.phase, scope="Native execution phases; proofs and semantic preservation separate"), schema=C.SCHEMA)
    try:
        audit.report.update(ci_origin=B.ci_origin(), implementation=C.implementation())
        if args.phase == "matrix":
            B.require(args.matrix is not None and not any((args.work, args.target, args.repeat, args.producer, args.generation)), "matrix requires only downloaded phase inputs")
            separate(args.matrix, audit.output)
            reports = [(p.parent, B.load(p)) for p in args.matrix.glob("*/receipt.json")]
            audit.report["phase_reports"] = [dict(path=str(root / "receipt.json"), sha256=B.sha(root / "receipt.json")) for root, _ in reports]
            if args.pins_json:
                B.require(all(report["pins"] == B.pins(args.pins_json) for _, report in reports), "matrix expected source pins")
            B.require(all(report["rust_version"] == args.rust_version for _, report in reports), "matrix expected Rust version")
            audit.report["compiler_sha256"] = C.compare_matrix(reports, audit.report["ci_origin"])
            audit.report["status"] = "passed"
        else:
            B.require(args.matrix is None and args.target and args.repeat and args.work and args.pins_json, "target/repeat/work/pins required")
            separate(args.work, audit.output)
            source_pins = B.pins(args.pins_json)
            origin = audit.report["ci_origin"]
            B.require(origin is None or origin["head_sha"] == source_pins["trident"], "phase head/source pin")
            B.require(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.rust_version), "exact stable Rust version")
            args.work.mkdir(parents=True, exist_ok=False)
            audit.env.update(CARGO_HOME=str(args.work.resolve() / "cargo-home"),
                             RUSTUP_TOOLCHAIN=f"{args.rust_version}-{args.target}")
            audit.report.update(target=args.target, repeat=args.repeat, pins=source_pins,
                                rust_version=args.rust_version, profile=B.PROFILE)
            rust = audit.run(["rustc", "-vV"], Path.cwd())
            B.native(args.target, rust, args.rust_version)
            audit.report["host"] = dict(platform=platform.platform(), python=sys.version, rust=rust,
                                        system=platform.system(), machine=platform.machine(), libc=platform.libc_ver()[0])
            (produce if args.phase == "producer" else consume)(audit, args, source_pins)
        seal(audit)
        if args.phase != "matrix":
            (C.producer if args.phase == "producer" else C.corpus)(audit.output, audit.report)
    except BaseException as error:
        audit.report.update(status="failed", error=f"{type(error).__name__}: {error}")
        if "files" not in audit.report:
            try:
                seal(audit)
            except BaseException as manifest_error:
                audit.report["manifest_error"] = f"{type(manifest_error).__name__}: {manifest_error}"
        audit.flush()
        print(audit.report["error"], file=sys.stderr)
        return 1
    audit.flush()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
