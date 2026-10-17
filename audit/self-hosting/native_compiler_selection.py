"""Pinned compiler selection for installed acceptance; Joy owns ART1 admission."""
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class CompilerSelection:
    def __init__(self, args, parser):
        self.binary = args.joy.resolve()
        self.provided = args.compiler is not None
        self.compiler = args.compiler.resolve() if self.provided else None
        if self.provided and not self.compiler.is_file():
            parser.error('--compiler must name an existing ART1 file')
        output = args.output.resolve()
        for name, path in [('--joy', self.binary), ('--compiler', self.compiler)]:
            if path is not None and (output == path or
                    (output.exists() and path.exists() and output.samefile(path))):
                parser.error(f'--output must not replace {name}')
        if args.output.exists() or args.output.is_symlink():
            parser.error('choose a new receipt path; existing evidence is preserved')
        self.binary_sha = sha(self.binary)
        self.compiler_sha = sha(self.compiler) if self.provided else None
        args.output.parent.mkdir(parents=True, exist_ok=True)
        try:
            self.output = args.output.open('x', encoding='utf-8')
        except FileExistsError:
            parser.error('choose a new receipt path; existing evidence is preserved')

    def write_report(self, report):
        # Keep the exclusively created inode open. A later path replacement
        # cannot redirect receipt updates into a compiler or another file.
        self.output.seek(0)
        self.output.truncate()
        json.dump(report, self.output, indent=2)
        self.output.write('\n')
        self.output.flush()

    def close(self):
        self.output.close()

    def select(self, destination, build):
        if not self.provided:
            build(destination)
            self.compiler = destination
            self.compiler_sha = sha(destination)
        self.check()
        return self.compiler

    def check(self):
        require(self.binary.is_file() and sha(self.binary) == self.binary_sha,
                'installed Joy changed during acceptance')
        if self.compiler_sha is not None:
            require(self.compiler.is_file() and sha(self.compiler) == self.compiler_sha,
                    'compiler changed during acceptance')

    def before(self, arguments, reference_only=False):
        self.check()
        if self.provided and arguments[0] == 'build':
            require(reference_only, 'provided-compiler mode cannot build a compiler')
            require('--artifact-profile' in arguments and
                    arguments[arguments.index('--artifact-profile') + 1] == 'raw',
                    'reference build must produce a raw-profile oracle')
            target = Path(arguments[arguments.index('-o') + 1]).resolve()
            require(target != self.compiler and target != self.binary and
                    not (target.exists() and (target.samefile(self.compiler) or
                                             target.samefile(self.binary))),
                    'reference build cannot replace a pinned input')

    def after(self, arguments, result, expected=0):
        self.check()
        if expected or result is None:
            return
        if arguments[0] == 'pack-job':
            path = Path(arguments[arguments.index('--compiler') + 1])
            require(result['package']['compiler_particle'] == path.read_bytes()[8:40].hex(),
                    'packed JOB1 compiler identity')
            job = Path(arguments[arguments.index('-o') + 1])
            require(result['package']['job_particle'] == job.read_bytes()[8:40].hex(),
                    'packed JOB1 artifact identity')
        elif arguments[0] == 'run-artifact':
            path = Path(arguments[1])
            require(result['execution']['program_particle'] == path.read_bytes()[8:40].hex(),
                    'executed artifact identity')
            input_file = Path(arguments[arguments.index('--input') + 1])
            require(result['execution']['input_particle'] == input_file.read_bytes()[8:40].hex(),
                    'executed input identity')

    def describe(self):
        current = sha(self.compiler) if self.compiler is not None and self.compiler.is_file() else None
        return dict(compiler_mode='provided' if self.provided else 'seed-build',
                    compiler_path=str(self.compiler) if self.compiler else None,
                    compiler_sha256_start=self.compiler_sha, compiler_sha256_end=current,
                    binary_sha256_start=self.binary_sha,
                    binary_sha256_end=sha(self.binary) if self.binary.is_file() else None)
