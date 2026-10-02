#!/usr/bin/env python3
"""Retain exact CI file bytes, with separately retained ZIP/API provenance."""

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import tempfile
import unicodedata
import zipfile
import zlib

MAX_BYTES = 4 * 1024**3
MAX_MEMBERS = 50_000
MAX_JSON = 32 * 1024**2
CHUNK = 1024**2
HEX = re.compile(r"[0-9a-f]{64}\Z")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def ordinary(path):
    require(stat.S_ISREG(path.lstat().st_mode), f"not an ordinary file: {path}")


def file_identity(path, limit=MAX_BYTES):
    ordinary(path)
    total, sha = 0, hashlib.sha256()
    with path.open("rb") as handle:
        for part in iter(lambda: handle.read(CHUNK), b""):
            total += len(part)
            require(total <= limit, f"file exceeds byte bound: {path}")
            sha.update(part)
    return {"sha256": sha.hexdigest(), "bytes": total}


def json_bytes(value):
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=True) + "\n").encode()


def parse_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    def invalid(value):
        raise ValueError(f"invalid JSON constant: {value}")
    return json.loads(raw, object_pairs_hook=pairs, parse_constant=invalid)


def read_json(path, limit=MAX_JSON):
    ordinary(path)
    require(path.stat().st_size <= limit, "JSON exceeds byte bound")
    raw = path.read_bytes()
    return raw, parse_json(raw)


def integer(value, maximum):
    return type(value) is int and 0 <= value <= maximum


def safe_path(value):
    require(isinstance(value, str) and 0 < len(value.encode()) <= 1024, "invalid path length")
    parts = value.split("/")
    reserved = {"CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"} | {f"{p}{i}" for p in ("COM", "LPT") for i in "123456789¹²³"}
    for part in parts:
        require(part not in ("", ".", "..") and len(part.encode()) <= 255, "invalid path component")
        require(not any(ord(c) < 32 or c in '<>:"\\|?*' for c in part), "nonportable path")
        require(not part.endswith((".", " ")) and part.split(".")[0].upper() not in reserved, "nonportable path")
    return value


def path_key(value):
    return unicodedata.normalize("NFC", value).casefold()


