"""Recheck copied gate evidence, unchanged source scope and documentation links."""
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess

d = Path(__file__).resolve().parent
r = d.parents[4]

def identity(path):
    return dict(bytes=path.stat().st_size, sha256=hashlib.sha256(path.read_bytes()).hexdigest())

copied = 0
for group in ("debug-cargo", "release-two-threads", "release-eight-threads"):
    for item in json.loads((d / f"{group}-provenance.json").read_text())["files"]:
        assert identity(d / group / item["path"]) == {k: item[k] for k in ("bytes", "sha256")}
        copied += 1
receipt = json.loads((d / "release-eight-threads/receipt.json").read_text())
assert receipt["status"] == "passed" and receipt["exit_code"] == 0 and receipt["warning_lines"] == []
assert receipt["base_commit"] == "bbcd7e455af1a92c6f2bb971dd3470325fad922f"
assert receipt["driver"] == identity(d / "run_release_gate.py")
assert receipt["environment_receipt"] == identity(d / "debug-cargo/receipt.json")
for name, expected in receipt["files"].items():
    assert identity(d / "release-eight-threads" / name) == expected
assert receipt["elapsed_seconds"] < receipt["wall_seconds"] == 1800
assert receipt["sampled_peak_group_rss_bytes"] <= receipt["sampled_group_rss_limit_bytes"] == 4 * 1024**3
counts = [list(map(int, row)) for row in re.findall(
    r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;",
    (d / "release-eight-threads/stdout").read_text())]
summary = json.loads((d / "release-counts.json").read_text())
assert summary["counts"] == [sum(row[i] for row in counts) for i in range(5)] == [1231, 0, 5, 0, 0]
assert summary["test_result_records"] == len(counts) == 49
assert summary["receipt"] == identity(d / "release-eight-threads/receipt.json")
paths = ["src", "lib", "compiler", "catalog", "silicon", "Cargo.toml", "Cargo.lock"]
command = ["git", "diff", "--exit-code", "bbcd7e455af1a92c6f2bb971dd3470325fad922f", "--", *paths]
subprocess.run(command, cwd=r, check=True, capture_output=True)
links = []
documents = [d / "README.md", d.parent / "README.md"]
for document in documents:
    for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", document.read_text()):
        if "://" in target or target.startswith("#"):
            continue
        assert (document.parent / target.split("#")[0]).exists(), (document, target)
        links.append(dict(source=str(document.relative_to(r)), target=target))
plan_lines = sum(len(p.read_text().splitlines()) for p in (r / ".claude").rglob("*.md"))
assert plan_lines <= 1000
for source in (d / "run_release_gate.py", d / "check_delivery.py", d.parent / "check_retention.py"):
    ast.parse(source.read_text())
subprocess.run(["git", "diff", "--check"], cwd=r, check=True, capture_output=True)
print(json.dumps(dict(status="passed", copied_gate_files=copied, test_counts=summary["counts"],
    local_links=links, plan_lines=plan_lines, production_command=command,
    scope="Local copied gate evidence and documentation; no new proof execution or network download."), indent=2))
