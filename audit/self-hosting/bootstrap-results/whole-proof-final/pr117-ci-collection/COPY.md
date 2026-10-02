# Evidence-only copy plan

Copy this complete external staging directory, preserving exact file bytes and relative paths, into a new helper audit directory such as `audit/self-hosting/bootstrap-results/whole-proof-final/pr117-ci-collection/`. Root selects the final integration path and owns the commit. No active collector, earlier failed scope, source gate, proof payload or original native archive is modified by this copy.

Before and after copying, compare every relative path, byte count and SHA256 against `delivery-files.json`. That manifest lists every file except itself, with no symlinks. Preserve `.gitattributes` (`* -text`) and confirm tracked paths stay within the repository's 200 UTF-16-unit Windows path limit. The manifest is bound externally by the preparation receipt and handoff; it does not contain its own identity.

Run `python3 -B -W error verify-retained.py` from the copied directory. `--originals` additionally rehashes the selected small originals at their recorded absolute paths on this host. Both commands execute only the unchanged generic verifier; source and command files in `objects/` remain data. `static-1/` and `intermediate-result-1/` preserve the earlier manifest versions and their original status. The final map is the complete object membership for this delivery.

Keep complete original ZIPs, decoded native evidence, inherited executable inputs and private raw HTTP headers at their original retained locations. Their identities and source receipts are public; their bodies are excluded from this audit copy. This integration records completed native CI evidence and its reviewed merge. It does not execute a compiler, rerun CI or decide SH8 full-certificate acceptance.

Use this v2 directory in place of the rejected first delivery. Preserve the exact three object-specific whitespace attributes and the compiled-cache reference. The isolated Git staging check covers these raw bytes without normalizing them. The prior delivery stays unchanged at its original path and its manifests remain superseded evidence.
