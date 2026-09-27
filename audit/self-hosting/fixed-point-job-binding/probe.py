"""Exercise real JOB1 snapshot binding without running either compiler."""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("fixed_point", HERE.parent / "check-selfhost-fixed-point.py")
CHECK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CHECK)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("receipt", "inventory-checker", "joy", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("choose a fresh output")
    receipt_bytes = CHECK.read(args.receipt)
    receipt, receipt_sha = json.loads(receipt_bytes), CHECK.digest(receipt_bytes)
    directory = Path(receipt["artifact_directory"]).resolve()
    manifest, _ = CHECK.read_json(directory / "package.json")
    inventory, _ = CHECK.read_json(Path(receipt["inventory"]))
    # A running full-build receipt may not have flushed the admitted job fields
    # yet. Its completed pack stdout still binds the retained JOB1 particle.
    packed = next(json.loads(row["stdout"]) for row in receipt["commands"]
                  if row["command"][1] == "pack-job" and row["exit_code"] == 0)
    job = CHECK.read(directory / "job.dag")
    CHECK.require(job[8:40].hex() == packed["package"]["job_particle"], "packed JOB1 identity")
    receipt["job_sha256"] = CHECK.digest(job)
    identities = dict(inventory_checker_sha256_start=CHECK.sha_file(args.inventory_checker),
                      job_checker=str(args.joy.resolve()), job_checker_sha256_start=CHECK.sha_file(args.joy))
    result = dict(scope="real source snapshot/JOB1 consistency component; no C2 or execution acceptance",
                  input_receipt=str(args.receipt.resolve()), input_receipt_sha256=receipt_sha,
                  input_job_sha256=receipt["job_sha256"], tools=identities)
    retained_receipt = args.output.with_suffix(".input.json")
    with retained_receipt.open("xb") as output:
        output.write(receipt_bytes)
    result["retained_input_receipt"] = str(retained_receipt.resolve())
    positive = dict(identities, inventory_checks=[], job_checks=[])
    CHECK.snapshot(receipt, directory, manifest, inventory, args.inventory_checker.resolve(), positive, bind_job=True)
    result["unchanged_snapshot"] = positive
    with tempfile.TemporaryDirectory(prefix="fixed-point-job-mutation-") as temporary:
        mutated = (Path(temporary) / "retained").resolve()
        shutil.copytree(directory, mutated)
        changed = copy.deepcopy(receipt)
        changed["artifact_directory"] = str(mutated)
        for module in manifest["modules"]:
            changed["sources"][module["logical_path"]]["copy"] = str(mutated / module["file"])
        source = changed["sources"]["native_compiler"]
        path = Path(source["copy"])
        old = path.read_bytes()
        new = old.replace(b"\n\n", b"\n ", 1)
        CHECK.require(len(old) == len(new) and old != new, "same-length whitespace mutation")
        path.write_bytes(new)
        source["sha256"] = CHECK.digest(new)
        (mutated / "source-root" / source["path"]).write_bytes(new)
        changed_inventory = mutated / "changed-inventory.json"
        command = [str(args.inventory_checker.resolve()), "--root", str(mutated / "source-root"),
                   "--entry", "compiler/nox/main.tri", "--output", str(changed_inventory)]
        generated = subprocess.run(command, capture_output=True, text=True, check=True, timeout=60)
        result["updated_inventory"] = dict(command=command, exit_code=generated.returncode,
                                           stdout=generated.stdout, stderr=generated.stderr)
        changed["inventory"] = str(changed_inventory)
        changed["inventory_sha256"] = CHECK.sha_file(changed_inventory)
        updated, _ = CHECK.read_json(changed_inventory)
        negative = dict(identities, inventory_checks=[], job_checks=[])
        try:
            CHECK.snapshot(changed, mutated, manifest, updated, args.inventory_checker.resolve(), negative, bind_job=True)
        except CHECK.Rejected as error:
            CHECK.require(str(error) == "retained JOB1 differs from the verified source snapshot", str(error))
            negative["expected_rejection"] = str(error)
        else:
            raise AssertionError("changed source accepted against old JOB1")
        result["changed_snapshot_with_updated_inventory"] = negative
    CHECK.require(CHECK.read(directory / "job.dag") == job, "original job preserved")
    result["original_job_preserved"] = True
    with args.output.open("x", encoding="utf-8") as output:
        json.dump(result, output, indent=2)
        output.write("\n")
    print(json.dumps(dict(status="passed", output=str(args.output))))


if __name__ == "__main__":
    main()
