//! Bounded unrolling with function-return continuations.
use super::*;

impl NoxCompiler {
    /// Each iteration returns the final function result. Only its fallthrough
    /// continuation restores the outer subject and executes later iterations.
    /// A source return bypasses that continuation, including inside nested loops.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn returning_loop(
        &mut self,
        var: &str,
        start: u64,
        end_const: Option<u64>,
        bound: Option<u64>,
        end: &Spanned<Expr>,
        body: &Block,
        rest: &[Spanned<Stmt>],
        k: Cont,
    ) -> LowerResult {
        let iterations = match (end_const, bound) {
            (Some(end), _) => end.saturating_sub(start),
            (None, Some(bound)) => bound,
            (None, None) => return Err("nox: dynamic loop requires `bounded N`".into()),
        };
        check_unroll(iterations)?;
        let outer = self.scope.clone();
        let continuation = self.compile_stmts_k(rest, k);
        self.scope = outer.clone();
        let mut next = continuation?;
        for offset in (0..iterations).rev() {
            let index = start
                .checked_add(offset)
                .ok_or("nox: bounded loop index overflow")?;
            let baseline_depth = self.scope.depth;
            self.scope.push_frame();
            self.scope.bind_loop_index(var, index)?;
            self.check_depth()?;
            let inner = self.compile_stmts_k(&body.stmts, &mut |c| {
                let mut finish = |c: &mut Self| {
                    let drop = reify_drop(c.scope.depth - baseline_depth, c.scope.depth);
                    Ok(seq(drop, next.clone()))
                };
                match body.tail_expr.as_deref() {
                    Some(tail) => c.effect_then(tail, &mut finish),
                    None => finish(c),
                }
            });
            self.scope = outer.clone();
            let iteration = seq(nox_cons(nox_quote(Noun::atom(index)), nox_axis(1)), inner?);
            next = if end_const.is_some() {
                iteration
            } else {
                let condition = nox_lt(nox_quote(Noun::atom(index)), self.compile_expr(&end.node)?);
                nox_branch(condition, iteration, next)
            };
            // Absorbed branches can duplicate continuations. Bound this actual
            // emitted tree, not only the number of source loop iterations.
            if count_nodes(&next) > MAX_INLINE_NODES {
                return Err("nox: return-aware loop exceeds formula node budget".into());
            }
        }
        Ok(next)
    }
}