def check_entries(entries):
    require(isinstance(entries, list) and len(entries) <= MAX_MEMBERS, "member bound")
    declared, namespace, total = set(), {}, 0
    for row in entries:
        name = safe_path(row["path"])
        require(name not in declared, "duplicate member")
        declared.add(name)
        kind = row["type"]
        require(kind in ("file", "directory"), "invalid member type")
        parts = name.split("/")
        for end in range(1, len(parts) + 1):
            current = "/".join(parts[:end])
            entry = (current, kind if end == len(parts) else "directory")
            key = path_key(current)
            require(key not in namespace or namespace[key] == entry, "path alias or file/directory conflict")
            namespace[key] = entry
        if kind == "file":
            require(integer(row["bytes"], MAX_BYTES), "invalid raw byte count")
            require(integer(row["gzip_bytes"], MAX_BYTES + MAX_BYTES // 100 + CHUNK), "invalid gzip byte count")
            require(HEX.fullmatch(row["sha256"]) and HEX.fullmatch(row["gzip_sha256"]), "invalid blob hash")
            total += row["bytes"]
    require(total <= MAX_BYTES, "uncompressed byte bound")
    return total


def metadata(raw, run_id, head_sha, name):
    require(len(raw) <= CHUNK, "metadata byte bound")
    value = parse_json(raw)
    require(type(value["workflow_run"]["id"]) is int and value["workflow_run"]["id"] == run_id and type(run_id) is int and run_id > 0, "run mismatch")
    require(re.fullmatch(r"[0-9a-f]{40}", head_sha) is not None and value["workflow_run"]["head_sha"] == head_sha, "head mismatch")
    require(safe_path(name) == name and "/" not in name and value["name"] == name, "name mismatch")
    require(type(value["id"]) is int and value["id"] > 0, "invalid artifact ID")
    require(integer(value["size_in_bytes"], MAX_BYTES), "ZIP byte bound")
    require(isinstance(value["digest"], str) and value["digest"].startswith("sha256:") and HEX.fullmatch(value["digest"][7:]), "invalid ZIP digest")
    return value


def directory(path):
    if path.exists() or path.is_symlink():
        require(stat.S_ISDIR(path.lstat().st_mode), f"not an ordinary directory: {path}")
    else:
        path.mkdir()


def retained(path, raw):
    if path.exists() or path.is_symlink():
        ordinary(path)
        require(path.stat().st_size == len(raw), "existing retained size differs")
        require(path.read_bytes() == raw, "existing retained bytes differ")
    else:
        with path.open("xb") as handle:
            handle.write(raw)


def blob_path(store, sha):
    require(isinstance(sha, str) and HEX.fullmatch(sha), "invalid content hash")
    return store / "blobs" / (sha + ".gz")


def decode_blob(store, row, output=None):
    path = blob_path(store, row["sha256"])
    require(file_identity(path, row["gzip_bytes"]) == {"sha256": row["gzip_sha256"], "bytes": row["gzip_bytes"]}, "compressed blob identity mismatch")
    count, sha = 0, hashlib.sha256()
    with gzip.open(path, "rb") as handle:
        while True:
            part = handle.read(min(CHUNK, row["bytes"] - count + 1))
            if not part:
                break
            count += len(part)
            require(count <= row["bytes"], "expanded blob exceeds expected size")
            sha.update(part)
            if output is not None:
                output.write(part)
    require(count == row["bytes"] and sha.hexdigest() == row["sha256"], "raw blob identity mismatch")


def encode_blob(store, handle, expected):
    descriptor, name = tempfile.mkstemp(prefix=".blob-", dir=store)
    temporary = Path(name)
    count, sha = 0, hashlib.sha256()
    try:
        with os.fdopen(descriptor, "wb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0, compresslevel=9) as zipped:
                for part in iter(lambda: handle.read(CHUNK), b""):
                    count += len(part)
                    require(count <= expected, "ZIP member exceeds declared size")
                    sha.update(part)
                    zipped.write(part)
        require(count == expected, "truncated ZIP member")
        identity = file_identity(temporary, MAX_BYTES + MAX_BYTES // 100 + CHUNK)
        row = {"sha256": sha.hexdigest(), "bytes": count, "gzip_sha256": identity["sha256"], "gzip_bytes": identity["bytes"]}
        dest = blob_path(store, row["sha256"])
        if dest.exists() or dest.is_symlink():
            # Deflate bytes may vary across compressor versions; raw identity is shared.
            existing = file_identity(dest, MAX_BYTES + MAX_BYTES // 100 + CHUNK)
            row.update(gzip_sha256=existing["sha256"], gzip_bytes=existing["bytes"])
            decode_blob(store, row)
        else:
            os.rename(temporary, dest)
        return row
    finally:
        temporary.unlink(missing_ok=True)


def load_index(store, expected_sha=None):
    for name in ("", "blobs", "manifests", "metadata"):
        require(stat.S_ISDIR((store / name).lstat().st_mode), "store directory is missing or unsafe")
    raw, index = read_json(store / "index.json")
    require(expected_sha is None or digest(raw) == expected_sha, "index identity mismatch")
    require(index["schema"] == 1 and isinstance(index["artifacts"], dict) and len(index["artifacts"]) <= 12, "invalid index")
    return index


def import_zip(zip_path, metadata_path, store, run_id, head_sha, name):
    raw, _ = read_json(metadata_path, CHUNK)
    api = metadata(raw, run_id, head_sha, name)
    require(file_identity(zip_path) == {"sha256": api["digest"][7:], "bytes": api["size_in_bytes"]}, "original ZIP identity mismatch")
    directory(store)
    lock = store / ".lock"
    with lock.open("x"):
        pass
    try:
        for part in ("blobs", "metadata", "manifests"):
            directory(store / part)
        index = load_index(store) if (store / "index.json").exists() else {"schema": 1, "run_id": run_id, "head_sha": head_sha, "artifacts": {}}
        require(index["run_id"] == run_id and index["head_sha"] == head_sha, "store run/head mismatch")
        require(len(index["artifacts"]) < 12, "artifact count bound")
        require(path_key(name) not in {path_key(n) for n in index["artifacts"]}, "duplicate artifact name")
        require(api["id"] not in [v["id"] for v in index["artifacts"].values()], "duplicate artifact ID")
        entries = []
        with zipfile.ZipFile(zip_path) as archive:
            members = archive.infolist()
            require(len(members) <= MAX_MEMBERS, "member bound")
            for member in members:
                mode = stat.S_IFMT(member.external_attr >> 16)
                kind = "directory" if member.is_dir() else "file"
                require(mode in (0, stat.S_IFDIR if kind == "directory" else stat.S_IFREG), "ZIP symlink or special file")
                require(not member.flag_bits & 1, "encrypted ZIP member")
                require(member.orig_filename == member.filename, "NUL in ZIP path")
                row = {"path": safe_path(member.filename[:-1] if member.is_dir() else member.filename), "type": kind}
                if kind == "file":
                    row.update(bytes=member.file_size, sha256="0" * 64, gzip_bytes=0, gzip_sha256="0" * 64)
                else:
                    require(member.file_size == 0, "nonempty directory entry")
                entries.append(row)
            total = check_entries(entries)
            for member, row in zip(members, entries):
                with archive.open(member) as source:
                    if row["type"] == "file":
                        row.update(encode_blob(store, source, row["bytes"]))
                    else:
                        require(source.read(1) == b"", "nonempty directory entry")
        manifest = {"schema": 1, "name": name, "id": api["id"], "run_id": run_id, "head_sha": head_sha, "zip": {"sha256": api["digest"][7:], "bytes": api["size_in_bytes"]}, "metadata_sha256": digest(raw), "member_count": len(entries), "total_bytes": total, "entries": sorted(entries, key=lambda row: row["path"])}
        encoded = json_bytes(manifest)
        require(len(encoded) <= MAX_JSON, "manifest exceeds byte bound")
        retained(store / "metadata" / (digest(raw) + ".json"), raw)
        retained(store / "manifests" / (digest(encoded) + ".json"), encoded)
        index["artifacts"][name] = {"id": api["id"], "manifest_sha256": digest(encoded)}
        encoded_index = json_bytes(index)
        with (store / "index.next").open("xb") as target:
            target.write(encoded_index)
        os.replace(store / "index.next", store / "index.json")
        return {"index_sha256": digest(encoded_index), "name": name, "manifest_sha256": digest(encoded), "members": len(entries), "bytes": total}
    finally:
        lock.unlink()


def restore(store, output, index_sha256):
    require(isinstance(index_sha256, str) and HEX.fullmatch(index_sha256), "invalid expected index hash")
    index = load_index(store, index_sha256)
    require(index["artifacts"], "empty archive")
    require(not output.exists() and not output.is_symlink(), "restore destination already exists")
    require(not (store / ".lock").exists(), "store import in progress")
    names, ids = set(), set()
    temporary = Path(tempfile.mkdtemp(prefix=".archive-restore-", dir=output.parent))
    try:
        for name, link in sorted(index["artifacts"].items()):
            safe_path(name)
            require("/" not in name and path_key(name) not in names and link["id"] not in ids, "duplicate artifact identity")
            names.add(path_key(name))
            ids.add(link["id"])
            sha = link["manifest_sha256"]
            require(isinstance(sha, str) and HEX.fullmatch(sha), "invalid manifest hash")
            raw, manifest = read_json(store / "manifests" / (sha + ".json"))
            require(digest(raw) == sha and manifest["schema"] == 1, "manifest identity mismatch")
            meta_sha = manifest["metadata_sha256"]
            require(isinstance(meta_sha, str) and HEX.fullmatch(meta_sha), "invalid metadata hash")
            meta_raw, _ = read_json(store / "metadata" / (meta_sha + ".json"), CHUNK)
            require(digest(meta_raw) == meta_sha, "metadata identity mismatch")
            api = metadata(meta_raw, index["run_id"], index["head_sha"], name)
            require(manifest["name"] == name and manifest["id"] == link["id"] == api["id"] and manifest["run_id"] == index["run_id"] and manifest["head_sha"] == index["head_sha"], "manifest provenance mismatch")
            require(manifest["zip"] == {"sha256": api["digest"][7:], "bytes": api["size_in_bytes"]}, "ZIP provenance mismatch")
            entries = manifest["entries"]
            require(check_entries(entries) == manifest["total_bytes"] and len(entries) == manifest["member_count"], "incomplete manifest")
            root = temporary / name
            root.mkdir()
            for row in entries:
                target = root / row["path"]
                target.parent.mkdir(parents=True, exist_ok=True)
                if row["type"] == "directory":
                    target.mkdir(exist_ok=True)
                else:
                    with target.open("xb") as handle:
                        decode_blob(store, row, handle)
        require(not output.exists() and not output.is_symlink(), "restore destination appeared")
        os.rename(temporary, output)
        return {"index_sha256": index_sha256, "artifacts": sorted(index["artifacts"])}
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    add = commands.add_parser("import")
    for name in ("zip", "metadata", "store"):
        add.add_argument("--" + name, required=True, type=Path)
    add.add_argument("--run-id", type=int, required=True)
    add.add_argument("--head-sha", required=True)
    add.add_argument("--name", required=True)
    replay = commands.add_parser("restore")
    replay.add_argument("--store", type=Path, required=True)
    replay.add_argument("--output", type=Path, required=True)
    replay.add_argument("--index-sha256", required=True)
    args = parser.parse_args()
    try:
        if args.command == "import":
            result = import_zip(args.zip, args.metadata, args.store, args.run_id, args.head_sha, args.name)
        else:
            result = restore(args.store, args.output, args.index_sha256)
        print(json.dumps(result, sort_keys=True))
    except (ValueError, TypeError, KeyError, OSError, EOFError, zipfile.BadZipFile, zlib.error, NotImplementedError) as error:
        parser.exit(1, f"archive: {error}\n")


if __name__ == "__main__":
    main()
