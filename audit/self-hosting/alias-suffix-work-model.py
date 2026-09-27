"""Static source model, never a guest execution or reduction measurement."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", type=Path, required=True)
    parser.add_argument("--closure", type=Path, required=True)
    args = parser.parse_args()
    inventory = json.loads(args.inventory.read_text())
    closure = json.loads(args.closure.read_text())
    imports = {}
    for name, row in closure["sources"].items():
        source = Path(row["copy"]).read_bytes()
        assert hashlib.sha256(source).hexdigest() == row["sha256"]
        imports[name] = re.findall(r"^use ([A-Za-z_][\w.]*)", source.decode(), re.M)
        assert set(imports[name]) == set(inventory["modules"][name]["imports"])
    owners, ordered = {}, []

    def discover(name):
        if name in owners:
            return
        owners[name] = len(owners)
        for dependency in imports[name]:
            discover(dependency)
        ordered.append(name)

    discover("native_compiler")
    assert len(ordered) == inventory["module_count"]
    names = sorted(inventory["modules"])
    positions = [dict(position=index, stop_stage=100 + index, graph_owner=owners[name],
                      job_index=names.index(name), name=name,
                      source_bytes=inventory["modules"][name]["source_bytes"],
                      functions=len(inventory["modules"][name]["functions"]))
                 for index, name in enumerate(ordered)]
    totals, modules = Counter(), {}
    for name, module in inventory["modules"].items():
        uses = imports[name]
        calls = {call: count for call, count in module["calls"].items()
                 if "." in call and any(call.rsplit(".", 1)[0] in
                     (owner, owner.rsplit(".", 1)[-1]) for owner in uses)}
        count = sum(calls.values())
        owner_reads, prefix_reads = 0, 0
        for call, occurrences in calls.items():
            prefix = call.rsplit(".", 1)[0]
            for owner in uses:
                if prefix == owner or len(prefix) >= len(owner):
                    continue
                start = len(owner) - len(prefix)
                owner_reads += occurrences
                if owner[start - 1] != ".":
                    continue
                for left, right in zip(owner[start:], prefix):
                    owner_reads += occurrences
                    if left == ".":
                        break
                    prefix_reads += occurrences
                    if left != right:
                        break
        row = dict(imported_call_sites=count, ordered_use_checks=count * len(uses),
                   old_basename_location_reads=count * sum(map(len, uses)),
                   suffix_owner_reads=owner_reads, suffix_prefix_reads=prefix_reads)
        modules[name] = row
        totals.update(row)
    print(json.dumps(dict(scope=__doc__, command=sys.argv,
                          revision=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
                          inventory_sha256=hashlib.sha256(args.inventory.read_bytes()).hexdigest(),
                          closure_sha256=hashlib.sha256(args.closure.read_bytes()).hexdigest(),
                          totals=dict(totals), modules=modules, inferred_positions=positions), indent=2))


if __name__ == "__main__":
    main()
