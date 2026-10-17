# Build documentation surface check

The installed Trident at `82fdd1fd45fd14f3ac8da60a53d9e1a3e675c18e` supplied
the command evidence in `receipt.json.gz`. That receipt records the binary
SHA-256, source hashes and exact argv, working directory, exit status and
output for all ten commands. `files.json` binds the original source, emitted
programs and losslessly compressed receipt.

Ordinary nox builds with cost reporting and an explicit release profile passed.
The obsolete `--hotspots`, `--hints`, `--annotate`, `--save-costs` and `--compare`
build flags reject. Combining engine and network selectors rejects; `doc` is
absent from the current command enum and rejects as well. These observations
correct the build guide and matching canonical CLI reference.

Provider ownership, executable lookup order, descriptor validation and command
dispatch were checked in the pinned `src/config/target/discover.rs`,
`src/config/target/{mod,os}.rs` and `src/cli/{mod,build}.rs`. This was a source
inspection; it adds no external-warrior execution, bootstrap or proof acceptance.

Review also corrected two distinctions: offline descriptors support discovery
without an installed provider, while delegated builds require one; and CLI
state arguments are provider-defined inputs. `state-source-inspection.json.gz`
pins the exact additional Trident/Joy source reads showing runtime forwarding,
Joy's JSON-certificate paths, and package/deploy rejection of state selection.
