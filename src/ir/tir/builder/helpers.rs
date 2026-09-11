// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Stack wrappers, label generation, cfg helpers, and typed spill effects.

use crate::ast::*;
use crate::span::Spanned;
use crate::tir::TIROp;

use super::TIRBuilder;

// ─── TIRBuilder helpers ────────────────────────────────────────────

impl TIRBuilder {
    // ── Cfg helpers ───────────────────────────────────────────────

    pub(crate) fn is_cfg_active(&self, cfg: &Option<Spanned<String>>) -> bool {
        match cfg {
            None => true,
            Some(flag) => self.cfg_flags.contains(&flag.node),
        }
    }

    pub(crate) fn is_item_cfg_active(&self, item: &Item) -> bool {
        match item {
            Item::Fn(f) => self.is_cfg_active(&f.cfg),
            Item::Const(c) => self.is_cfg_active(&c.cfg),
            Item::Struct(s) => self.is_cfg_active(&s.cfg),
            Item::Event(e) => self.is_cfg_active(&e.cfg),
        }
    }

    // ── Label generation ──────────────────────────────────────────

    pub(crate) fn fresh_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        format!("{}__{}", prefix, self.label_counter)
    }

    // ── Stack effect flushing ─────────────────────────────────────

    pub(crate) fn flush_stack_effects(&mut self) {
        self.ops.extend(self.stack.drain_side_effects());
    }

    // ── Emit helpers ──────────────────────────────────────────────

    /// Ensure stack space, flush spill effects, push the TIROp, push temp to model.
    pub(crate) fn emit_and_push(&mut self, op: TIROp, result_width: u32) {
        if result_width > 0 {
            self.stack.ensure_space(result_width);
            self.flush_stack_effects();
        }
        self.ops.push(op);
        self.stack.push_temp(result_width);
    }

    /// Push an anonymous temporary onto the stack model (no TIROp emitted).
    pub(crate) fn push_temp(&mut self, width: u32) {
        self.stack.push_temp(width);
        self.flush_stack_effects();
    }

    /// Find depth and width of a named variable (may trigger reload if spilled).
    pub(crate) fn find_var_depth_and_width(&mut self, name: &str) -> Option<(u32, u32)> {
        let r = self.stack.find_var_depth_and_width(name);
        self.flush_stack_effects();
        r
    }

    /// Discard abstract stack words; warriors legalize instruction widths.
    pub(crate) fn emit_pop(&mut self, n: u32) {
        if n > 0 {
            self.ops.push(TIROp::Pop(n));
        }
    }

    /// Build a block into a separate Vec<TIROp> by temporarily swapping out self.ops.
    pub(crate) fn build_block_as_ir(&mut self, block: &Block) -> Vec<TIROp> {
        let saved_ops = std::mem::take(&mut self.ops);
        self.build_block(block);
        let nested = std::mem::take(&mut self.ops);
        self.ops = saved_ops;
        nested
    }
}
