# Preserve exact evidence bytes under Git CRLF conversion

At commit `5b68e00af9d0e41f8a181471a74e8b01f174f1d9`, actual
`git -c core.autocrlf=true cat-file --filters HEAD:<path>` changed the result
inventory from SHA-256 `566b8fb950c84f629ef5a47a9eb5eb653e600f78769af78936ec796ce7ae2a5b`
to `ddbfd35e42b3bde9aebb3ca8103ad1a52b6868e5c522fd1c936f5afba4defece`.
The neighboring prepared inventory changed from
`7fb109ee6c3d23a0dd6d08de94e966ab41c74fb4a8843cde779d57fbb3b11357` to
`c553b48c84b82994198755f341895b6db1a98bf6c664104a447c4b1f3810c6ac`.
The actual commands, original/filtered sizes and hashes are retained in
[before.json](before.json), with complete filtered output bytes in its gzip files.

The result directory now sets `* -text`. One exact path rule in the repository
attributes preserves the neighboring `prepared-files.json`. The neighboring
attributes file already preserves `prepared/**`; it remains byte-identical
because the earlier review binds its exact contents. These rules preserve
canonical bytes during checkout and leave whitespace checks enabled.

[The checkout checker](check_checkout.py) exercises real Git filters for every
tracked byte-bound input of the result checker and the result delivery inventory,
plus all twelve prepared source/review copies. It materializes the filtered
files, reproduces both original inventory failures, restores the protected
bytes, then runs the result checker, the earlier failure-delivery checker and
sixteen adversarial tests. Its source-impact inventory compares every tracked
file outside this audit directory and the one explicitly reviewed root
attributes file. Compiler/runtime sources and earlier measured evidence remain
unchanged. This exercises Git's CRLF conversion on macOS; it does not claim a
new native Windows runtime run.
