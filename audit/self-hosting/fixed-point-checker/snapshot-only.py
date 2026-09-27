"""Exercise only retained source revalidation; no fixed-point acceptance."""
import argparse
import importlib.util
import json
from pathlib import Path
import sys

runner = Path(__file__).resolve().parents[1] / "check-selfhost-fixed-point.py"
spec = importlib.util.spec_from_file_location("fixed_point", runner)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
parser = argparse.ArgumentParser(description=__doc__)
for name in ("receipt", "inventory-checker", "output"):
    parser.add_argument("--" + name, type=Path, required=True)
args = parser.parse_args()
if args.output.exists() or args.output.is_symlink():
    parser.error("choose a new receipt path")
receipt, receipt_sha = module.read_json(args.receipt)
checker = args.inventory_checker.resolve()
report = dict(schema="trident/fixed-point-snapshot-guard/v1", status="running",
              scope="source snapshot metadata guard only; no completed fixed-point comparison",
              invocation=[sys.executable, *sys.argv], cwd=str(Path.cwd()),
              receipt=str(args.receipt.resolve()), receipt_sha256=receipt_sha,
              receipt_status=receipt["status"], checker_sha256=module.sha_file(runner),
              inventory_checker_sha256_start=module.sha_file(checker), inventory_checks=[])
try:
    directory = Path(receipt["artifact_directory"]).resolve()
    manifest, manifest_sha = module.read_json(directory / "package.json")
    module.require(manifest == receipt["manifest"], "retained manifest mismatch")
    inventory, inventory_sha = module.read_json(Path(receipt["inventory"]))
    module.require(inventory_sha == receipt["inventory_sha256"], "retained inventory mismatch")
    identities = module.snapshot(receipt, directory, manifest, inventory, checker, report)
    report.update(status="snapshot-verified", module_count=len(identities),
                  source_bytes=sum(row["source_bytes"] for row in identities.values()),
                  source_sha256_set=identities, manifest_sha256=manifest_sha, inventory_sha256=inventory_sha)
except Exception as error:
    report.update(status="rejected", error=dict(kind=type(error).__name__, message=str(error)))
with args.output.open("x") as output:
    json.dump(report, output, indent=2)
    output.write("\n")
print(json.dumps({key: report[key] for key in ("status", "scope", "receipt_status")}))
raise SystemExit(0 if report["status"] == "snapshot-verified" else 1)
