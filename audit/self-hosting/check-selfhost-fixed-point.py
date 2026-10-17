"""Check retained C1(S)->C2 and C2(S)->C3 files for an exact byte fixed point.

This is receipt/file consistency evidence, not full self-host acceptance. Joy's
recorded admission owns canonical ART1 validation and particle cryptography.
Explicitly supplied prebuilt tools check the source inventory and canonically
repack JOB1 from those exact bytes. They never execute a compiler stage;
semantic corpus acceptance and execution proofs remain separate gates.
"""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import tempfile

MAX_FILE = 16 << 20
SCOPE = "byte fixed point only; semantic corpus and full self-host acceptance not established"


class Rejected(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Rejected(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sha_file(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1 << 20), b""):
            value.update(chunk)
    return value.hexdigest()


def read(path, maximum=MAX_FILE):
    with path.open("rb") as source:
        data = source.read(maximum + 1)
    require(len(data) <= maximum, f"file exceeds bound: {path}")
    return data


def read_json(path):
    data = read(path)
    return json.loads(data), digest(data)


def particle(value, label):
    require(isinstance(value, str) and re.fullmatch(r"[0-9a-f]{64}", value), label)
    return value


def relative(value):
    require(isinstance(value, str) and value and "\\" not in value, "canonical relative source path")
    path = PurePosixPath(value)
    require(not path.is_absolute() and str(path) == value and
            all(part not in ("", ".", "..") for part in path.parts), "canonical relative source path")
    return path


def same_path(value, expected, label):
    require(isinstance(value, str) and Path(value).resolve() == expected.resolve(), label)


def artifact(path, expected_sha, limits, label):
    data = read(path, limits["artifact_bytes"])
    require(digest(data) == particle(expected_sha, f"{label} SHA256"), f"{label} bytes changed")
    # A framing guard, not an independent NOXDAG/ART1 cryptographic validator.
    require(len(data) >= 44 and data[:8] == b"NOXDAG01", f"{label} NOXDAG header")
    count = int.from_bytes(data[40:44], "little")
    require(0 < count <= limits["artifact_nodes"] and len(data) >= 44 + count * 41,
            f"{label} NOXDAG minimum framing")
    return data, data[8:40].hex(), count


def option(command, name):
    require(command.count(name) == 1, f"command option {name}")
    index = command.index(name)
    require(index + 1 < len(command), f"command value {name}")
    return command[index + 1]


def command_receipts(receipt, binary, compiler, directory, inventory):
    commands = receipt["commands"]
    require(isinstance(commands, list) and len(commands) >= 3, "completed probe commands")
    selected = {}
    for row in commands:
        command = row["command"]
        require(isinstance(command, list) and len(command) >= 2 and
                all(isinstance(value, str) for value in command), "command argv")
        require(type(row["exit_code"]) is int and row["exit_code"] == 0, "probe command failed")
        if command[1] in ("pack-job", "run-artifact"):
            kind = command[1]
            require(kind not in selected, "duplicate Joy command; retries are not accepted")
            require(kind == "pack-job" or "pack-job" in selected, "Joy execution precedes admission")
            same_path(command[0], binary, "command Joy binary")
            selected[kind] = (command, json.loads(row["stdout"]))
        else:
            require(not selected, "metadata command follows Joy execution/admission")
            # The existing probe uses cargo solely for its metadata inventory.
            # This checker itself never invokes cargo or a compiler build.
            prefix = ["cargo", "run", "--release", "--locked", "--offline",
                      "--example", "selfhost_inventory", "--"]
            metadata = command[:len(prefix)] == prefix or Path(command[0]).name == "selfhost_inventory"
            require(metadata and "--check" in command, "unexpected probe command")
            same_path(option(command, "--output"), inventory, "inventory command output")
            require(option(command, "--entry") == "compiler/nox/main.tri", "inventory entry")
    require(set(selected) == {"pack-job", "run-artifact"}, "Joy admission/execution commands")
    packed, admitted = selected["pack-job"]
    executed, execution = selected["run-artifact"]
    require(admitted == receipt["admission"] and execution == receipt["execution"], "Joy stdout binding")
    same_path(option(packed, "--compiler"), compiler, "pack compiler")
    same_path(option(packed, "--manifest"), directory / "package.json", "pack manifest")
    same_path(option(packed, "-o"), directory / "job.dag", "packed job path")
    same_path(executed[2], compiler, "executed compiler")
    same_path(option(executed, "--input"), directory / "job.dag", "executed JOB1")
    same_path(option(executed, "-o"), directory / "result.dag", "published artifact path")
    require(option(executed, "--emit") == "program", "program publication command")
    host = receipt["host_flags"]
    require(isinstance(host, list) and host and all(isinstance(v, str) for v in host), "host flags")
    allowed = {"--arena-nodes", "--budget", "--frames", "--time-ms", "--validation-visits",
               "--resident-nodes", "--collection-work"}
    require(len(host) % 2 == 0 and len(host[::2]) == len(set(host[::2])) and
            set(host[::2]) <= allowed and
            all(re.fullmatch(r"[0-9]+", value) and int(value) > 0 for value in host[1::2]),
            "bounded numeric host flags")
    require({"--arena-nodes", "--budget", "--frames", "--time-ms"} <= set(host[::2]) and
            (("--resident-nodes" in host) == ("--collection-work" in host)), "complete host limits")
    require(packed[1:3] == ["pack-job", "--compiler"] and packed[4] == "--manifest" and
            packed[6] == "-o" and packed[8:] == host, "pack argument/host flag binding")
    require(executed[3] == "--input" and executed[5:8] == ["--emit", "program", "-o"] and
            executed[9:] == host, "execution argument/host flag binding")


def snapshot(receipt, directory, manifest, inventory, checker, report, *, bind_job=False):
    sources, modules = receipt["sources"], manifest["modules"]
    names = [module["logical_path"] for module in modules]
    require(names == sorted(sources) and len(names) == len(set(names)) and names, "complete module set/order")
    require(set(inventory["modules"]) == set(names), "inventory module set")
    require(inventory["schema"] == 1 and inventory["roots"] == ["native_compiler"], "native compiler inventory root")
    require(manifest["entry_module"] == "native_compiler" and manifest["entry_function"] == "main", "compiler entry")
    identities, used_paths, total = {}, set(), 0
    with tempfile.TemporaryDirectory(prefix="trident-fixed-point-source-") as temporary:
        tree = Path(temporary)
        for module in modules:
            name = module["logical_path"]
            source, declared = sources[name], inventory["modules"][name]
            path = relative(source["path"])
            require(str(path) not in used_paths, "duplicate canonical source path")
            used_paths.add(str(path))
            expected = "compiler/nox/main.tri" if name == "native_compiler" else "lib/" + name.replace(".", "/") + ".tri"
            require(str(path) == expected == declared["path"], "canonical module source path")
            copy = (directory / relative(module["file"])).resolve()
            require(copy.is_relative_to(directory), "snapshot file escapes retained directory")
            same_path(source["copy"], copy, "snapshot copy binding")
            content = read(copy)
            require(len(content) == source["source_bytes"] == declared["source_bytes"], "snapshot source length")
            require(digest(content) == particle(source["sha256"], "source SHA256"), "snapshot source SHA256")
            total += len(content)
            require(total <= MAX_FILE, "complete source byte bound")
            target = tree / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(content)
            identities[name] = dict(path=str(path), sha256=source["sha256"], source_bytes=len(content),
                                    origin_name=module["origin_name"], origin_version=module["origin_version"])
        require(total == receipt["source_bytes"] == inventory["source_bytes"] == manifest["limits"]["source_bytes"], "source total")
        require(len(names) == receipt["module_count"] == inventory["module_count"], "module count")
        before = sha_file(checker)
        require(before == report["inventory_checker_sha256_start"], "inventory checker changed")
        command = [str(checker), "--root", str(tree), "--entry", "compiler/nox/main.tri",
                   "--output", receipt["inventory"], "--check"]
        row = dict(command=command, exit_code=None, status="running", stdout="", stderr="")
        report["inventory_checks"].append(row)
        checked = subprocess.run(command, capture_output=True, text=True, check=False, timeout=60)
        row.update(status="completed", exit_code=checked.returncode, stdout=checked.stdout, stderr=checked.stderr)
        report["inventory_checker_sha256_end"] = sha_file(checker)
        require(report["inventory_checker_sha256_end"] == before, "inventory checker changed during check")
        require(checked.returncode == 0, "snapshot inventory check failed")
        require(sha_file(Path(receipt["inventory"])) == receipt["inventory_sha256"], "inventory changed during check")
        # Repack from the very copies checked above, rather than trusting a
        # source_particle string in historical stdout. This binds source bytes,
        # module origins, options, limits and compiler to the actual saved JOB1.
        if bind_job:
            packed_manifest = dict(manifest)
            packed_manifest["modules"] = [dict(module, file=str(tree / identities[module["logical_path"]]["path"]))
                                          for module in modules]
            repack_job(receipt, tree, packed_manifest, report)
    return identities


def repack_job(receipt, temporary, manifest, report):
    joy = Path(report["job_checker"])
    before = sha_file(joy)
    require(before == report["job_checker_sha256_start"] == receipt["binary_sha256"],
            "job checker must be the recorded Joy binary")
    compiler = temporary / "compiler.dag"
    compiler_bytes = read(Path(receipt["compiler"]))
    require(digest(compiler_bytes) == receipt["compiler_sha256"], "compiler changed before job check")
    compiler.write_bytes(compiler_bytes)
    package, output = temporary / "package.json", temporary / "job.dag"
    package.write_text(json.dumps(manifest) + "\n", encoding="utf-8")
    command = [str(joy), "pack-job", "--compiler", str(compiler), "--manifest", str(package),
               "-o", str(output), *receipt["host_flags"]]
    row = dict(command=command, exit_code=None, status="running", stdout="", stderr="")
    report["job_checks"].append(row)
    checked = subprocess.run(command, capture_output=True, text=True, check=False, timeout=60)
    row.update(status="completed", exit_code=checked.returncode, stdout=checked.stdout, stderr=checked.stderr)
    report["job_checker_sha256_end"] = sha_file(joy)
    require(report["job_checker_sha256_end"] == before, "job checker changed during check")
    require(checked.returncode == 0, "snapshot JOB1 repack failed")
    packed = read(output, manifest["limits"]["artifact_bytes"])
    retained = read(Path(receipt["artifact_directory"]) / "job.dag", manifest["limits"]["artifact_bytes"])
    require(digest(retained) == receipt["job_sha256"], "JOB1 changed during snapshot check")
    require(packed == retained, "retained JOB1 differs from the verified source snapshot")
    row.update(job_sha256=digest(packed), exact_job_bytes_equal=True)


def step(path, checker, report):
    receipt, receipt_sha = read_json(path)
    require(receipt["schema"] == "trident/native-closure-probe/v1", "closure receipt schema")
    require(receipt["status"] == "compiler-returned" and receipt["published_kind"] == "program", "successful compiler program publication required")
    directory = Path(receipt["artifact_directory"]).resolve()
    binary, compiler, inventory_path = [Path(receipt[key]).resolve() for key in ("binary", "compiler", "inventory")]
    manifest, manifest_sha = read_json(directory / "package.json")
    require(manifest == receipt["manifest"] and manifest["version"] == 1, "retained package manifest")
    limits, options = manifest["limits"], manifest["options"]
    require(all(type(value) is int and value > 0 for value in limits.values()), "positive integer LIM1 fields")
    require(limits["artifact_bytes"] <= MAX_FILE and limits["artifact_nodes"] <= 196608, "artifact transport ceiling")
    require(options["target"] == 0 and options["input_profile"] == options["output_profile"] == 1, "compiler ART1 output profile")
    require(sha_file(binary) == receipt["binary_sha256"] == receipt["binary_sha256_end"], "Joy binary start/end binding")
    require(receipt["compiler_sha256"] == receipt["compiler_sha256_end"], "compiler start/end binding")
    compiler_bytes, compiler_particle, _ = artifact(compiler, receipt["compiler_sha256"], limits, "compiler")
    _, job_particle, job_count = artifact(directory / "job.dag", receipt["job_sha256"], limits, "JOB1")
    require(job_count == receipt["job_dag_entries"], "JOB1 node count")
    result, result_particle, result_count = artifact(directory / "result.dag", receipt["result_sha256"], limits, "emitted ART1")
    inventory, inventory_sha = read_json(inventory_path)
    require(inventory_sha == receipt["inventory_sha256"], "retained inventory SHA256")
    command_receipts(receipt, binary, compiler, directory, inventory_path)
    admission, execution = receipt["admission"], receipt["execution"]
    require(admission["schema"] == "joy/job-pack/v1" and admission["ok"] is True, "Joy admission success")
    require(execution["schema"] == "joy/artifact-run/v1" and execution["ok"] is True, "Joy execution success")
    same_path(admission["artifact"], directory / "job.dag", "admission artifact path")
    same_path(execution["artifact"], directory / "result.dag", "execution artifact path")
    package, ran = admission["package"], execution["execution"]
    compiled = ran["compiler_job"]
    require(ran["trace_mode"] == "none", "Joy NoTrace execution required")
    require(type(ran["charged_reductions"]) is int and 0 < ran["charged_reductions"] <= limits["reductions"],
            "successful gas must be positive and within LIM1")
    require(compiled["status"] == "success" and compiled["diagnostics"] == [], "successful compiler_job result")
    require(package["compiler_particle"] == ran["program_particle"] == compiler_particle, "executed compiler ART1 identity")
    require(package["job_particle"] == ran["input_particle"] == job_particle, "executed JOB1 identity")
    require(execution["published_particle"] == compiled["compiled_particle"] == result_particle, "published/compiler-returned ART1 identity")
    for key in ("options", "limits", "entry_module", "entry_function"):
        require(package[key] == compiled[key] == manifest[key], f"admitted/returned {key}")
    require(package["modules"] == compiled["modules"], "admitted/returned module identities")
    require(package["package_particle"] == compiled["package_particle"], "admitted/returned package identity")
    particle(package["package_particle"], "package particle")
    identities = snapshot(receipt, directory, manifest, inventory, checker, report, bind_job=True)
    require([module["logical_path"] for module in package["modules"]] == list(identities), "admitted module set/order")
    for module in package["modules"]:
        source = identities[module["logical_path"]]
        for key in ("source_bytes", "origin_name", "origin_version"):
            require(module[key] == source[key], f"admitted module {key}")
        particle(module["particle"], "module particle")
        particle(module["source_particle"], "source particle")
    summary = dict(receipt=str(path), receipt_sha256=receipt_sha, artifact_directory=str(directory),
                   manifest_sha256=manifest_sha, inventory_sha256=inventory_sha,
                   compiler_sha256=receipt["compiler_sha256"], compiler_particle=compiler_particle,
                   emitted_sha256=digest(result), emitted_particle=result_particle, emitted_dag_entries=result_count,
                   binary_sha256=receipt["binary_sha256"], module_count=len(identities), source_bytes=receipt["source_bytes"])
    report["steps"].append(summary)
    return dict(summary=summary, sources=identities, options=options, limits=limits,
                entry=(manifest["entry_module"], manifest["entry_function"]), modules=package["modules"],
                package_particle=package["package_particle"], host=receipt["host_flags"],
                compiler=compiler_bytes, result=result)


def verify(first, second, checker, report):
    require(first != second, "two distinct step receipts required")
    a, b = step(first, checker, report), step(second, checker, report)
    require(a["summary"]["artifact_directory"] != b["summary"]["artifact_directory"], "distinct retained step directories required")
    for key in ("sources", "options", "limits", "entry", "modules", "package_particle", "host"):
        require(a[key] == b[key], f"steps differ: {key}")
    for key in ("binary_sha256", "inventory_sha256"):
        require(a["summary"][key] == b["summary"][key], f"steps differ: {key}")
    require(b["compiler"] == a["result"], "second compiler is not the first emitted C2")
    require(a["result"] == b["result"], "C2 and C3 artifact bytes differ")
    require(a["summary"]["emitted_particle"] == b["summary"]["emitted_particle"], "C2 and C3 particles differ")
    report["fixed_point"] = dict(artifact_sha256=a["summary"]["emitted_sha256"],
                                particle=a["summary"]["emitted_particle"], artifact_bytes=len(a["result"]),
                                compiler_chain_bound=True, exact_artifact_bytes_equal=True,
                                source_sha256_set=a["sources"], options=a["options"], limits=a["limits"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("first", "second", "inventory-checker", "joy", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists() or args.output.is_symlink():
        parser.error("choose a new receipt path; existing evidence is preserved")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    report = dict(schema="trident/selfhost-fixed-point/v1", status="running", scope=SCOPE,
                  first=str(args.first.resolve()), second=str(args.second.resolve()),
                  inventory_checker=str(args.inventory_checker.resolve()), inventory_checks=[],
                  job_checker=str(args.joy.resolve()), job_checks=[], steps=[])
    # Exclusive creation also protects evidence when two processes race.
    with args.output.open("x", encoding="utf-8") as output:
        def flush():
            output.seek(0)
            output.truncate()
            json.dump(report, output, indent=2)
            output.write("\n")
            output.flush()
        flush()
        try:
            checker = args.inventory_checker.resolve()
            report["inventory_checker_sha256_start"] = sha_file(checker)
            report["job_checker_sha256_start"] = sha_file(args.joy.resolve())
            verify(args.first.resolve(), args.second.resolve(), checker, report)
            report["inventory_checker_sha256_end"] = sha_file(checker)
            require(report["inventory_checker_sha256_start"] == report["inventory_checker_sha256_end"], "inventory checker start/end binding")
            report["job_checker_sha256_end"] = sha_file(args.joy.resolve())
            require(report["job_checker_sha256_start"] == report["job_checker_sha256_end"], "job checker start/end binding")
            report["status"] = "passed"
        except Exception as error:
            report.pop("fixed_point", None)
            report.update(status="rejected", error=dict(kind=type(error).__name__, message=str(error)))
        finally:
            flush()
    print(json.dumps(dict(status=report["status"], receipt=str(args.output), scope=SCOPE)))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
