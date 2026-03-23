// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Suggestion generation for verification reports.
//!
//! Analyzes verification results and constraint systems to produce
//! actionable fix suggestions (violated constraints, redundant assertions,
//! unconstrained divine inputs).

use crate::solve::{format_constraint, VerificationReport};
use crate::sym::{Constraint, ConstraintSystem, SymValue};

use super::JsonSuggestion;

pub fn generate_suggestions(
    system: &ConstraintSystem,
    report: &VerificationReport,
) -> Vec<JsonSuggestion> {
    let mut suggestions = Vec::new();

    // For each counterexample: "fix_violation" suggestion
    for ce in &report.random_result.counterexamples {
        suggestions.push(JsonSuggestion {
            kind: "fix_violation".to_string(),
            message: format!(
                "Constraint #{} violated (random testing): {}",
                ce.constraint_index, ce.constraint_desc
            ),
            constraint_index: Some(ce.constraint_index),
        });
    }
    for ce in &report.bmc_result.counterexamples {
        // Avoid duplicate suggestions for the same constraint
        let already = suggestions
            .iter()
            .any(|s| s.constraint_index == Some(ce.constraint_index));
        if !already {
            suggestions.push(JsonSuggestion {
                kind: "fix_violation".to_string(),
                message: format!(
                    "Constraint #{} violated (BMC): {}",
                    ce.constraint_index, ce.constraint_desc
                ),
                constraint_index: Some(ce.constraint_index),
            });
        }
    }

    // For each redundant assertion: "remove_redundant" suggestion
    for &idx in &report.redundant_assertions {
        let desc = if idx < system.constraints.len() {
            format_constraint(&system.constraints[idx])
        } else {
            format!("constraint #{}", idx)
        };
        suggestions.push(JsonSuggestion {
            kind: "remove_redundant".to_string(),
            message: format!(
                "Constraint #{} appears redundant (always true): {}",
                idx, desc
            ),
            constraint_index: Some(idx),
        });
    }

    // For divine inputs with no constraints: "add_assertion" suggestion
    let constrained_divines = collect_constrained_divines(system);
    for di in &system.divine_inputs {
        let name = di.to_string();
        if !constrained_divines.contains(&name) {
            suggestions.push(JsonSuggestion {
                kind: "add_assertion".to_string(),
                message: format!(
                    "Divine input '{}' has no constraints -- unconstrained nondeterminism \
                     may allow a malicious prover to choose arbitrary values",
                    name
                ),
                constraint_index: None,
            });
        }
    }

    suggestions
}

/// Collect divine variable names that appear in at least one constraint.
fn collect_constrained_divines(system: &ConstraintSystem) -> Vec<String> {
    let mut names = Vec::new();
    for c in &system.constraints {
        collect_divine_refs_constraint(c, &mut names);
    }
    names.sort();
    names.dedup();
    names
}

fn collect_divine_refs_constraint(c: &Constraint, out: &mut Vec<String>) {
    match c {
        Constraint::Equal(a, b) => {
            collect_divine_refs_value(a, out);
            collect_divine_refs_value(b, out);
        }
        Constraint::AssertTrue(v) => {
            collect_divine_refs_value(v, out);
        }
        Constraint::Conditional(cond, inner) => {
            collect_divine_refs_value(cond, out);
            collect_divine_refs_constraint(inner, out);
        }
        Constraint::RangeU32(v) => {
            collect_divine_refs_value(v, out);
        }
        Constraint::DigestEqual(a, b) => {
            for v in a {
                collect_divine_refs_value(v, out);
            }
            for v in b {
                collect_divine_refs_value(v, out);
            }
        }
    }
}

fn collect_divine_refs_value(v: &SymValue, out: &mut Vec<String>) {
    match v {
        SymValue::Var(var) => {
            if var.name.starts_with("divine_") {
                out.push(var.to_string());
            }
        }
        SymValue::Add(a, b)
        | SymValue::Mul(a, b)
        | SymValue::Sub(a, b)
        | SymValue::Eq(a, b)
        | SymValue::Lt(a, b) => {
            collect_divine_refs_value(a, out);
            collect_divine_refs_value(b, out);
        }
        SymValue::Neg(a) | SymValue::Inv(a) => {
            collect_divine_refs_value(a, out);
        }
        SymValue::Ite(c, t, e) => {
            collect_divine_refs_value(c, out);
            collect_divine_refs_value(t, out);
            collect_divine_refs_value(e, out);
        }
        SymValue::Hash(inputs, _) => {
            for v in inputs {
                collect_divine_refs_value(v, out);
            }
        }
        SymValue::Const(_) | SymValue::Divine(_) | SymValue::PubInput(_) => {}
        SymValue::FieldAccess(inner, _) => {
            collect_divine_refs_value(inner, out);
        }
    }
}
