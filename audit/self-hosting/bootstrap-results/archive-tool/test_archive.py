#!/usr/bin/env python3
"""Focused byte-preservation, completeness and fail-closed archive guards."""

import importlib.util
import io
import json
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import warnings
import zipfile

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("archive_tool", HERE / "archive.py")
archive = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(archive)
HEAD = "a" * 40


class ArchiveTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.store = self.root / "store"
        self.number = 0

    def fixture(self, entries=None, name="bootstrap-x86_64-apple-darwin-repeat-1-attempt-1", run=12, head=HEAD):
        self.number += 1
        path = self.root / f"input-{self.number}.zip"
        entries = entries if entries is not None else [("receipt.json", b'{"status":"failed"}\r\n'), ("commands/0.stderr", b""), ("binary.dag", bytes(range(256)) * 100)]
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as handle:
                for member, raw in entries:
                    handle.writestr(member, raw)
        data = {"id": self.number, "name": name, "size_in_bytes": path.stat().st_size, "digest": "sha256:" + archive.file_identity(path)["sha256"], "workflow_run": {"id": run, "head_sha": head}}
        meta = self.root / f"input-{self.number}.json"
        # Deliberate formatting and CRLF are retained, not reserialized.
        meta.write_bytes((json.dumps(data, indent=3) + "\r\n").encode())
        return path, meta, data

    def ingest(self, fixture):
        path, meta, data = fixture
        return archive.import_zip(path, meta, self.store, data["workflow_run"]["id"], data["workflow_run"]["head_sha"], data["name"])

    def replay(self):
        output = self.root / "restored"
        result = archive.restore(self.store, output, archive.file_identity(self.store / "index.json")["sha256"])
        return output, result

    def manifest(self):
        index = json.loads((self.store / "index.json").read_bytes())
        name, link = next(iter(index["artifacts"].items()))
        path = self.store / "manifests" / (link["manifest_sha256"] + ".json")
        return name, path, json.loads(path.read_bytes())

    def replace_manifest(self, value):
        # Rebind the local index to exercise structural validation beneath hashes.
        index_path = self.store / "index.json"
        index = json.loads(index_path.read_bytes())
        name = next(iter(index["artifacts"]))
        raw = archive.json_bytes(value)
        sha = archive.digest(raw)
        (self.store / "manifests" / (sha + ".json")).write_bytes(raw)
        index["artifacts"][name]["manifest_sha256"] = sha
        index_path.write_bytes(archive.json_bytes(index))

    def test_twelve_roots_share_exact_bytes_and_retain_raw_metadata(self):
        originals = []
        for repeat in range(1, 3):
            for platform in range(6):
                item = self.fixture(name=f"bootstrap-platform-{platform}-repeat-{repeat}-attempt-1")
                originals.append(item)
                self.ingest(item)
        self.assertEqual(len(list((self.store / "blobs").iterdir())), 3)
        empty = self.store / "blobs" / (archive.digest(b"") + ".gz")
        self.assertTrue(empty.is_file())
        output, result = self.replay()
        self.assertEqual(len(result["artifacts"]), 12)
        for path, meta, data in originals:
            with zipfile.ZipFile(path) as handle:
                for member in handle.infolist():
                    restored = output / data["name"] / member.filename
                    self.assertFalse(restored.is_symlink())
                    self.assertEqual(restored.read_bytes(), handle.read(member))
            retained = self.store / "metadata" / (archive.digest(meta.read_bytes()) + ".json")
            self.assertEqual(retained.read_bytes(), meta.read_bytes())

    def test_deterministic_store_and_empty_directories(self):
        fixture = self.fixture([("empty/", b""), ("a/b", b"x\x00\r\n")])
        self.ingest(fixture)
        first = {str(p.relative_to(self.store)): p.read_bytes() for p in self.store.rglob("*") if p.is_file()}
        self.store = self.root / "second"
        self.ingest(fixture)
        second = {str(p.relative_to(self.store)): p.read_bytes() for p in self.store.rglob("*") if p.is_file()}
        self.assertEqual(first, second)
        output, _ = self.replay()
        self.assertTrue((output / fixture[2]["name"] / "empty").is_dir())

    def test_existing_alternate_gzip_encoding_is_verified_and_reused(self):
        first = self.fixture()
        original = archive.gzip.GzipFile
        def alternate(*args, **kwargs):
            if kwargs.get("mode") == "wb":
                kwargs.update(mtime=123, compresslevel=1)
            return original(*args, **kwargs)
        with patch.object(archive.gzip, "GzipFile", side_effect=alternate):
            self.ingest(first)
        before = {p.name: p.read_bytes() for p in (self.store / "blobs").iterdir()}
        second = self.fixture(name="second-artifact")
        self.ingest(second)
        after = {p.name: p.read_bytes() for p in (self.store / "blobs").iterdir()}
        self.assertEqual(before, after)
        self.assertTrue(all(int.from_bytes(raw[4:8], "little") == 123 for raw in after.values()))
        output, result = self.replay()
        self.assertEqual(len(result["artifacts"]), 2)
        for item in (first, second):
            with zipfile.ZipFile(item[0]) as source:
                for member in source.infolist():
                    self.assertEqual((output / item[2]["name"] / member.filename).read_bytes(), source.read(member))

    def test_original_zip_verified_before_zip_parser(self):
        item = self.fixture()
        item[0].write_bytes(item[0].read_bytes() + b"x")
        with patch.object(archive.zipfile, "ZipFile", side_effect=AssertionError("parser called")):
            with self.assertRaisesRegex(ValueError, "original ZIP identity"):
                self.ingest(item)
        self.assertFalse(self.store.exists())

    def test_wrong_provenance_and_digest_are_rejected(self):
        for key, value in (("size_in_bytes", 0), ("digest", "md5:bad"), ("id", True)):
            with self.subTest(key=key):
                item = self.fixture()
                data = item[2].copy()
                data[key] = value
                item[1].write_text(json.dumps(data))
                with self.assertRaises(ValueError):
                    self.ingest(item)
        item = self.fixture()
        for run, head, name in ((13, HEAD, item[2]["name"]), (12, "b" * 40, item[2]["name"]), (12, HEAD, "wrong")):
            with self.assertRaises(ValueError):
                archive.import_zip(item[0], item[1], self.store, run, head, name)

    def test_path_escapes_aliases_conflicts_duplicates_and_symlinks_rejected(self):
        bad_names = ("../escape", "/absolute", "C:/escape", "a\\escape", "a/../b", "a//b", "con.txt", "trailing. ", "COM¹", "COM².txt", "x/LPT³.log", "CONIN$", "CONOUT$")
        cases = [[(name, b"bad")] for name in bad_names]
        cases += [[("same", b"1"), ("same", b"2")], [("A/x", b"1"), ("a/y", b"2")], [("a", b"1"), ("a/b", b"2")], [("e\u0301", b"1"), ("\u00e9", b"2")]]
        link = zipfile.ZipInfo("link")
        link.create_system = 3
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        cases.append([(link, b"../escape")])
        for entries in cases:
            with self.subTest(entries=entries):
                with self.assertRaises(ValueError):
                    self.ingest(self.fixture(entries))
                self.assertFalse((self.store / "index.json").exists())

    def test_member_and_uncompressed_bounds(self):
        item = self.fixture()
        with patch.object(archive, "MAX_MEMBERS", 2):
            with self.assertRaisesRegex(ValueError, "member bound"):
                self.ingest(item)
        # ZIP is small; expanded data exceeds this deliberately tiny test bound.
        with patch.object(archive, "MAX_BYTES", 1000):
            with self.assertRaises(ValueError):
                self.ingest(item)
        self.assertFalse((self.store / "index.json").exists())

    def test_crc_corruption_cannot_publish_partial_artifact(self):
        item = self.fixture([("first", b"first"), ("last", b"corrupt me")])
        # Use stored data for a controlled payload edit, then honestly rebind ZIP metadata.
        with zipfile.ZipFile(item[0], "w", zipfile.ZIP_STORED) as handle:
            handle.writestr("first", b"first")
            handle.writestr("last", b"corrupt me")
        item[0].write_bytes(item[0].read_bytes().replace(b"corrupt me", b"corrupt ME"))
        item[2]["digest"] = "sha256:" + archive.file_identity(item[0])["sha256"]
        item[2]["size_in_bytes"] = item[0].stat().st_size
        item[1].write_text(json.dumps(item[2]))
        with self.assertRaises(zipfile.BadZipFile):
            self.ingest(item)
        self.assertFalse((self.store / "index.json").exists())
        self.assertFalse((self.store / ".lock").exists())
        with self.assertRaises(FileNotFoundError):
            self.replay()

    def test_generated_manifest_bound_prevents_unrestorable_publication(self):
        with patch.object(archive, "MAX_JSON", 10):
            with self.assertRaisesRegex(ValueError, "manifest exceeds byte bound"):
                self.ingest(self.fixture())
        self.assertFalse((self.store / "index.json").exists())

    def test_existing_retained_size_is_checked_before_reading(self):
        target = self.root / "oversized.json"
        target.write_bytes(b"larger than expected")
        with patch.object(Path, "read_bytes", side_effect=AssertionError("unbounded read")):
            with self.assertRaisesRegex(ValueError, "existing retained size differs"):
                archive.retained(target, b"x")

    def test_altered_or_missing_blob_does_not_publish_any_restored_root(self):
        for action in ("alter", "remove", "header"):
            with self.subTest(action=action):
                self.store = self.root / action
                self.ingest(self.fixture())
                _, _, manifest = self.manifest()
                row = next(row for row in manifest["entries"] if row["type"] == "file")
                path = archive.blob_path(self.store, row["sha256"])
                if action == "remove":
                    path.unlink()
                else:
                    raw = bytearray(path.read_bytes())
                    raw[4 if action == "header" else -1] ^= 1
                    path.write_bytes(raw)
                with self.assertRaises((ValueError, FileNotFoundError)):
                    self.replay()
                self.assertFalse((self.root / "restored").exists())
                self.assertFalse(list(self.root.glob(".archive-restore-*")))

    def test_raw_hash_checked_even_if_compressed_identity_is_rebound(self):
        self.ingest(self.fixture())
        _, _, manifest = self.manifest()
        row = next(row for row in manifest["entries"] if row["type"] == "file")
        path = archive.blob_path(self.store, row["sha256"])
        raw = io.BytesIO()
        with archive.gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as handle:
            handle.write(b"wrong")
        path.write_bytes(raw.getvalue())
        row["gzip_sha256"] = archive.digest(raw.getvalue())
        row["gzip_bytes"] = len(raw.getvalue())
        self.replace_manifest(manifest)
        with self.assertRaises(ValueError):
            self.replay()

    def test_manifest_damage_and_incomplete_manifest_rejected(self):
        self.ingest(self.fixture())
        _, path, value = self.manifest()
        original = path.read_bytes()
        path.write_bytes(original + b" ")
        with self.assertRaisesRegex(ValueError, "manifest identity"):
            self.replay()
        path.write_bytes(original)
        value["entries"].pop()
        self.replace_manifest(value)
        with self.assertRaisesRegex(ValueError, "incomplete manifest"):
            self.replay()

    def test_rebound_manifest_cannot_escape_output(self):
        self.ingest(self.fixture())
        _, _, value = self.manifest()
        value["entries"][0]["path"] = "../outside"
        self.replace_manifest(value)
        with self.assertRaises(ValueError):
            self.replay()
        self.assertFalse((self.root / "outside").exists())

    def test_missing_metadata_and_wrong_index_rejected(self):
        result = self.ingest(self.fixture())
        with self.assertRaisesRegex(ValueError, "index identity"):
            archive.restore(self.store, self.root / "restored", "0" * 64)
        next((self.store / "metadata").iterdir()).unlink()
        with self.assertRaises(FileNotFoundError):
            archive.restore(self.store, self.root / "restored", result["index_sha256"])

    def test_duplicate_artifacts_and_cross_run_import_preserve_index(self):
        item = self.fixture()
        self.ingest(item)
        before = (self.store / "index.json").read_bytes()
        for other in (item, self.fixture(name="different", run=13), self.fixture(name="different", head="b" * 40)):
            with self.assertRaises(ValueError):
                self.ingest(other)
            self.assertEqual((self.store / "index.json").read_bytes(), before)

    def test_existing_output_and_store_symlinks_are_rejected(self):
        self.ingest(self.fixture())
        output, _ = self.replay()
        before = sorted(str(p) for p in output.rglob("*"))
        with self.assertRaisesRegex(ValueError, "already exists"):
            self.replay()
        self.assertEqual(sorted(str(p) for p in output.rglob("*")), before)
        target = self.root / "elsewhere"
        target.mkdir()
        link = self.root / "link"
        try:
            link.symlink_to(target, target_is_directory=True)
        except OSError as error:
            self.skipTest(f"symlink creation unavailable: {error}")
        self.store = link
        with self.assertRaises(ValueError):
            self.ingest(self.fixture())

    def test_blob_symlink_is_rejected_without_publishing_output(self):
        self.ingest(self.fixture())
        blob = next((self.store / "blobs").iterdir())
        target = self.root / "elsewhere.gz"
        blob.rename(target)
        try:
            blob.symlink_to(target)
        except OSError as error:
            self.skipTest(f"symlink creation unavailable: {error}")
        with self.assertRaisesRegex(ValueError, "not an ordinary file"):
            self.replay()
        self.assertFalse((self.root / "restored").exists())

    def test_cli_roundtrip_and_nonzero_failure(self):
        item = self.fixture()
        command = [sys.executable, *(["-O"] if sys.flags.optimize else []), str(HERE / "archive.py")]
        added = subprocess.run(command + ["import", "--zip", str(item[0]), "--metadata", str(item[1]), "--store", str(self.store), "--run-id", "12", "--head-sha", HEAD, "--name", item[2]["name"]], capture_output=True, text=True)
        self.assertEqual(added.returncode, 0, added.stderr)
        copied = subprocess.run(command + ["restore", "--store", str(self.store), "--output", str(self.root / "cli-restored"), "--index-sha256", json.loads(added.stdout)["index_sha256"]], capture_output=True, text=True)
        self.assertEqual(copied.returncode, 0, copied.stderr)
        failed = subprocess.run(command + ["restore", "--store", str(self.store), "--output", str(self.root / "missing"), "--index-sha256", "0" * 64], capture_output=True, text=True)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("index identity mismatch", failed.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
