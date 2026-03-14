---
status: draft
author: mastercyb
---

# Noun Types: Why Nox Drops cell? and What Trident Does Instead

## Problem

Nock op 3 tests whether a noun is an atom or a cell at runtime:

```
*[a 3 b]  ->  0 if *[a b] is a cell
              1 if *[a b] is an atom
```

Nox drops this. The question is whether to add it back.

## Why not

A STARK trace is a fixed-width table of field elements. Every nox
pattern maps to a polynomial transition constraint selected by a
4-bit tag. `cell?` breaks this model in one specific way: atoms and
cells have structurally different representations. An atom is a
Goldilocks field element. A cell is a pointer into the noun DAG.
The distinction between them is not a field predicate — it is a type
judgment that requires a separate witness column in the AIR trace.

That column either:

- adds prover cost and proof size for a predicate used rarely, or
- must be known statically anyway — in which case the runtime check
  is dead weight

Nock needs `cell?` because Hoon compiles through Nock at runtime:
the evaluator is the runtime. Nox's evaluator is the proof system.
Dynamic type dispatch at the VM level defeats static arithmetization.

**Rule:** if a predicate is always a compile-time constant in the
intended source language, it does not belong in the VM.

## What Trident does instead

The expressiveness that `cell?` provides in Nock comes from one
pattern: dispatching on unknown noun structure. Trident recovers
this with noun types — discriminated unions over noun structure,
resolved statically.

### Noun type syntax

```trident
type Noun =
  | Atom  of Field        -- a single Goldilocks element
  | Word  of u64          -- bitwise-domain atom
  | Hash  of [Field; 4]   -- 256-bit identity
  | Pair  of Noun * Noun  -- a cell
```

A value of type Noun carries its structure in the type, not at
runtime. The compiler knows at every call site which branch is live.

### Pattern match syntax

```trident
match expr {
  Atom f      => ...
  Word w      => ...
  Hash h      => ...
  Pair(l, r)  => ...
}
```

Matches must be exhaustive. The compiler rejects missing branches.
Each arm compiles to a branch chain in nox with statically known
offsets.

### Generic traversal

Functions over unknown structure use Noun as a type parameter:

```trident
fn depth(n: Noun) -> Field {
  match n {
    Atom _       => 0
    Word _       => 0
    Hash _       => 0
    Pair(l, r)   => 1 + max(depth(l), depth(r))
  }
}
```

The recursion is bounded by the type structure, not by a runtime
tag check. The compiler inlines or specializes based on call-site
types.

### What this compiles to

```nox
-- match n { Atom f => e1, Pair(l,r) => e2 }
-- compiles to: branch on static tag, then axis into the pair

[4 [axis(n, tag_slot)] [e1] [e2]]
```

No extra witness column. No runtime type oracle. The branch is a
standard nox op 4, driven by a field element the compiler placed
at a known axis.

## Design tension

`language.md` currently says "No enums. No sum types." The Noun
type is a discriminated union — a sum type. Two paths:

**Path A: Noun as a built-in type.** Keep the "no sum types" rule
for user code. Make Noun a compiler-intrinsic type like Field or
Digest. Match syntax works only on Noun. Minimal surface change,
but one-off magic.

**Path B: Add sum types to the language.** Remove the "no enums"
restriction. Noun becomes a library type defined in `vm/nock/`.
Orthogonal and composable, but every sum type needs a tag —
what does that cost in the AIR trace on stack targets?

Leaning Path A until there's evidence other targets benefit from
general sum types.

## Summary

| Need | Nock solution | Nox + Trident solution |
|------|---------------|------------------------|
| Atom vs cell dispatch | `cell?` op at runtime | Exhaustive match on Noun type, static |
| Generic noun traversal | Runtime recursion + `cell?` | Parameterized functions over Noun |
| Defensive input checking | Op 3 guard | Type-checked at Trident boundary; ill-typed input rejected before nox |
| Circuit cost | Not applicable | Zero — branch compiles to op 4 |

The type system is the right layer. The VM should not carry type
predicates that are always statically decidable in the source language.
