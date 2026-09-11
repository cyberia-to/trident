# ⌨️ CLI Reference

[← Language Reference](language.md)

---

```nu
# Build
trident build <file>                    # Compile to target assembly
trident build <file> --target neptune   # OS target → derives TRITON
trident build <file> --target ethereum  # OS target → derives EVM
trident build <file> --target linux     # OS target → derives x86-64
trident build <file> --target triton    # Bare VM target (no OS)
trident build <file> --target miden     # Bare VM → .masm
trident build <file> --engine triton    # VM target (geeky register)
trident build <file> --terrain triton   # VM target (gamy register)
trident build <file> --network neptune  # OS target (geeky register)
trident build <file> --union neptune    # OS target (gamy register)
trident build <file> --costs            # Print cost analysis
trident build <file> --hotspots         # Top cost contributors
trident build <file> --hints            # Optimization hints (H0001-H0004)
trident build <file> --annotate         # Per-line cost annotations
trident build <file> --save-costs <json>  # Save cost report to JSON
trident build <file> --compare <json>   # Compare against baseline costs
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

# Docs
trident doc <file>                      # Generate documentation
trident doc <file> -o <docs.md>         # Generate to file
trident doc <file> --engine triton      # VM target (geeky register)
trident doc <file> --terrain triton     # VM target (gamy register)
trident doc <file> --network neptune    # OS target (geeky register)
trident doc <file> --union neptune      # OS target (gamy register)

# Package
trident package <file>                  # Compile + hash + produce .deploy/ artifact
trident package <file> --target neptune # Package for specific OS/VM target
trident package <file> --engine triton    # VM target (geeky register)
trident package <file> --terrain triton   # VM target (gamy register)
trident package <file> --network neptune  # OS target (geeky register)
trident package <file> --union neptune    # OS target (gamy register)
trident package <file> --vimputer main    # Chain instance (geeky register)
trident package <file> --state main       # Chain instance (gamy register)
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
trident run <file> --vimputer main      # Chain instance (geeky register)
trident run <file> --state main         # Chain instance (gamy register)
trident run <file> --input-values 1,2,3 # Public input field elements
trident run <file> --secret 42          # Secret/divine input values

# Prove (delegates to warrior)
trident prove <file>                    # Compile and generate proof via warrior
trident prove <file> --target neptune   # Prove on specific target
trident prove <file> --engine triton    # VM target (geeky register)
trident prove <file> --terrain triton   # VM target (gamy register)
trident prove <file> --network neptune  # OS target (geeky register)
trident prove <file> --union neptune    # OS target (gamy register)
trident prove <file> --vimputer main    # Chain instance (geeky register)
trident prove <file> --state main       # Chain instance (gamy register)
trident prove <file> --output proof.bin # Write proof to file
trident prove <file> --input-values 1,2 # Public input for proof

# Verify (delegates to warrior)
trident verify <proof>                  # Verify a proof via warrior
trident verify <proof> --target neptune # Verify against target
trident verify <proof> --engine triton    # VM target (geeky register)
trident verify <proof> --terrain triton   # VM target (gamy register)
trident verify <proof> --network neptune  # OS target (geeky register)
trident verify <proof> --union neptune    # OS target (gamy register)
trident verify <proof> --vimputer main    # Chain instance (geeky register)
trident verify <proof> --state main       # Chain instance (gamy register)

# Deploy
trident deploy <file>                   # Compile, package, deploy to registry
trident deploy <dir>.deploy/            # Deploy pre-packaged artifact
trident deploy <file> --engine triton    # VM target (geeky register)
trident deploy <file> --terrain triton   # VM target (gamy register)
trident deploy <file> --network neptune  # OS target (geeky register)
trident deploy <file> --union neptune    # OS target (gamy register)
trident deploy <file> --vimputer main    # Chain instance (geeky register)
trident deploy <file> --state main       # Chain instance (gamy register)
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

---

## Three-Register Flags

Trident uses a **three-register** naming model for targets. Each register
has two synonyms — one *geeky* (technical) and one *gamy* (metaphorical) —
plus a *universal* shorthand for backward compatibility.

| Register | Geeky | Gamy | Universal | Resolves |
|----------|-------|------|-----------|----------|
| **VM** | `--engine <name>` | `--terrain <name>` | `--target <name>` | Which VM to compile for |
| **OS** | `--network <name>` | `--union <name>` | `--target <name>` | Which OS layer to bind |
| **Chain** | `--vimputer <name>` | `--state <name>` | *(deploy only)* | Which chain instance to deploy to |

**Resolution rules:**

- `--target <name>` is the universal shorthand. It resolves to a VM, an OS,
  or both (an OS implies its underlying VM). This flag is always accepted
  and remains the recommended default for simple cases.
- The geeky and gamy names are interchangeable — `--engine triton` and
  `--terrain triton` mean the same thing. Choose whichever register
  vocabulary your team prefers.
- When both VM and OS registers are provided, the OS's declared VM must
  match the explicit VM register (or an error is raised).
- The chain register (`--vimputer` / `--state`) is only available on
  deployment commands (`deploy`, `package`, `run`, `prove`, `verify`).
  It selects a specific chain instance within the resolved OS.

**Compilation commands** (`build`, `check`, `test`, `doc`) accept the
VM and OS registers (4 flags). **Deployment commands** (`deploy`,
`package`, `run`, `prove`, `verify`) accept all three registers (6 flags).

---

### Warrior Discovery

Trident is the weapon. **Warriors** wield it on specific battlefields.

`run`, `prove`, and `verify` delegate to external **warrior** binaries.
Each warrior is specialized for a target VM+OS combination, bringing the
heavy dependencies (provers, VMs, chain clients) that Trident stays clean of.

Resolution order for finding a warrior:

1. Look for `trident-<target>` on PATH
2. Check the target's `[warrior]` config in `vm/<target>/target.toml`
3. If target is an OS, check the underlying VM's warrior config

If no warrior is found, Trident compiles the program and prints installation
guidance. Warriors are installed separately (e.g. `cargo install trident-trisha`).

### Target Resolution

`--target <name>` (universal register) resolves as:

1. Is `<name>` an OS? → load `UnionConfig` from `os/<name>/target.toml`, derive VM from `vm` field
2. Is `<name>` a VM? → load `TerrainConfig` from `vm/<name>/target.toml`, no OS (bare compilation)
3. Neither → error: unknown target

When explicit registers are used instead of `--target`:

1. `--engine <name>` / `--terrain <name>` → load `TerrainConfig` from `vm/<name>/target.toml`
2. `--network <name>` / `--union <name>` → load `UnionConfig` from `os/<name>/target.toml`
3. `--vimputer <name>` / `--state <name>` → select chain instance within the resolved OS
4. If both VM and OS registers are given, the OS's declared `vm` field must match the VM register

See [targets.md](targets.md) for the full target registry.

---

## 🔗 See Also

- [Language Reference](language.md) — Types, operators, builtins, grammar, sponge, Merkle, extension field, proof composition
- [Standard Library](stdlib.md) — `std.*` modules
- [Grammar](grammar.md) — EBNF grammar
- [OS Reference](os.md) — OS concepts, `os.*` gold standard, extensions
- [Target Reference](targets.md) — All VMs and OSes
