"""Read-only frame comparison; no Hemera, decompression, or evaluator execution."""
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import stat
import sys
import time

BASE = Path(__file__).resolve().parent.parent
ROOT = Path(__file__).resolve().parent
ORIGINAL = BASE / 'whole-proof/attempts/c2-selfbuild-1/proof.joysc'
MUTANT = BASE / 'whole-proof-attacks-v2-c2/whole-c2/certificate-rebound-job-limit.joysc'
EXPECTED = [
    '4db898cbc5133e0b96c758507d32b62aa82c4f39271cedd949b96d641a67de86',
    '2f6ce311471e969b92d2d23fda33077bfe7bd3895fe2d415c8e6e0c08b94a4ae',
]
SIZE = 10569174820


def require(condition, message):
    if not condition:
        raise ValueError(message)


def file_state(path):
    s = path.lstat()
    require(stat.S_ISREG(s.st_mode), 'regular file required')
    return dict(device=s.st_dev, inode=s.st_ino, bytes=s.st_size,
                mtime_ns=s.st_mtime_ns, ctime_ns=s.st_ctime_ns)


def small_identity(path):
    require(path.stat().st_size <= 16 * 1024 * 1024, 'bounded metadata')
    data = path.read_bytes()
    return dict(bytes=len(data), sha256=hashlib.sha256(data).hexdigest())


def compare():
    before = [file_state(p) for p in (ORIGINAL, MUTANT)]
    require(all(s['bytes'] == SIZE for s in before), 'exact expected sizes')
    hashes = [hashlib.sha256(), hashlib.sha256()]
    wire = [0, 0]

    def exact(stream, n, which):
        require(0 <= n <= 65536, 'bounded read size')
        data = stream.read(n)
        require(len(data) == n, 'complete read')
        wire[which] += n
        require(wire[which] <= SIZE, 'wire cap')
        hashes[which].update(data)
        return data

    frames = decoded = payload_bytes = changed_previous = changed_digest = 0
    payload_hash = hashlib.sha256()
    previous = [None, None]
    with ORIGINAL.open('rb') as a, MUTANT.open('rb') as b:
        streams = [a, b]
        prefixes = [exact(s, 40, i) for i, s in enumerate(streams)]
        require(all(p[:8] == b'JOYSC001' for p in prefixes), 'magic')
        require(prefixes[0][8:] != prefixes[1][8:], 'rebound context differs')
        while True:
            heads = [exact(s, 50, i) for i, s in enumerate(streams)]
            require(heads[0][:18] == heads[1][:18], 'header first18 equality')
            head = heads[0]
            require(int.from_bytes(head[:8], 'little') == frames, 'sequence')
            kind, codec = head[8:10]
            dec = int.from_bytes(head[10:14], 'little')
            enc = int.from_bytes(head[14:18], 'little')
            require(kind in (0, 1), 'kind')
            if kind == 1:
                require((codec, dec, enc) == (0, 0, 0), 'terminal dimensions')
            else:
                require(1 <= dec <= 65536, 'decoded dimensions')
            require((codec == 0 and enc == dec) or
                    (codec == 1 and 0 < enc < dec), 'codec dimensions')
            for i in range(2):
                if previous[i] is not None:
                    require(heads[i][18:] == previous[i], 'stored previous linkage')
            changed_previous += heads[0][18:] != heads[1][18:]
            payloads = [exact(s, enc, i) for i, s in enumerate(streams)]
            require(payloads[0] == payloads[1], 'complete encoded payload equality')
            payload_hash.update(payloads[0])
            previous = [exact(s, 32, i) for i, s in enumerate(streams)]
            changed_digest += previous[0] != previous[1]
            frames += 1
            decoded += dec
            payload_bytes += enc
            require(frames <= 422849 and decoded <= 27711707814, 'exact total caps')
            if kind == 1:
                require(all(s.read(1) == b'' for s in streams), 'exact terminal EOF')
                break
    require(wire == [SIZE, SIZE], 'complete whole files')
    digests = [h.hexdigest() for h in hashes]
    require(digests == EXPECTED, 'expected whole SHA256 identities')
    require((frames, decoded) == (422849, 27711707814), 'constructor totals')
    after = [file_state(p) for p in (ORIGINAL, MUTANT)]
    require(before == after, 'input states unchanged')
    return dict(paths=[str(ORIGINAL), str(MUTANT)], states_before=before,
                states_after=after, bytes=wire, sha256=digests,
                contexts=[p[8:].hex() for p in prefixes], frames=frames,
                decoded_bytes=decoded, encoded_payload_bytes=payload_bytes,
                encoded_payload_sha256=payload_hash.hexdigest(),
                equal_header_first18_count=frames, equal_payload_count=frames,
                differing_previous_fields=changed_previous,
                differing_digest_fields=changed_digest,
                single_terminal_exact_eof=True, stored_previous_linkage=True,
                hemera_recomputed=False, decompressed=False, evaluator_run=False)


if __name__ == '__main__':
    resource.setrlimit(resource.RLIMIT_CPU, (300, 300))
    resource.setrlimit(resource.RLIMIT_FSIZE, (1024 * 1024, 1024 * 1024))
    signal.signal(signal.SIGALRM, lambda *_: (_ for _ in ()).throw(TimeoutError('600s wall cap')))
    signal.alarm(600)
    start = time.time_ns()
    result = dict(schema='trident/completed-mutant-readonly-comparison/v1',
                  command=[sys.executable, '-B', str(Path(__file__).resolve())],
                  cwd=os.getcwd(), source=small_identity(Path(__file__)),
                  python=dict(path=sys.executable, version=sys.version),
                  limits=dict(wall_seconds=600, cpu_seconds=300,
                              output_bytes=1048576, per_read_bytes=65536),
                  started_ns=start, status='running')
    try:
        result['comparison'] = compare()
        result['status'] = 'passed-readonly-comparison'
    except BaseException as error:
        result.update(status='failed', error=repr(error))
        raise
    finally:
        result['elapsed_seconds'] = (time.time_ns() - start) / 1e9
        result['rusage'] = list(resource.getrusage(resource.RUSAGE_SELF))
        with (ROOT / 'comparison.json').open('x') as output:
            json.dump(result, output, indent=2)
            output.write('\n')
        print(json.dumps(result, indent=2))
