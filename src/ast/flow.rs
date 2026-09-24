//! Literal assertion failure, resolved by the owning compilation stage.
use super::{Expr, Literal};

pub(crate) fn is_false_assert(expr: &Expr, is_assert: impl FnOnce(&str) -> bool) -> bool {
    let Expr::Call { path, args, .. } = expr else {
        return false;
    };
    args.len() == 1
        && matches!(args[0].node, Expr::Literal(Literal::Bool(false)))
        && is_assert(&path.node.as_dotted())
}

/// A known condition uses the target's actual boolean encoding and field.
pub(crate) fn constant_condition(
    expr: &Expr,
    constants: &std::collections::BTreeMap<String, u64>,
    target: &str,
) -> Option<bool> {
    if let Expr::Literal(Literal::Bool(value)) = expr {
        return Some(*value);
    }
    let value = constant_scalar(expr, constants)?;
    match target {
        "nox" => Some(value % nebu::field::P == 0),
        "triton" => Some(value % nebu::field::P != 0),
        _ => None,
    }
}

fn constant_scalar(
    expr: &Expr,
    constants: &std::collections::BTreeMap<String, u64>,
) -> Option<u64> {
    match expr {
        Expr::Literal(Literal::Integer(value)) => Some(*value),
        Expr::Var(name) => constants.get(name).copied(),
        _ => None,
    }
}

/// Conservative nonemptiness proof for the portable U32 loop domain.
/// Larger source atoms can wrap in a target field; their raw order is no proof.
pub(crate) fn constant_nonempty_loop(
    start: &Expr,
    end: &Expr,
    bound: Option<u64>,
    constants: &std::collections::BTreeMap<String, u64>,
    target: &str,
) -> bool {
    if !matches!(target, "nox" | "triton") {
        return false;
    }
    match (
        constant_scalar(start, constants),
        constant_scalar(end, constants),
    ) {
        (Some(start), Some(end)) => {
            start < end && end <= u64::from(u32::MAX) && bound.is_none_or(|n| n > 0)
        }
        _ => false,
    }
}

/// A local root shadows every dotted reference below that root as well.
pub(crate) fn shadow_flow_constant(
    constants: &mut std::collections::BTreeMap<String, u64>,
    name: &str,
) {
    let prefix = format!("{name}.");
    constants.retain(|key, _| key != name && !key.starts_with(&prefix));
}
