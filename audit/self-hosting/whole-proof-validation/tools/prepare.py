"""Pack alternate complete self-build requests without executing their compiler."""
import copy
import json
from pathlib import Path
import shutil
from guard import document, identity, run

ROOT = Path(__file__).resolve().parent
FROZEN = ROOT.parent / "whole-proof"
INSTALL = ROOT.parent / "production-install"
JOY = INSTALL / "installed/bin/joy"


def coordinates(path):
    raw = path.read_bytes()
    if len(raw) > 16 * 1024**2 or raw[:8] != b"NOXDAG01":
        raise ValueError("bounded compiler framing")
    nodes, cursor = {}, 44
    for _ in range(int.from_bytes(raw[40:44], "little")):
        particle, length = raw[cursor:cursor + 32], raw[cursor + 32]
        body = raw[cursor + 33:cursor + 33 + length]
        if length not in (8, 64) or len(body) != length:
            raise ValueError("compiler record")
        nodes[particle] = int.from_bytes(body, "little") if length == 8 else (body[:32], body[32:])
        cursor += 33 + length
    if cursor != len(raw):
        raise ValueError("compiler trailing bytes")
    fields, rest = [], raw[8:40]
    for _ in range(5):
        head, rest = nodes[rest]
        fields.append(head)
    if nodes[rest] != 0 or [nodes[p] for p in fields[:4]] != [0x41525431, 0, 1, 1]:
        raise ValueError("compiler ART1 schema")
    return dict(program_particle=raw[8:40].hex(), formula_particle=fields[4].hex())


def main():
    destination = ROOT / "inputs"
    destination.mkdir(exist_ok=False)
    preparation = json.loads((FROZEN / "preparation.json").read_text())
    profile = json.loads((FROZEN / "profile.json").read_text())
    assert identity(JOY) == preparation["binary"]
    originals = FROZEN / "inputs"
    # Copy only frozen inputs, never the live certificate directory.
    for name, expected in preparation["files"].items():
        assert identity(originals / name) == expected, name
        target = destination / "frozen" / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(originals / name, target)
    for name in ["profile.json", "preparation.json", "installed-source-receipt.json"]:
        shutil.copyfile(FROZEN / name, destination / name)
    base = json.loads((destination / "frozen/package.json").read_text())
    report = dict(schema="trident/whole-proof-alternate-inputs/v1", status="preparing",
                  binary=identity(JOY), frozen_preparation=identity(FROZEN / "preparation.json"),
                  scope="Whole 94-module requests; pack-job admission only; no whole proof consumed or compiler executed",
                  compiler_coordinates={str(g): coordinates(destination / f"frozen/c{g}.dag") for g in [1, 2]},
                  generations={})
    for generation in [1, 2]:
        original_pack = json.loads((FROZEN / f"pack-c{generation}.stdout").read_text())["package"]
        rows = {}
        for mode in ["compiler", "source", "dependency", "cfg", "job-limit"]:
            directory = destination / f"c{generation}" / mode
            directory.mkdir(parents=True)
            manifest = copy.deepcopy(base)
            edited = {}
            for module in manifest["modules"]:
                data = (destination / "frozen" / module["file"]).read_bytes()
                before = data
                if mode == "source" and module["logical_path"] == "native_compiler":
                    assert b"as_u32(0)" in data
                    data = data.replace(b"as_u32(0)", b"as_u32(1)", 1)
                if mode == "dependency" and module["logical_path"] == "std.compiler.nox.ascii":
                    assert data.count(b"as_u32(58)") == 1
                    data = data.replace(b"as_u32(58)", b"as_u32(59)", 1)
                assert len(data) == len(before)
                target = directory / module["file"]
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(data)
                if data != before:
                    edited[module["logical_path"]] = dict(original=identity(destination / "frozen" / module["file"]), altered=identity(target))
            if mode == "cfg":
                manifest["options"]["cfg_flags"] = ["whole_proof_changed"]
            if mode == "job-limit":
                manifest["limits"]["reductions"] -= 1
            document(directory / "package.json", manifest)
            compiler_generation = 3 - generation if mode == "compiler" else generation
            compiler = destination / f"frozen/c{compiler_generation}.dag"
            argv = [JOY, "pack-job", "--compiler", compiler, "--manifest", directory / "package.json",
                    "--output", directory / "job.dag", *profile["host_flags"]]
            inputs = [compiler, directory / "package.json", *(directory / m["file"] for m in manifest["modules"])]
            command = run(f"pack-c{generation}-{mode}", argv, "pack", inputs)
            admission = json.loads((ROOT / f"attempts/pack-c{generation}-{mode}/stdout").read_text())["package"]
            assert len(admission["modules"]) == 94
            assert sum(m["source_bytes"] for m in admission["modules"]) == 370544
            assert admission["job_particle"] != original_pack["job_particle"]
            rows[mode] = dict(compiler_generation=compiler_generation, compiler=identity(compiler),
                              manifest=identity(directory / "package.json"), job=identity(directory / "job.dag"),
                              admission=admission, source_changes=edited,
                              command_receipt=f"attempts/pack-c{generation}-{mode}/receipt.json", command_exit=command["exit_code"])
        report["generations"][str(generation)] = dict(original_admission=original_pack, variants=rows)
        document(ROOT / "preparation.json", report)
    report["status"] = "prepared"
    report["sources_unchanged"] = all(identity(originals / n) == v for n, v in preparation["files"].items())
    assert report["sources_unchanged"]
    document(ROOT / "preparation.json", report)
    print(json.dumps(dict(status="prepared", generations=2, alternate_jobs=10)))


if __name__ == "__main__":
    main()
