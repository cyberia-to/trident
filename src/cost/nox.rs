// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Honest reduction cost for the nox target.
//!
//! nox executes a program by reducing a Noun *formula* against a subject.
//! Every pattern application costs a fixed number of reductions (the unit of
//! nox proving cost — one soundness-witness slot per reduction). The cost of a
//! whole program is the sum of the pattern costs actually executed.
//!
//! This module computes that sum by walking the *emitted* formula (what the
//! NoxCompiler produces — the single source of truth), never by re-deriving
//! from the AST. Straight-line code has an exact cost; every `branch` (from
//! `if`/`else` and from dynamic-bounded loop guards) makes the executed cost
//! depend on which arm is taken, so the bill is reported as a range
//! `min..=max` — an honest "≤ max", never a fabricated exact number.
//!
//! ## Sync
//!
//! [`PATTERN_REDUCTIONS`] mirrors `cyber-nox`'s private `reduce::COSTS` table
//! (see `nox/rs/reduce.rs` and `nox/specs/trace.md`). It is one of the four
//! builtin-sync sites (reference/language.md · typecheck · ir lowering ·
//! **cost**). If nox's cost table changes, this array changes with it; the
//! `pattern_costs_match_nox_semantics` test pins the values.

use crate::ir::tree::lower::Noun;

/// Per-pattern reduction cost, indexed by nox pattern tag (0..=17).
/// Mirrors `cyber-nox` `reduce::COSTS`. Multi-row patterns (inv, lt, xor, and,
/// not, shl, hash) cost more than one reduction because they emit one trace
/// row per bit / per Poseidon2 round.
pub const PATTERN_REDUCTIONS: [u64; 18] = [
    1,  // 0  axis
    1,  // 1  quote
    1,  // 2  compose
    1,  // 3  cons
    1,  // 4  branch
    1,  // 5  add
    1,  // 6  sub
    1,  // 7  mul
    64, // 8  inv   — Fermat exponent, one row per bit of p-2
    1,  // 9  eq
    64, // 10 lt    — bit decomposition of two 64-bit reprs
    32, // 11 xor   — 32-bit word
    32, // 12 and
    32, // 13 not
    32, // 14 shl
    25, // 15 hash  — 24 Poseidon2 rounds + 1 squeeze
    1,  // 16 call
    1,  // 17 look
];

/// Human-readable pattern names, indexed by tag.
pub const PATTERN_NAMES: [&str; 18] = [
    "axis", "quote", "compose", "cons", "branch", "add", "sub", "mul", "inv", "eq", "lt", "xor",
    "and", "not", "shl", "hash", "call", "look",
];

/// A reduction bill: the executed reduction count is somewhere in `min..=max`.
/// When `min == max` the cost is exact (no branch on any executed path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bill {
    pub min: u64,
    pub max: u64,
}

impl Bill {
    fn exact(c: u64) -> Self {
        Bill { min: c, max: c }
    }
    fn add(self, other: Bill) -> Self {
        Bill {
            min: self.min + other.min,
            max: self.max + other.max,
        }
    }
    /// Combine two branch arms: exactly one runs, so the executed cost lies
    /// between the cheaper arm's min and the dearer arm's max.
    fn branch(a: Bill, b: Bill) -> Self {
        Bill {
            min: a.min.min(b.min),
            max: a.max.max(b.max),
        }
    }
    pub fn is_exact(&self) -> bool {
        self.min == self.max
    }
}

/// The full reduction cost of a nox program: the executed-reduction range plus
/// a static inventory of how many times each pattern is emitted in the formula
/// (counting every arm — an upper structural bound, useful for seeing where the
/// cost lives).
#[derive(Debug, Clone)]
pub struct NoxCost {
    pub bill: Bill,
    /// Static count of each pattern's applications across the whole formula
    /// (indexed by tag). Every emitted arm is counted, so branch arms are both
    /// present here even though only one runs.
    pub pattern_counts: [u64; 18],
    /// Total nodes in the formula (atoms + cells) — a size proxy.
    pub nodes: u64,
}

impl NoxCost {
    /// Analyze an emitted nox formula.
    pub fn analyze(formula: &Noun) -> Self {
        let mut counts = [0u64; 18];
        let bill = walk(formula, &mut counts);
        NoxCost {
            bill,
            pattern_counts: counts,
            nodes: count_nodes(formula),
        }
    }

