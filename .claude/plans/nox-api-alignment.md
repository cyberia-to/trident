# nox API alignment

Trident is written against the **old** `nox::noun` value/arena API. Current nox
renamed and reshaped it. Align trident to build again.

## The gap

| old `nox::noun` | current `nox` |
|-----------------|---------------|
| `Order<N>` (the arena) | `Reduction<N>` |
| `NounId` (a handle) | `Order` (`= u32`) |
| `Noun { Atom{value,..}, Cell{left,right} }` (`DataEntry.inner`) | `Data { Atom{value}, Pair{left,right} }` |
| `Tag` (`Tag::Field`) | dropped — atoms are untagged |
| `arena.get(id) -> DataEntry` | `-> Option<&DataEntry>` |
| `arena.atom_value(id) -> Option<(Goldilocks, Tag)>` | `-> Option<Goldilocks>` |
| `arena.atom(value, Tag)` | `arena.atom(value)` |
| `arena.cell(a, b)` | `arena.pair(a, b)` |

`nox::run_mir2nox` and the `Noun` enum in `ir/tree/lower/` are **trident's own**
(not nox) — untouched.

## Strategy — aliased imports

Keep trident's type names (`Order` = arena, `NounId` = handle) by aliasing the
new nox names back to them. Only imports + a few call sites change, not every
usage.

- `use nox::noun::{Order, NounId};` → `use nox::{Reduction as Order, Order as NounId};`  (×25)
- `use nox::noun::{Order, Tag};` → `use nox::Reduction as Order;`  (×4, test modules)
- `use nox::noun::{Order, NounId, Noun, Tag};` → `use nox::{Reduction as Order, Order as NounId};`  (compile.rs; `Noun` unused there)
- `use nox::noun::{Order, NounId, Noun};` → `use nox::{Reduction as Order, Order as NounId, Data as Noun};`  (compile/mod.rs)

## Call-site edits

Global (safe — `Noun::cell`/`Noun::atom` are `::` assoc fns on trident's own
Noun, not matched):
- `.cell(` → `.pair(`
- drop `, Tag::Field` argument from `.atom(v, Tag::Field)`

Hand edits in `compile/mod.rs` helpers (`formula_parts`/`body_pair`/`atom_u64`):
- `Noun::Cell` → `Noun::Pair` (Data variant renamed)
- `order.get(x).inner` → `.get(x)` now returns `Option`; use `.ok_or(Malformed)?.inner`
- `atom_value(x)` match `Some((v, _))` → `Some(v)`

## Verify

`cargo build -p trident-lang` clean → `cargo test` → install `trident` → it joins
the toolset (`cy trident`). Branch `feat/nox-api-alignment`, no commit to master.
