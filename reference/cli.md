# ⌨️ CLI Reference

[← Language Reference](language.md)

---

```nu
# Build
trident build <file>                    # Compile to nox (default)
trident build <file> --target nox       # Explicit nox reference ABI
trident build <file> --target cyber     # nox package described by Joy
trident build <file> --target neptune   # Delegate to installed Trisha
trident build <file> --target triton    # Delegate to installed Trisha
trident build <file> --engine triton    # VM target (geeky register)
trident build <file> --terrain triton   # VM target (gamy register)
trident build <file> --network neptune  # OS target (geeky register)
trident build <file> --union neptune    # OS target (gamy register)
trident build <file> --costs            # Print cost analysis
trident build <file> --profile release # Select compilation cfg profile
trident build <file> -o <out>           # Custom output path

# Check
trident check <file>                    # Type-check only
trident check <file> --costs            # Type-check + cost analysis
trident check <file> --engine triton    # VM target (geeky register)
trident check <file> --terrain triton   # VM target (gamy register)
trident check <file> --network neptune  # OS target (geeky register)
trident check <file> --union neptune    # OS target (gamy register)

# Format
trident fmt <file>                      # Format in place
trident fmt <dir>/                      # Format all .tri in directory
trident fmt <file> --check              # Check only (exit 1 if unformatted)

# Test
trident test <file>                     # Run #[test] functions
trident test <file> --engine triton     # VM target (geeky register)
trident test <file> --terrain triton    # VM target (gamy register)
trident test <file> --network neptune   # OS target (geeky register)
trident test <file> --union neptune     # OS target (gamy register)

# Audit
trident audit <file>                    # Verify #[requires]/#[ensures]
trident audit <file> --z3              # Formal verification via Z3

# Package
trident package <file>                  # Compile + hash + produce .deploy/ artifact
trident package <file> --target neptune # Package for specific OS/VM target
trident package <file> --engine triton    # VM target (geeky register)
trident package <file> --terrain triton   # VM target (gamy register)
trident package <file> --network neptune  # OS target (geeky register)
trident package <file> --union neptune    # OS target (gamy register)
trident package <file> --vimputer main  # Rejected: state-specific packaging unavailable
trident package <file> --state main     # Same unsupported state selection
trident package <file> -o <dir>         # Output to custom directory
trident package <file> --audit          # Run verification before packaging
trident package <file> --dry-run        # Show what would be produced

# Run (delegates to warrior)
trident run <file>                      # Compile and run via warrior
trident run <file> --target neptune     # Run on specific target
trident run <file> --engine triton      # VM target (geeky register)
trident run <file> --terrain triton     # VM target (gamy register)
trident run <file> --network neptune    # OS target (geeky register)
trident run <file> --union neptune      # OS target (gamy register)
trident run <file> --vimputer state.json  # State input interpreted by the warrior
trident run <file> --state state.json  # State input interpreted by the warrior
trident run <file> --input-values 1,2,3 # Public input field elements
trident run <file> --secret 42          # Secret/divine input values

# Prove (delegates to warrior)
trident prove <file>                    # Compile and generate proof via warrior
trident prove <file> --target neptune   # Prove on specific target
trident prove <file> --engine triton    # VM target (geeky register)
trident prove <file> --terrain triton   # VM target (gamy register)
trident prove <file> --network neptune  # OS target (geeky register)
trident prove <file> --union neptune    # OS target (gamy register)
trident prove <file> --vimputer state.json  # State input interpreted by the warrior
trident prove <file> --state state.json  # State input interpreted by the warrior
trident prove <file> --output proof.bin # Write proof to file
trident prove <file> --input-values 1,2 # Public input for proof

# Verify (delegates to warrior)
trident verify <proof>                  # Verify a proof via warrior
trident verify <proof> --target neptune # Verify against target
trident verify <proof> --engine triton    # VM target (geeky register)
trident verify <proof> --terrain triton   # VM target (gamy register)
trident verify <proof> --network neptune  # OS target (geeky register)
trident verify <proof> --union neptune    # OS target (gamy register)
trident verify <proof> --vimputer state.json  # State input interpreted by the warrior
trident verify <proof> --state state.json  # State input interpreted by the warrior

# Deploy
trident deploy <file>                   # Compile, package, deploy to registry
trident deploy <dir>.deploy/            # Deploy pre-packaged artifact
trident deploy <file> --engine triton    # VM target (geeky register)
trident deploy <file> --terrain triton   # VM target (gamy register)
trident deploy <file> --network neptune  # OS target (geeky register)
trident deploy <file> --union neptune    # OS target (gamy register)
trident deploy <file> --vimputer main   # Rejected: registry publication has no state selection
trident deploy <file> --state main      # Same unsupported state selection
trident deploy <file> --registry <url>  # Deploy to specific registry
trident deploy <file> --audit           # Audit before deploying
trident deploy <file> --dry-run         # Show what would be deployed

# Hash
trident hash <file>                     # Show function content hashes
trident hash <file> --full              # Show full 256-bit hashes

# View
trident view <name>                     # View a function definition
trident view <name> -i <file>           # From specific file

# Equivalence
trident equiv <file> <fn_a> <fn_b>      # Check two functions are equivalent

# Benchmarks
trisha bench <dir>                      # Warrior-owned verified Triton benchmarks

# Store (definitions store)
trident store add <file>                # Add definitions to codebase
trident store list                      # List all definitions
trident store lookup <hash>             # Find definition by hash
trident store diff <file>               # Show changed definitions

# Atlas (Package Registry)
trident atlas publish                # Publish definitions to Atlas
trident atlas pull <hash|name>       # Pull definition by hash or name
trident atlas search <query>         # Search definitions
trident atlas serve                  # Start local Atlas server
# Dependencies
trident deps list                       # Show declared dependencies
trident deps lock                       # Lock dependency versions
trident deps fetch                      # Download locked dependencies

# Project
trident init <name>                     # Create new program project
trident init --lib <name>               # Create new library project
trident generate <spec.tri>             # Generate scaffold from spec
trident lsp                             # Start LSP server
```