    /// Format the reduction bill for `trident build --cost`.
    pub fn format_report(&self) -> String {
        let mut out = String::new();
        out.push_str("Cost model: reductions (nox)\n");
        if self.bill.is_exact() {
            out.push_str(&format!("  reductions:  {}\n", self.bill.max));
        } else {
            out.push_str(&format!(
                "  reductions:  {}..={}  (≤ {}; branch-dependent)\n",
                self.bill.min, self.bill.max, self.bill.max
            ));
        }
        out.push_str(&format!("  formula nodes: {}\n", self.nodes));
        out.push_str("  by pattern (static, all arms):\n");
        // Show only patterns that actually appear, heaviest contribution first.
        let mut rows: Vec<(usize, u64, u64)> = (0..18)
            .filter(|&t| self.pattern_counts[t] > 0)
            .map(|t| (t, self.pattern_counts[t], self.pattern_counts[t] * PATTERN_REDUCTIONS[t]))
            .collect();
        rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
        for (t, n, sub) in rows {
            out.push_str(&format!(
                "    {:<8} ×{:<5} = {} reductions\n",
                PATTERN_NAMES[t], n, sub
            ));
        }
        out
    }

    /// Headline number for a bench table cell (max = upper bound).
    pub fn max_reductions(&self) -> u64 {
        self.bill.max
    }
}

/// If `n` is a quote formula `[1 x]`, return the quoted noun `x`.
fn as_quote(n: &Noun) -> Option<&Noun> {
    match n {
        Noun::Cell(h, t) => match h.as_ref() {
            Noun::Atom(1) => Some(t),
            _ => None,
        },
        Noun::Atom(_) => None,
    }
}

/// Extract the two children of a `[a b]` cell, or `None` if malformed.
fn pair(n: &Noun) -> Option<(&Noun, &Noun)> {
    match n {
        Noun::Cell(a, b) => Some((a, b)),
        Noun::Atom(_) => None,
    }
}

/// Walk a formula, accumulating per-pattern counts and returning its bill.
/// Only genuine formula positions are recursed into — quoted data (`[1 c]`)
/// and axis addresses (`[0 addr]`) are leaves, never walked as code.
fn walk(formula: &Noun, counts: &mut [u64; 18]) -> Bill {
    let Some((head, body)) = pair(formula) else {
        // A bare atom in formula position is not executable code; contribute 0.
        return Bill::default();
    };
    let tag = match head {
        Noun::Atom(v) => *v,
        // A cell head (autocons / distribution) is not produced by NoxCompiler;
        // count nothing rather than guess.
        Noun::Cell(..) => return Bill::default(),
    };
    if (tag as usize) < 18 {
        counts[tag as usize] += 1;
    }
    let base = Bill::exact(cost(tag));
    match tag {
        // Leaves: subject is data, not code.
        0 | 1 => base,
        // compose [2 [a b]] = reduce(reduce(s,a), reduce(s,b)). The dispatch
        // costs 1; then a and b are each evaluated, and the results are
        // reduced together. The NoxCompiler always emits the continuation as a
        // quoted formula (`seq(t, rest)` = `[2 [t [1 rest]]]`): b evaluates to
        // that formula, which then runs against a's result. So we cost b's
        // quote (1) plus the quoted continuation itself.
        2 => match pair(body) {
            Some((a, b)) => {
                let bill = base.add(walk(a, counts));
                match as_quote(b) {
                    Some(cont) => {
                        // b is `[1 cont]`: quote dispatch (1) + run the
                        // continuation formula.
                        if (1usize) < 18 {
                            counts[1] += 1;
                        }
                        bill.add(Bill::exact(PATTERN_REDUCTIONS[1]))
                            .add(walk(cont, counts))
                    }
                    // Dynamic continuation (never emitted by NoxCompiler): the
                    // formula produced by b is not statically known.
                    None => bill.add(walk(b, counts)),
                }
            }
            None => base,
        },
        // cons [3 [a b]] — both children are formulas evaluated against s.
        3 => match pair(body) {
            Some((a, b)) => base.add(walk(a, counts)).add(walk(b, counts)),
            None => base,
        },
        // branch [4 [test [yes no]]] — test always runs; exactly one arm runs.
        4 => match pair(body).and_then(|(t, rest)| pair(rest).map(|(y, n)| (t, y, n))) {
            Some((test, yes, no)) => base
                .add(walk(test, counts))
                .add(Bill::branch(walk(yes, counts), walk(no, counts))),
            None => base,
        },
        // Binary field / bitwise ops [tag [a b]].
        5 | 6 | 7 | 9 | 10 | 11 | 12 | 14 => match pair(body) {
            Some((a, b)) => base.add(walk(a, counts)).add(walk(b, counts)),
            None => base,
        },
        // Unary ops: inv [8 a], not [13 a], hash [15 a] — body IS the operand.
        8 | 13 | 15 => base.add(walk(body, counts)),
        // call [16 [tag_f check_f]] — both are formulas.
        16 => match pair(body) {
            Some((a, b)) => base.add(walk(a, counts)).add(walk(b, counts)),
            None => base,
        },
        // look [17 body].
        17 => base.add(walk(body, counts)),
        _ => base,
    }
}

