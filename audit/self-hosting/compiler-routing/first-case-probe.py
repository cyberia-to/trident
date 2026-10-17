"""Stop externally after one real case; never claim full corpus acceptance."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
from unittest.mock import patch


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


runner = Path(__file__).resolve().parents[1] / "run-native-compiler.py"
sys.path.insert(0, str(runner.parent))
spec = importlib.util.spec_from_file_location("runner", runner)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
real_run = subprocess.run
commands = []
compiler = Path(sys.argv[sys.argv.index("--compiler") + 1]).resolve()
binary = Path(sys.argv[sys.argv.index("--joy") + 1]).resolve()
output = Path(sys.argv[sys.argv.index("--output") + 1]).with_suffix(".partial.json")
assert not output.exists(), "choose a new partial receipt path"
compiler_start, binary_start = sha(compiler), sha(binary)


class ExternalStop(Exception):
    pass


def probe(command, **kwargs):
    assert command[1] != "build", command
    if command[1] == "pack-job" and commands:
        # Reaching the next package requires all precedence-case assertions.
        assert [row["command"][1] for row in commands] == ["pack-job", "run-artifact", "run-artifact"]
        raise ExternalStop()
    result = real_run(command, **kwargs)
    commands.append({"command": command, "exit_code": result.returncode,
                     "stdout": result.stdout, "stderr": result.stderr})
    return result


with patch.object(module.subprocess, "run", probe):
    try:
        module.main()
    except ExternalStop:
        compiler_end, binary_end = sha(compiler), sha(binary)
        assert compiler_start == compiler_end and binary_start == binary_end
        receipt = {
            "schema": "trident/provided-compiler-partial/v1",
            "scope": "one real precedence case; externally stopped before the second package; not full corpus acceptance",
            "status": "partial-external-stop", "case": "precedence", "expected": 14,
            "invocation": [sys.executable, *sys.argv], "cwd": str(Path.cwd()),
            "runner_sha256": sha(runner), "probe_sha256": sha(Path(__file__)),
            "binary_sha256_start": binary_start, "binary_sha256_end": binary_end,
            "compiler_sha256_start": compiler_start, "compiler_sha256_end": compiler_end,
            "commands": commands, "build_invocations": 0,
        }
        output.write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps({k: v for k, v in receipt.items() if k != "commands"}))