`build` accepts a source file or a directory containing `trident.toml`. A
directory selects the manifest entry; an explicit source file remains the
entry while inheriting project settings. Target selection uses the explicit
flag, then the project target, then `nox`. Output defaults to the selected
target's extension; `--costs` reports nox reductions or requests the installed
stack warrior's cost report. Catalog declarations alone do not implement a
compiler backend. Triton/Neptune require the `external-targets` feature;
builds also require an installed Trisha provider. Discovery accepts that
provider's descriptor or an explicit offline descriptor.

For compilation by the native self-built compiler, use Joy's `pack-job` and
`run-artifact` commands described in the
[self-built compiler guide](../docs/guides/self-hosted-compilation.md).

`trident check` prints compiler diagnostics to stderr and exits with status1
when source discovery, parsing or type checking fails. Successful checks print
`OK: <input>` and exit with status0.

Compiler filesystem inputs have per-file UTF-8 transport limits: 4 MiB for
source entries and imported modules, and 1 MiB for project manifests and
dependency lockfiles. Inputs must resolve to regular files; ordinary source
symlinks remain supported. Reads keep their byte cap if a file grows. On macOS
and Linux x86_64/aarch64, nonblocking opens and descriptor identity checks also
reject replacement by a stream or another file during admission. These bounds
do not establish an overall compilation time or aggregate source-package limit.

---

## Three-Register Flags

Trident uses a **three-register** naming model for targets. Each register
has two synonyms — one *geeky* (technical) and one *gamy* (metaphorical) —
plus a *universal* shorthand for backward compatibility.

| Register | Geeky | Gamy | Universal | Resolves |
|----------|-------|------|-----------|----------|
| **VM** | `--engine <name>` | `--terrain <name>` | `--target <name>` | Which VM to compile for |
| **OS** | `--network <name>` | `--union <name>` | `--target <name>` | Which OS layer to bind |
| **State** | `--vimputer <value>` | `--state <value>` | — | State selection passed to the warrior |

**Resolution rules:**

- `--target <name>` is the universal shorthand. It resolves to a VM, an OS,
  or both (an OS implies its underlying VM). This flag is always accepted
  and remains the recommended default for simple cases.
- The geeky and gamy names are interchangeable — `--engine triton` and
  `--terrain triton` mean the same thing. Choose whichever register
  vocabulary your team prefers.
- Choose one of `--engine`, `--terrain`, `--network` or `--union` per command;
  these flags are mutually exclusive. A supplied register flag overrides
  `--target`.
- The state register (`--vimputer` / `--state`) is exposed by `deploy`,
  `package`, `run`, `prove` and `verify`. Runtime commands pass the selection
  to the warrior, which owns its meaning and validation. `package` and
  `deploy` currently reject explicit state selection.

**Compilation commands** (`build`, `check`, `test`) accept the
VM and OS registers (4 flags). The other commands listed above expose
the state flags too, subject to their command-specific support.

---

### Warrior Discovery

Trident is the weapon. **Warriors** wield it on specific battlefields.

`run`, `prove`, `verify`, and stack-target `build` delegate to external warrior binaries.
Each warrior is specialized for a target VM+OS combination, bringing the
heavy dependencies (provers, VMs, chain clients) that Trident stays clean of.

The registered owner is Joy for `nox`/`cyber` and Trisha for
`triton`/`neptune` when external targets are enabled. Executable discovery
searches PATH for `trident-<target>`, then `trident-<owner>`, then `<owner>`
(including the platform executable suffix). The same provider supplies
`describe --target <name>` and receives delegated commands.

Target packages declare supported commands. An explicit
`TRIDENT_TARGET_PACKAGES` directory supplies bounded JSON descriptors for
offline discovery; delegation additionally checks the installed provider's
compilation identity against that package. A missing provider or unsupported
command fails with a nonzero exit. Joy installation guidance names
`cargo install cyber-joy`; Trisha guidance points to its release artifacts.

### Target Resolution

`nox` has a built-in reference terrain ABI. Other registered targets obtain
terrain and optional union data from their owner's validated target package.
Unregistered catalog entries cannot provide an executable implementation.
The selected name follows the register precedence above.

`run`, `prove` and `verify` forward a selected state value to the installed
provider. For example, Joy's state input is a JSON file path on its supported
nox/cyber targets. Use the provider's command contract for that input.

See [targets.md](targets.md) for the full target registry.

---

## 🔗 See Also

- [Language Reference](language.md) — Types, operators, builtins, grammar, sponge, Merkle, extension field, proof composition
- [Standard Library](stdlib.md) — `std.*` modules
- [Grammar](grammar.md) — EBNF grammar
- [OS Reference](os.md) — OS concepts, `os.*` gold standard, extensions
- [Target Reference](targets.md) — All VMs and OSes