fn cost(tag: u64) -> u64 {
    if (tag as usize) < PATTERN_REDUCTIONS.len() {
        PATTERN_REDUCTIONS[tag as usize]
    } else {
        1
    }
}

fn count_nodes(n: &Noun) -> u64 {
    match n {
        Noun::Atom(_) => 1,
        Noun::Cell(a, b) => 1 + count_nodes(a) + count_nodes(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::tree::lower::Noun;

    fn atom(v: u64) -> Noun {
        Noun::atom(v)
    }
    fn cell(a: Noun, b: Noun) -> Noun {
        Noun::cell(a, b)
    }

    #[test]
    fn pattern_costs_match_nox_semantics() {
        // Pinned mirror of cyber-nox reduce::COSTS. If nox changes, change here.
        assert_eq!(PATTERN_REDUCTIONS[0], 1); // axis
        assert_eq!(PATTERN_REDUCTIONS[8], 64); // inv
        assert_eq!(PATTERN_REDUCTIONS[10], 64); // lt
        assert_eq!(PATTERN_REDUCTIONS[11], 32); // xor
        assert_eq!(PATTERN_REDUCTIONS[15], 25); // hash
        assert_eq!(PATTERN_REDUCTIONS[16], 1); // call
    }

    #[test]
    fn quote_is_one_reduction() {
        // [1 42]
        let f = cell(atom(1), atom(42));
        let c = NoxCost::analyze(&f);
        assert_eq!(c.bill, Bill { min: 1, max: 1 });
        assert!(c.bill.is_exact());
    }

    #[test]
    fn add_of_two_quotes_is_three() {
        // [5 [[1 3] [1 5]]] — add(1 reduction) + quote + quote = 3
        let f = cell(atom(5), cell(cell(atom(1), atom(3)), cell(atom(1), atom(5))));
        let c = NoxCost::analyze(&f);
        assert_eq!(c.bill.max, 3);
        assert!(c.bill.is_exact());
        assert_eq!(c.pattern_counts[5], 1);
        assert_eq!(c.pattern_counts[1], 2);
    }

    #[test]
    fn does_not_walk_quoted_data() {
        // quote of a pair: [1 [5 [1 1]]] — the inner [5 ...] is DATA, not an
        // add pattern. Cost must be 1 (just the quote), not 1+add.
        let data = cell(atom(5), cell(atom(1), atom(1)));
        let f = cell(atom(1), data);
        let c = NoxCost::analyze(&f);
        assert_eq!(c.bill.max, 1, "quoted data must not be counted as code");
        assert_eq!(c.pattern_counts[5], 0);
    }

    #[test]
    fn hash_costs_twenty_five_plus_operand() {
        // [15 [1 7]] — hash(25) + quote(1) = 26
        let f = cell(atom(15), cell(atom(1), atom(7)));
        let c = NoxCost::analyze(&f);
        assert_eq!(c.bill.max, 26);
        assert_eq!(c.pattern_counts[15], 1);
    }

    #[test]
    fn compose_runs_the_quoted_continuation() {
        // seq(t, rest) = [2 [t [1 rest]]]. The continuation `rest` is quoted
        // but executed by compose, so its cost must be counted (not treated as
        // inert data). t = [1 0] (quote, 1), rest = [5 [[1 1][1 2]]] (add: 3).
        // compose(1) + t(1) + quote-dispatch(1) + rest(3) = 6.
        let t = cell(atom(1), atom(0));
        let rest = cell(atom(5), cell(cell(atom(1), atom(1)), cell(atom(1), atom(2))));
        let f = cell(atom(2), cell(t, cell(atom(1), rest)));
        let c = NoxCost::analyze(&f);
        assert_eq!(c.bill.max, 6);
        assert!(c.bill.is_exact());
        assert_eq!(c.pattern_counts[5], 1, "continuation's add must be counted");
    }

    #[test]
    fn branch_produces_a_range() {
        // [4 [test [yes no]]] with test=[9[[1 0][1 0]]] (eq: 1+1+1=3),
        // yes=[15 [1 1]] (hash: 26), no=[1 0] (quote: 1)
        // total = branch(1) + test(3) + arm∈[1,26] → min 5, max 30
        let test = cell(atom(9), cell(cell(atom(1), atom(0)), cell(atom(1), atom(0))));
        let yes = cell(atom(15), cell(atom(1), atom(1)));
        let no = cell(atom(1), atom(0));
        let f = cell(atom(4), cell(test, cell(yes, no)));
        let c = NoxCost::analyze(&f);
        assert_eq!(c.bill.min, 5);
        assert_eq!(c.bill.max, 30);
        assert!(!c.bill.is_exact());
    }
}
