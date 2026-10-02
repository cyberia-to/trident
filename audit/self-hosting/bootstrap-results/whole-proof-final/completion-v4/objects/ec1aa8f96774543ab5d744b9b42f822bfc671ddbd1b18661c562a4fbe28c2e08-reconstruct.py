"""Explicit prefix reconstruction to a new file; never invoked by reclamation."""
import argparse
import hashlib
import os
from pathlib import Path
import time

from safe_files import CHUNK, Held, digest, identity, parent_fd, read_json, require

ROOT = Path(__file__).resolve().parent


def reconstruct(plan, destination):
    """Authenticate all original bytes before creating a fresh output."""
    destination = Path(destination)
    protected = {plan['original']['path'], plan['target']['path']} | {p['path'] for p in plan['protected']}
    require(str(destination) not in protected, 'cannot reconstruct over a protected/original pathname')
    end = time.monotonic() + 600
    expected = plan['recipe']['prefix']
    with Held(Path(plan['original']['path']), plan['original']['state']) as original:
        require(digest(original, end) == plan['recipe']['original'], 'authenticate whole original before extraction')
        with parent_fd(destination) as (pfd, name):
            free = os.fstatvfs(pfd).f_bavail * os.fstatvfs(pfd).f_frsize
            require(free >= expected['bytes'] + 8589934592, 'fresh reconstruction admission preserves 8 GiB floor')
            fd = os.open(name, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600, dir_fd=pfd)
            try:
                copied, h = 0, hashlib.sha256()
                while copied < expected['bytes']:
                    require(time.monotonic() < end, 'bounded reconstruction scan')
                    block = os.pread(original.fd, min(CHUNK, expected['bytes'] - copied), copied)
                    require(block, 'original truncated while reconstructing')
                    with memoryview(block) as view:
                        while view:
                            wrote = os.write(fd, view)
                            require(wrote > 0, 'short reconstruction write')
                            view = view[wrote:]
                    h.update(block)
                    copied += len(block)
                os.fsync(fd)
                os.fsync(pfd)
                require({'bytes': copied, 'sha256': h.hexdigest()} == expected, 'reconstructed prefix identity')
                require(digest(original, end) == plan['recipe']['original'], 'whole original unchanged after extraction')
            finally:
                os.close(fd)
    require(identity(destination) == expected, 'fresh reconstructed file readback')
    return expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan-sha256', required=True)
    parser.add_argument('--destination-name', required=True)
    args = parser.parse_args()
    require(args.destination_name == Path(args.destination_name).name and
            args.destination_name.startswith('reconstructed-'), 'new local reconstruction filename')
    require(identity(ROOT / 'plan.json')['sha256'] == args.plan_sha256, 'exact reconstruction manifest')
    print(reconstruct(read_json(ROOT / 'plan.json'), ROOT / args.destination_name))


if __name__ == '__main__':
    main()
