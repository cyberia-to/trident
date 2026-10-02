"""Tiny receipt fixtures for admission logic; these are not execution evidence.

All checker functions are the actual SHA-pinned implementation. Only its
source-pin dictionary and expected compiler-output hash are replaced with the
synthetic fixture identities. No child or proof evaluator is ever invoked.
"""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
from unittest import mock

BASE = Path(__file__).resolve().parents[2]


def identity(path):
    data = Path(path).read_bytes()
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


class PriorFixture:
    def __init__(self, base, module):
        self.base, self.module, self.g = base, module, 1
        self.whole = base / "whole-proof"
        self.attacks = base / "whole-proof-attacks-v2-c1"
        self.shared = base / "whole-proof-attacks-parallel-v2"
        self.work = self.attacks / "whole-c1"
        self.producer = self.whole / "attempts/c1-selfbuild-1"
        self.verifier = self.whole / "attempts/c1-fresh-verification-1"
        self.proof = self.put(self.producer / "proof.joysc", b"synthetic complete certificate")
        self.compiler = self.put(self.verifier / "compiler.dag", b"synthetic compiler")
        self.joy = self.put(base / "production-install/installed/bin/joy", b"not executable")
        self.helper = self.put(self.attacks / "target-helper/release/whole-proof-mutator", b"not executable")
        self.guard = self.put(self.attacks / "guard.py", b"synthetic guard identity")
        checker = base / "whole-proof-final-review-v2/check.py"
        checker.parent.mkdir(parents=True)
        shutil.copyfile(BASE / "whole-proof-final-review-v2/check.py", checker)
        self.pins = {str(self.guard.relative_to(base)): identity(self.guard)["sha256"]}
        self.checker_hash = identity(checker)["sha256"]
        self.profile = dict(host_flags=["--budget", "20000000000", "--frames", "65536"],
                            wire_bytes=1000, decoded_bytes=2000, records=100, steps=200, cache_slots=2)
        write(self.whole / "profile.json", self.profile)
        self.flags = list(self.profile["host_flags"])
        for flag, key in (("proof-bytes", "wire_bytes"), ("proof-decoded-bytes", "decoded_bytes"),
                          ("proof-records", "records"), ("proof-steps", "steps"), ("proof-cache-slots", "cache_slots")):
            self.flags += ["--" + flag, str(self.profile[key])]
        self.put(self.whole / "run.py", b"synthetic producer driver")
        self.put(self.whole / "installed-source-receipt.json", b"synthetic installation identity")
        self.result = dict(format="joy-nox-disclosed-compiler-v1", disclosure="complete public witness",
                           physical_resource_claim="unattested", program_particle="program",
                           input_particle="input", output_particle="output", charged_reductions=7,
                           compiler_job={"fixture": True}, logical_peak_frames=3, expanded_steps=8,
                           records=4, transport={"decoded_bytes": 5}, elapsed_micros=1)
        old = {k: self.result[k] for k in ("program_particle", "input_particle", "output_particle",
                                          "charged_reductions", "compiler_job")}
        old.update(peak_frames=3, compaction={"evaluator_checkpoints": 9})
        write(self.whole / "inputs/accepted-c2-step.json", {"execution": {"execution": old}})
        self.put(self.whole / "inputs/c1.dag", b"original compiler")
        self.put(self.whole / "inputs/c1-job.dag", b"original job")
        self.inputs = {p.name: identity(p) for p in (self.whole / "inputs").iterdir()}
        write(self.whole / "preparation.json", {"files": self.inputs})
        self.frozen = self.attacks / "inputs/frozen"
        self.frozen.mkdir(parents=True)
        for name in self.inputs:
            shutil.copyfile(self.whole / "inputs" / name, self.frozen / name)
        self.put(self.frozen / "c2.dag", b"alternate compiler")
        for directory, action in ((self.producer, "prove"), (self.verifier, "verify")):
            response = dict(ok=True, schema="joy/artifact-proof/v1" if action == "prove" else "joy/artifact-verification/v1",
                            verification=self.result)
            write(directory / "stdout", response)
            self.put(directory / "stderr", b"")
            self.put(directory / "resources.jsonl", b"{}\n")
            command = [str(self.joy), action + "-artifact", str(self.whole / "inputs/c1.dag"),
                       "--input", str(self.whole / "inputs/c1-job.dag"), *self.flags]
            command += ["--output", "proof.joysc"] if action == "prove" else [
                "--proof", str(self.proof), "--output", "compiler.dag", "--emit", "program"]
            record = dict(status="passed", exit_code=0, action=action, generation=1,
                          driver=identity(self.whole / "run.py"), preparation=identity(self.whole / "preparation.json"),
                          profile_identity=identity(self.whole / "profile.json"), profile=self.profile,
                          installed_source_receipt=identity(self.whole / "installed-source-receipt.json"),
                          binary=identity(self.joy), binary_after=identity(self.joy),
                          inputs_before=self.inputs, inputs_after=self.inputs, command=command,
                          cwd=str(directory), environment={"PATH": ""}, accepted_execution_coordinates={"fixture": True},
                          files={p.name: identity(p) for p in directory.iterdir()})
            if action == "verify":
                record.update(proof_input=dict(path=str(self.proof), **identity(self.proof)),
                              proof_input_after=identity(self.proof))
            write(directory / "receipt.json", record)
        self.index, self.noun = self.work / "index.json", self.work / "result.dag"
        write(self.index, dict(schema="trident/whole-proof-mutation-index/v1",
                              source_sha256=identity(self.proof)["sha256"], source_bytes=self.proof.stat().st_size,
                              records=self.result["records"], decoded_bytes=self.result["transport"]["decoded_bytes"]))
        shutil.copyfile(self.compiler, self.noun)
        write(self.shared / "plan.json", {"proofs": {"1": identity(self.proof)}})
        self.put(self.shared / "resources.py", b"synthetic accounting identity")
        write(self.shared / "admission.json", dict(pid=900, children={"1": 901}))
        variants = {}
        self.coords = {str(g): dict(program_particle=str(g) * 64, formula_particle=str(g + 2) * 64) for g in (1, 2)}
        for mode in ("compiler", "source", "dependency", "cfg", "job-limit"):
            g = 2 if mode == "compiler" else 1
            job = self.put(self.attacks / f"inputs/c1/{mode}/job.dag", ("job " + mode).encode())
            variants[mode] = dict(compiler_generation=g, compiler=identity(self.frozen / f"c{g}.dag"),
                                  job=identity(job), admission={"job_particle": "4" * 64})
        self.preparation = dict(compiler_coordinates=self.coords, generations={"1": {"variants": variants}})
        write(self.attacks / "preparation.json", self.preparation)
        self.diagnostic("index", [self.helper, "index", self.proof, self.index, self.noun, identity(self.proof)["sha256"]],
                        0, dict(generation=1, complete_proof=True),
                        {str(p): identity(p) for p in (self.helper, self.proof)}, "helper", {})
        self.suite = dict(schema="trident/whole-self-build-certificate-checks/v2", status="failed", generation=1,
                          profile=self.profile, original_proof=identity(self.proof),
                          immutable_inputs_before={str(self.helper): identity(self.helper)},
                          immutable_inputs_after={str(self.helper): identity(self.helper)}, controls=[], rejections=[])
        self.suite["controls"].append(dict(name="original-fresh-verification", reused_existing_actual_control=True,
                                           receipt=str(self.verifier / "receipt.json"),
                                           receipt_identity=identity(self.verifier / "receipt.json"),
                                           certificate=identity(self.proof), output=identity(self.compiler)))
        for name in ("rechain", *module.PRIOR_NAMES):
            self.case(name)
        self.suite_path = self.work / "receipt.json"
        write(self.suite_path, self.suite)

    def put(self, path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        return path

    def diagnostic(self, name, argv, code, metadata, inputs, kind, stdout, stderr=b""):
        directory = self.attacks / ("attempts/whole-c1-" + name)
        write(directory / "stdout", stdout) if stdout is not None else self.put(directory / "stdout", b"")
        self.put(directory / "stderr", stderr)
        self.put(directory / "resources.jsonl", b"{}\n")
        caps = dict(wall=1800, cpu=1800, rss=1024**3, file=self.proof.stat().st_size + 32 * 1024**2) if kind == "helper" else dict(
            wall=7500, cpu=7500, rss=6 * 1024**3, file=32 * 1024**2)
        row = dict(schema="trident/whole-proof-attack-command/v2", status="passed", exit_code=code, expected_exit=code,
                   argv=list(map(str, argv)), metadata=metadata, cwd=str(directory), environment={"PATH": ""},
                   inputs_before=inputs, inputs_after=inputs, driver=identity(self.guard), caps=caps,
                   sampled_scope_disk_cap=26 * 1024**3, free_floor=8 * 1024**3, sample_interval_seconds=1,
                   shared_profile=identity(self.shared / "plan.json"), shared_driver=identity(self.shared / "resources.py"),
                   admission=identity(self.shared / "admission.json"), per_stream_log_bytes=1024**2, pid=902,
                   files={p.name: identity(p) for p in directory.iterdir()})
        write(directory / "receipt.json", row)
        write(self.shared / ("native-processes/" + directory.name + ".json"),
              dict(pid=902, pgid=902, parent=901, coordinator=900, argv=row["argv"]))
        return str((directory / "receipt.json").relative_to(self.attacks))

    def case(self, name):
        negative = name != "rechain"
        program, job = self.frozen / "c1.dag", self.frozen / "c1-job.dag"
        mode, context = "rechain", []
        error = "semantic record: Key" if name.startswith("rebound-") else "format/context mismatch"
        if negative:
            variant = name.split("-", 1)[1]
            g = 2 if variant == "compiler" else 1
            program, job = self.frozen / f"c{g}.dag", self.attacks / f"inputs/c1/{variant}/job.dag"
            if name.startswith("binding-"):
                mode = None
            else:
                mode = "rebind"
                context = [self.coords[str(g)]["program_particle"], self.coords[str(g)]["formula_particle"],
                           "4" * 64, "1", "20000000000", "65536"]
        candidate = self.work / ("certificate-" + name + ".joysc") if mode else self.proof
        cert_identity = identity(self.proof) if not negative or mode is None else dict(bytes=37, sha256="a" * 64)
        recipe = None
        if mode:
            output = dict(mode=mode, wire_bytes=cert_identity["bytes"], source_records=self.result["records"],
                          source_sha256=identity(self.proof)["sha256"])
            receipt = self.diagnostic("construct-" + name, [self.helper, "mutate", self.proof, self.index,
                                      self.noun, candidate, mode, *context], 0, dict(generation=1, mode=mode),
                                      {str(p): identity(p) for p in (self.helper, self.proof, self.index, self.noun)}, "helper", output)
            recipe = dict(mode=mode, context=context, certificate=cert_identity, construction_receipt=receipt)
        output = self.put(self.work / (name + ".dag"), b"protected" if negative else self.compiler.read_bytes())
        argv = [self.joy, "verify-artifact", program, "--input", job, "--proof", candidate,
                "--output", output, "--emit", "program", *self.flags, *(["--force"] if negative else [])]
        inputs = {str(p): identity(p) for p in (self.joy, program, job)}
        inputs[str(candidate)] = cert_identity
        if negative:
            inputs[str(output)] = identity(output)
        receipt = self.diagnostic("verify-" + name, argv, 1 if negative else 0,
                                  dict(generation=1, expected_error=error if negative else None), inputs, "verify",
                                  None if negative else dict(ok=True, schema="joy/artifact-verification/v1", verification=self.result),
                                  error.encode() if negative else b"")
        row = dict(name=name, certificate=cert_identity, recipe=recipe, verification_receipt=receipt)
        if negative:
            row.update(error=error, protected_output=identity(output))
        self.suite["rejections" if negative else "controls"].append(row)

    def review(self, result=None):
        actual_spec = importlib.util.spec_from_file_location
        fixture = self

        class Loader:
            def create_module(self, _spec):
                return None

            def exec_module(self, module):
                real = actual_spec(module.__name__, fixture.base / "whole-proof-final-review-v2/check.py")
                real.loader.exec_module(module)
                module.PINS = fixture.pins
                module.OUTPUT = identity(fixture.compiler)["sha256"]

        def synthetic_spec(name, path):
            return importlib.util.spec_from_loader(name, Loader())

        with mock.patch.object(self.module.importlib.util, "spec_from_file_location", synthetic_spec):
            return self.module.review(self.base, 1, self.proof, self.verifier, self.result if result is None else result)
