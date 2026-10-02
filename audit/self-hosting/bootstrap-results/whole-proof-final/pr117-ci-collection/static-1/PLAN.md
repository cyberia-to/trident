# PR117 v4 evidence delivery preparation

This private staging scope retains exact reviewed collector source, tests, preparation, superseded source packets, raw test outcomes and root/peer gates while original collection runs. It contains no new acceptance result. The collector remains frozen and owns all active HTTP work.

Original v1/v2/v3 scopes, complete native ZIPs, decoded artifacts, private HTTP headers, credentials and signed redirect responses are never copied into delivery. Hash-bound references in retained source maps and completed receipts preserve their provenance. Every private raw header stays local; public delivery includes only its path, byte count and SHA256. The inherited large evidence and executable files stay reference-only.

After actual collection and owner/peer matrix replay pass, extend this separate staging manifest with final wrapper, receipt/accounting, exact raw outer streams and independent result receipts. Validate every retained object with the unchanged generic verify-retained.py, including --originals. Then provide the packet to root for evidence-only helper delivery; no helper or active collector scope is modified here. Original failures remain failures and superseded preparations remain preparations. No SH8 proof acceptance is inferred from original native CI matrix replay.
