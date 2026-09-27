# Raw CI artifact retention

`archive.py` stores complete uncompressed GitHub artifact file trees in a
shared content-addressed store. Each distinct raw file, including the empty
file, has one deterministic gzip blob. Original API JSON bytes and a complete
path manifest retain the download identity. This is storage and replay;
compiler acceptance remains the existing runner's responsibility.

Import requires the expected run ID, commit and artifact name. The original
ZIP's size and SHA-256 must match the API metadata before ZIP parsing begins.
The ZIP download remains immutable outside this store. The original ZIP
container cannot be recreated from this representation.

```sh
python3 audit/self-hosting/bootstrap-results/archive-tool/archive.py import \
  --zip /absolute/download.zip --metadata /absolute/artifact-api.json \
  --run-id RUN_ID --head-sha COMMIT --name ARTIFACT_NAME --store STORE
python3 audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store STORE --output FRESH_DIRECTORY --index-sha256 RETAINED_INDEX_SHA256
```

Each import prints the resulting index identity. Preserve that identity with
the reviewed Git revision or an independent receipt. Hashes detect damage;
the locally supplied API JSON is not an authenticated GitHub response by
itself. Restore requires the retained index hash, verifies every manifest,
metadata file and compressed/raw blob, then publishes a fresh directory
containing one root per original artifact name. Pass the twelve restored
roots to the unchanged runner's `--matrix` command. The tool does not decide
whether twelve artifacts constitute the required platforms or repeats.

One store admits at most twelve artifacts from one run and commit. Import
rejects duplicate artifact names or IDs. Each ZIP is bounded to 4 GiB of
compressed data, 50,000 members and 4 GiB of uncompressed data; metadata is
bounded to 1 MiB and generated manifests to 32 MiB. Portable relative paths
are at most 1,024 UTF-8 bytes.
Duplicate paths, case/normalization aliases, file/directory conflicts,
symlinks, special files, encrypted entries and path escapes are rejected.
Directory entries are retained; restored files are ordinary files. File
contents, including line endings, are copied without interpretation.
Reserved Windows names follow the
[Microsoft file-naming rules](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file).

Imports serialize through an exclusive store lock. Blob publication precedes
the atomic index update. A failed import can leave unreferenced blobs, but
cannot publish a partial artifact. Restore stages all artifacts before
publication and rejects an existing destination. It never overwrites source
ZIPs, metadata, stored manifests or blobs. Treat the store as immutable while
restoring; hostile concurrent filesystem mutation is outside this local
archival tool's contract.

The store layout is `index.json`, `manifests/<manifest-sha256>.json`,
`metadata/<raw-api-sha256>.json`, and `blobs/<raw-file-sha256>.gz`. The index
binds artifact names and IDs to manifests; each manifest binds provenance,
member count, total bytes, paths, and both compressed and raw identities.
New gzip blobs use level 9, an empty filename and timestamp zero. Repeated
imports with the same Python/zlib implementation produce identical stored
bytes. When a raw hash already exists, import verifies and reuses its exact
stored encoding, so a different compressor version need not reproduce it.
Restore verifies the compressed identity recorded in each manifest.

Run the focused guards with:

```sh
python3 audit/self-hosting/bootstrap-results/archive-tool/test_archive.py
python3 -O audit/self-hosting/bootstrap-results/archive-tool/test_archive.py
```

Local verification used base revision
`027ed19bc4ebf92e9f3085559184135d6e993d59` plus `archive.py` SHA-256
`acef178c58f48005c30ed7a300f84fc5ed8c5ab0ddcf498b318bb74ec2ec4e4e`.
Both test commands passed all 19 guards. The import/restore commands above
were also run against the original Intel artifact: run `36353842247`, head
`23691cd2c6885bf25bfc023799552559724dbc2b`, artifact ID `10943644187`, name
`bootstrap-x86_64-apple-darwin`. Its original ZIP is 8,537,516 bytes with
SHA-256 `2fed4c7afe122895e744edd09cab63e03de4fa9a92d81fa4c81763c0d22defdd`.

All 356 files (17,906,765 raw bytes) and 14 directories matched the original
independently extracted tree; every file also matched its ZIP member byte
for byte. The store contains 136 distinct gzip blobs totaling 8,371,959
bytes, or 8,483,441 bytes including index, manifest and exact API metadata.
The original ZIP and metadata remained unchanged. These are measured
single-artifact sizes, with no projection for future CI bundles.

Exact command arrays, raw logs, every file identity and the independent
comparison result are retained locally in
`../measurements/bootstrap-archive-final/verification.json`, SHA-256
`7decd10fa4a302912fc0233fb5d29f99d742af065f3cc6c3aca8c850b3a22ecc`.
The final index is
`7b8251b7a95522fff85bd61b60da0219c1c773e7024b3c40e5d8a5a0e1625fd5`.
That Intel bundle still records its original compiler deadline failure;
successful archival does not change its result.
