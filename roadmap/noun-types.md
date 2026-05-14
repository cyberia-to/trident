---
status: draft
author: mastercyb
---

# Noun Types: Why Nox Drops cell? and What Trident Does Instead

Related: [[polynomial-target]], [[dependent-types]], [[five-algebras]], [[nox]], [[cybergraph]], [[soft3]], [[hemera]], [[bbg]]

## Stack Integration

The [[nox]] data model — atoms as Goldilocks field elements, cells as noun DAG pointers — is what Trident programs manipulate when they interact with [[cybergraph]] data. Every cyberlink in [[cybergraph]] is a noun: `[from_cid, to_cid]` is a pair of [[hemera]]-addressed atoms. A Trident function that traverses a cybergraph knowledge structure is operating on a tree of `Pair(Atom, Atom)` and `Hash` nouns. Without noun types, the only way to dispatch correctly is `cell?` at runtime — which requires a separate witness column in the [[nox]] trace and grows [[bbg]] focus cost for every dispatch.

With noun types, the compiler knows the structure of each [[cybergraph]] noun at the call site. A function that takes `Pair(Hash, Hash)` as input has zero runtime type overhead — the branch compiles to [[nox]] pattern 4 (branch) driven by a field element at a statically known axis. No extra witness column. No [[bbg]] focus charge for type resolution.

[[soft3]]'s `query(cid, dimension)` returns a noun. Currently the return type is opaque — callers must defensively check structure at runtime. Noun types make these return values typesafe: the dimension parameter and the known schema of the target CID together determine the return noun type at compile time. Programs that query [[cybergraph]] get static guarantees about the shape of what they receive, without runtime overhead in the [[nox]] trace.

The `Hash` noun variant (a `[Field; 4]` — 256-bit [[hemera]] address) is the primary currency of [[cybergraph]] interaction. Encoding it as a distinct noun type rather than a raw `Atom` prevents an entire class of bugs where a content address is accidentally treated as a scalar value, or a scalar is accidentally submitted as a CID to [[soft3]]'s `particle()` call.

## Problem

Nock op 3 tests whether a noun is an atom or a cell at runtime:

```
*[a 3 b]  ->  0 if *[a b] is a cell
              1 if *[a b] is an atom
```

Nox drops this (nox op 3 is `cons` — cell construction, not cell predicate). The question is whether to add `cell?` back.

## Why not

A nox execution trace is a sequence of field elements over Goldilocks. Every nox
pattern maps to a polynomial transition constraint selected by a
4-bit tag. `cell?` breaks this model in one specific way: atoms and
cells have structurally different representations. An atom is a
Goldilocks field element (nebu scalar). A cell is a pointer into the noun DAG.
The distinction between them is not a field predicate — it is a type
judgment that requires a separate witness column in the nox trace.

That column either:

- adds prover cost and proof size for a predicate used rarely, or
- must be known statically anyway — in which case the runtime check
  is dead weight

Nock needs `cell?` because Hoon compiles through Nock at runtime:
the evaluator is the runtime. Nox's evaluator is zheng (SuperSpartan IOP + Brakedown PCS).
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
standard nox pattern 4 (branch), driven by a field element the compiler placed
at a known axis. See [`../reference/vm.md`](../reference/vm.md) for the
full nox pattern table and [`../reference/language.md`](../reference/language.md)
for the type system that makes this possible.

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
what does that cost in the nox trace, and on other compilation targets?
See [[dependent-types]] for how dependent typing could eliminate the tag column
entirely for statically provable cases.

Leaning Path A until there's evidence other targets benefit from
general sum types. [[five-algebras]] may surface additional noun-like
structures that would justify Path B.

## Summary

| Need | Nock solution | Nox + Trident solution |
|------|---------------|------------------------|
| Atom vs cell dispatch | `cell?` op at runtime | Exhaustive match on Noun type, static |
| Generic noun traversal | Runtime recursion + `cell?` | Parameterized functions over Noun |
| Defensive input checking | Op 3 guard | Type-checked at Trident boundary; ill-typed input rejected before nox |
| Circuit cost | Not applicable | Zero — branch compiles to nox pattern 4 |

The type system is the right layer. The VM should not carry type
predicates that are always statically decidable in the source language.
See [[polynomial-target]] for how this design enables clean lowering
to nox without runtime type overhead.

## Vision

In a fully live cyber ecosystem, every Trident program that reads from or writes to [[cybergraph]] is working with typed nouns. A program that traverses a knowledge graph — following cyberlinks from `Hash` to `Hash`, descending into `Pair` structures, extracting `Atom` values — does so with compile-time guarantees that match the schema registered in [[Atlas]].

When a [[soft3]] `query(cid, dimension)` call returns a noun, the return type is determined by the schema CID stored in [[Atlas]] for that dimension. The compiler resolves this at compile time, the noun type flows through all downstream code, and the [[nox]] trace carries no runtime type machinery at all. The proof generated by [[zheng]] is smaller; the [[bbg]] focus cost is lower; the [[cybergraph]] cyberlink recording the computation's result is cheaper to write.

The deeper consequence: programs that manipulate [[cybergraph]] knowledge structures are now verifiably correct at the type level. A misrouted cyberlink — a program that confuses a `Hash` CID with a scalar field element and passes it as an argument to [[soft3]]'s `particle()` — is a compile-time error, not a runtime failure that corrupts the knowledge graph. The type boundary is the defense perimeter for the entire [[cybergraph]] substrate's data integrity.