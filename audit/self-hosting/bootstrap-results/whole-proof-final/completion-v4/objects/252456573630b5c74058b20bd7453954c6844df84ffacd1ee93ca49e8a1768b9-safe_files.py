"""Closed-file identity and durable, non-overwriting evidence primitives."""
import contextlib
import hashlib
import json
import os
from pathlib import Path
import stat
import time

CHUNK = 1024 * 1024


def require(value, message):
    if not value:
        raise ValueError(message)


def state(value):
    names = ('st_dev', 'st_ino', 'st_mode', 'st_nlink', 'st_size',
             'st_mtime_ns', 'st_ctime_ns', 'st_birthtime')
    return {key: getattr(value, key, None) for key in names}


@contextlib.contextmanager
def parent_fd(path):
    """Reject symlinks in every ancestor, not only the final filename."""
    path = Path(path)
    require(path.is_absolute() and str(path) == os.path.normpath(path), 'canonical absolute path')
    fd = os.open('/', os.O_RDONLY | os.O_DIRECTORY)
    try:
        for part in path.parts[1:-1]:
            child = os.open(part, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW, dir_fd=fd)
            os.close(fd)
            fd = child
        yield fd, path.name
    finally:
        os.close(fd)


def same_directory(left, right):
    return (left.st_dev, left.st_ino) == (right.st_dev, right.st_ino)


class Held:
    def __init__(self, path, expected=None):
        self.path = Path(path)
        self.parent = parent_fd(self.path)
        self.pfd, self.name = self.parent.__enter__()
        self.fd = None
        self.unlinked = False
        try:
            self.fd = os.open(self.name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=self.pfd)
            self.expected = state(os.fstat(self.fd))
            require(stat.S_ISREG(self.expected['st_mode']), 'regular file required')
            require(self.expected['st_nlink'] == 1, 'single-link file required')
            if expected is not None:
                require(self.expected == expected, 'recorded inode/size/time identity changed')
            self.check()
        except BaseException:
            self.close()
            raise

    def check(self):
        require(state(os.fstat(self.fd)) == self.expected, 'held inode changed')
        with parent_fd(self.path) as (pfd, name):
            require(same_directory(os.fstat(self.pfd), os.fstat(pfd)), 'parent directory replaced')
            if self.unlinked:
                try:
                    os.stat(name, dir_fd=pfd, follow_symlinks=False)
                except FileNotFoundError:
                    return
                raise ValueError('unlinked name was recreated')
            require(state(os.stat(name, dir_fd=pfd, follow_symlinks=False)) == self.expected,
                    'named inode changed')

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None
        if self.parent is not None:
            self.parent.__exit__(None, None, None)
            self.parent = None

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        self.close()

    def unlink(self):
        self.check()
        os.unlink(self.name, dir_fd=self.pfd)
        self.unlinked = True  # Record the actual side effect before any later failure.
        after = state(os.fstat(self.fd))
        require(after['st_nlink'] == 0, 'classified inode was not unlinked')
        require(all(after[k] == v for k, v in self.expected.items()
                    if k not in ('st_ctime_ns', 'st_nlink')), 'inode content changed at unlink')
        self.expected = after
        os.fsync(self.pfd)
        self.check()


def deadline(end):
    require(time.monotonic() < end, 'read-only transition time allowance exceeded')


def digest(held, end):
    held.check()
    h = hashlib.sha256()
    n = 0
    while n < held.expected['st_size']:
        deadline(end)
        block = os.pread(held.fd, min(CHUNK, held.expected['st_size'] - n), n)
        require(block, 'unexpected EOF')
        h.update(block)
        n += len(block)
    require(not os.pread(held.fd, 1, n), 'unexpected trailing bytes')
    held.check()
    return {'bytes': n, 'sha256': h.hexdigest()}


def identity(path, end=None):
    with Held(path) as held:
        return digest(held, end or time.monotonic() + 600)


def compare_prefix(original, duplicate, recipe, end):
    """Compare every duplicate byte and authenticate the whole original."""
    original.check()
    duplicate.check()
    size = recipe['prefix']['bytes']
    require(0 < size < original.expected['st_size'], 'strict proper prefix required')
    require(duplicate.expected['st_size'] == size, 'wrong prefix size')
    whole, prefix, copied = (hashlib.sha256() for _ in range(3))
    n = 0
    while n < size:
        deadline(end)
        amount = min(CHUNK, size - n)
        a = os.pread(original.fd, amount, n)
        b = os.pread(duplicate.fd, amount, n)
        require(len(a) == amount and a == b, 'full prefix byte equality failed')
        whole.update(a)
        prefix.update(a)
        copied.update(b)
        n += amount
    require(not os.pread(duplicate.fd, 1, size), 'duplicate has trailing bytes')
    while n < original.expected['st_size']:
        deadline(end)
        block = os.pread(original.fd, min(CHUNK, original.expected['st_size'] - n), n)
        require(block, 'original unexpectedly truncated')
        whole.update(block)
        n += len(block)
    require(not os.pread(original.fd, 1, n), 'original grew')
    result = {'original': {'bytes': n, 'sha256': whole.hexdigest()},
              'prefix': {'bytes': size, 'sha256': prefix.hexdigest()},
              'duplicate': {'bytes': size, 'sha256': copied.hexdigest()}}
    require(result['original'] == recipe['original'], 'whole original authentication failed')
    require(result['prefix'] == result['duplicate'] == recipe['prefix'], 'prefix digest failed')
    original.check()
    duplicate.check()
    return result


def durable_bytes(path, data, mode=0o600):
    with parent_fd(path) as (pfd, name):
        fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode, dir_fd=pfd)
        with os.fdopen(fd, 'wb') as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.fsync(pfd)


def save(path, value):
    durable_bytes(path, (json.dumps(value, indent=2) + '\n').encode())


def make_directory(path):
    """Create a fresh evidence directory and sync its parent entry."""
    with parent_fd(path) as (pfd, name):
        os.mkdir(name, mode=0o700, dir_fd=pfd)
        os.fsync(pfd)


def ensure_directory(path):
    path = Path(path)
    if not path.exists():
        ensure_directory(path.parent)
        make_directory(path)
    with parent_fd(path / '_unused') as (pfd, _):
        os.fsync(pfd)


def read_json(path):
    with Held(path) as held:
        require(held.expected['st_size'] <= 16 * CHUNK, 'JSON input exceeds bounded size')
        data = os.pread(held.fd, held.expected['st_size'] + 1, 0)
        held.check()
        require(len(data) == held.expected['st_size'], 'JSON input changed')
        return json.loads(data)
