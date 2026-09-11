//! Assignments and branch stack cleanup.
use super::*;
use crate::span::Spanned;

impl TIRBuilder {
    /// Lower an assignment `place = value` on the stack machine.
    ///
    /// Handles three place forms produced by the parser:
    /// - simple variable `x`
    /// - dotted struct field `p.x` / `p.q.r` (a dotted `Place::Var`)
    /// - array/struct element `a[i]` (a `Place::Index`), constant index
    ///
    /// Each writes a single-word slot in place (evaluate the value, then
    /// `swap`/`pop` it into the target slot). Multi-word field/element writes
    /// and runtime array indices are not yet supported on the stack machine and
    /// emit an explicit error rather than miscompiling.
    pub(crate) fn build_assign(&mut self, place: &Place, value: &Expr) {
        match place {
            Place::Var(name) if !name.contains('.') => {
                // Simple variable reassignment (width-1 slot).
                self.build_expr(value);
                let depth = self.stack.access_var(name);
                self.flush_stack_effects();
                self.store_top_into(depth);
                self.stack.pop();
            }
            // Dotted field access `p.x` — the parser encodes this as a dotted
            // Place::Var; a structured FieldAccess reduces to the same store.
            Place::Var(name) => {
                self.build_dotted_field_store(name, value);
            }
            Place::FieldAccess(inner, field) => {
                // Flatten a structured field-access place into a dotted name
                // when the base is a (possibly dotted) variable.
                if let Some(base) = Self::place_dotted_name(inner) {
                    let full = format!("{}.{}", base, field.node);
                    self.build_dotted_field_store(&full, value);
                } else {
                    self.build_expr(value);
                    self.ops.push(TIROp::Comment(
                        "ERROR: unsupported field-assignment target".to_string(),
                    ));
                    self.ops.push(TIROp::Pop(1));
                    self.stack.pop();
                }
            }
            Place::Index(inner, index) => {
                self.build_index_store(inner, index, value);
            }
        }
    }

    /// Store the single word on top of the stack into the slot at `depth`
    /// (measured from the current top, with the value already pushed).
    fn store_top_into(&mut self, depth: u32) {
        if depth <= 15 {
            self.ops.push(TIROp::Swap(depth));
            self.ops.push(TIROp::Pop(1));
        } else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: assignment target at depth {} exceeds stack window (16)",
                depth
            )));
            self.ops.push(TIROp::Pop(1));
        }
    }

    /// Recover a dotted variable name from a place whose base is a variable.
    fn place_dotted_name(place: &Spanned<Place>) -> Option<String> {
        match &place.node {
            Place::Var(name) => Some(name.clone()),
            Place::FieldAccess(inner, field) => {
                Self::place_dotted_name(inner).map(|b| format!("{}.{}", b, field.node))
            }
            Place::Index(..) => None,
        }
    }

    /// Store into a dotted struct field `base.f0.f1…` (width-1 field only).
    fn build_dotted_field_store(&mut self, name: &str, value: &Expr) {
        let parts: Vec<&str> = name.split('.').collect();
        // Find the longest prefix that names a live variable, and bring it onto
        // the stack BEFORE evaluating the value (so a reload can't land on top
        // of the value word).
        let mut base_split = None;
        for split in 1..parts.len() {
            let var_name = parts[..split].join(".");
            if self.stack.has_var(&var_name) {
                self.stack.access_var(&var_name);
                self.flush_stack_effects();
                base_split = Some(split);
                break;
            }
        }
        self.build_expr(value);
        let Some(split) = base_split else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: unresolved assignment target '{}'",
                name
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        let var_name = parts[..split].join(".");
        let fields = &parts[split..];
        let base_info = self.stack.find_var_depth_and_width(&var_name);
        self.flush_stack_effects();
        let Some((base_depth, _)) = base_info else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: unresolved assignment target '{}'",
                name
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        match self.resolve_nested_field_offset(&var_name, fields) {
            Some((combined_offset, 1)) => {
                let real_depth = base_depth + combined_offset;
                self.store_top_into(real_depth);
            }
            Some((_, field_width)) => {
                self.ops.push(TIROp::Comment(format!(
                    "ERROR: assignment to multi-word field '{}' (width {}) not yet supported on the stack machine",
                    name, field_width
                )));
                self.ops.push(TIROp::Pop(1));
            }
            None => {
                self.ops.push(TIROp::Comment(format!(
                    "ERROR: unresolved field path '{}'",
                    name
                )));
                self.ops.push(TIROp::Pop(1));
            }
        }
        self.stack.pop();
    }

    /// Store into `a[idx]` with a constant index into a named variable
    /// (width-1 element only).
    fn build_index_store(&mut self, inner: &Spanned<Place>, index: &Spanned<Expr>, value: &Expr) {
        let base_name = match &inner.node {
            Place::Var(name) if !name.contains('.') => Some(name.clone()),
            _ => None,
        };
        let const_idx = match &index.node {
            Expr::Literal(Literal::Integer(i)) => Some(*i as u32),
            _ => None,
        };
        let (Some(name), Some(idx)) = (base_name, const_idx) else {
            self.build_expr(value);
            self.ops.push(TIROp::Comment(
                "ERROR: only constant-index assignment into a named array is supported on the stack machine".to_string(),
            ));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        if self.stack.has_var(&name) {
            self.stack.access_var(&name);
            self.flush_stack_effects();
        }
        self.build_expr(value);
        let info = self.stack.find_var_with_elem_width(&name);
        self.flush_stack_effects();
        let Some((var_depth, var_width, elem_width)) = info else {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: unresolved array '{}'",
                name
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        };
        if elem_width != 1 {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: assignment to multi-word array element '{}[{}]' (elem width {}) not yet supported",
                name, idx, elem_width
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        }
        if (idx + 1) * elem_width > var_width {
            self.ops.push(TIROp::Comment(format!(
                "ERROR: index {} out of bounds for '{}' (width {})",
                idx, name, var_width
            )));
            self.ops.push(TIROp::Pop(1));
            self.stack.pop();
            return;
        }
        let base_offset = var_width - (idx + 1) * elem_width;
        let real_depth = var_depth + base_offset;
        self.store_top_into(real_depth);
        self.stack.pop();
    }

    /// Append Pop ops to clean up locals created in an if/else branch.
    /// `post_depth` is stack_depth() after the branch body, `pre_depth` before.
    /// `keep` is the number of words to preserve on top (e.g. a tail expression value).
    pub(super) fn append_branch_cleanup(body: &mut Vec<TIROp>, post_depth: u32, pre_depth: u32, keep: u32) {
        let leftover = post_depth.saturating_sub(pre_depth + keep);
        if leftover > 0 {
            if keep > 0 {
                // Swap the result value(s) past the dead locals, then pop.
                if leftover <= 15 {
                    body.push(TIROp::Swap(leftover));
                } else {
                    for _ in 0..leftover {
                        body.push(TIROp::Swap(1));
                    }
                }
            }
            let mut remaining = leftover;
            while remaining > 0 {
                let batch = remaining.min(5);
                body.push(TIROp::Pop(batch));
                remaining -= batch;
            }
        }
    }
}
