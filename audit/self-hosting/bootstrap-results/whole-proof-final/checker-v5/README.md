# V5 final checker preparation

This packet preserves the independently reviewed checker before actual V5
acceptance. The 29-file checker manifest is
[9c56357c798c6e3e7727c2f55991e66c69b243b88340f7a334461f56fbdc4584](objects/9c56357c798c6e3e7727c2f55991e66c69b243b88340f7a334461f56fbdc4584).
The [root gate](objects/eeaba901de02584018eb0d4b5f4ceac978c738349b0d4ff2468a9f58f2cf4897)
and [peer review](objects/01049a7695f58545dbd9caa7b15993e995c4fa8dc4a6d1ce5817a414e412e601)
bind those exact sources. This archive was prepared in the Trident helper
worktree at revision be44748a9b4568d63a19e2f341c25c01d75654eb; original
experiment paths identify the measured family.

Author, root and peer independently passed 80 offline tests against this source
packet. The peer also passed eight independent synthetic schedule probes and
rehashed all 547 pinned originals. The retained receipts record exact commands,
source identities, raw stdout/stderr and the individual outcomes. The primary
test command in the original checker directory was:

    python3.14 -B -W error -m unittest -v test_checker test_native test_selected test_schedule_replay

The separate retained-data preparation replay used:

    python3.14 -B -W error replay_retained_preparation.py retained-preparation-1

It passed the original failed V2/V3 histories, completed prefix transition and
failed V4 restricted schedule in 17.664607416954823 seconds. This partial replay
does not run the V5 completion checker. Its exact production sources remain
identical to the final reviewed packet.

Final acceptance still requires both actual V5 generations and the sequential
coordinator to pass. The checker then authenticates the original positive proofs
and extracted-compiler corpora, two historical controls per generation, nine V2
and three V3 rejections per generation, eleven fresh V5 cases for C1, and one
selected completed V4 cost rejection plus ten fresh V5 cases for C2. It replays
42 native command registrations and 23 physical-capacity permits with the
unchanged resource ceilings. All failed enclosing histories keep their original
statuses. No native result or SH8 acceptance is claimed by this preparation.

The preserved 6ce30 source candidate and its passing test receipt remain here.
Root review found that two fixtures contained literal backslash-n text and
reached malformed-input parsing. The final candidate changes only those two
fixtures and requires their intended errors. The exact source delta, corrected
LF probes and new test receipts are retained. The earlier F4 preparation also
remains unchanged; its BSD birth-padding parser finding was fixed only in F5
and exercised by the actual partial prefix replay.

[retained-files.json](retained-files.json) maps 165 original files, totaling
1,307,751 bytes, to 59 unique SHA256 objects totaling 479,004 bytes. Every object
preserves the original bytes. Five exact inherited source objects require only
a blank-at-EOF whitespace exception; all objects use -text to preserve their
identities across native checkouts. The map retains duplicated provenance paths
without duplicating identical stored bytes.

Private full-host snapshots and selected historical objects containing nested
raw_stdout remain local. Their source-pin references are retained without
copying those payloads. This object archive describes original experiment paths;
it is not a relocated executable checkout. Verify its stored bytes with:

    python3 -B -W error verify-retained.py

Use --originals only where the original local paths still exist. Actual V5
acceptance, durable proof transfer, release integration and owner publication
have separate evidence and decisions.
