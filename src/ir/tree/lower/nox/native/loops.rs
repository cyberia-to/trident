//! One reusable candidate body; dynamic guards never terminate later candidates.
use super::*;

impl Compiler<'_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn for_loop(
        &mut self,
        key: (u32, u32),
        name: &str,
        start: &Expr,
        end: &Expr,
        bound: Option<u64>,
        body: &Block,
    ) -> LowerResult {
        let slots = self
            .function
            .loops
            .get(&key)
            .cloned()
            .ok_or("missing native loop slots")?;
        let code = Code::Loop(self.function.definition.name.node.clone(), key.0, key.1);
        let (start_min, start_max) = self
            .range(start)
            .ok_or("nox: for-loop start must be a compile-time constant or outer loop index")?;
        let end_range = self.range(end);
        let empty = match end_range {
            Some((_, max)) => max <= start_min,
            None => bound == Some(0),
        };
        if empty {
            self.emitted.insert(code, keep(nox_axis(1)));
            // Nested planned entries still need code even when the entire body
            // is statically absent. They are unreachable zero-iteration entries.
            self.empty_loops(body);
            return Ok(keep(nox_axis(1)));
        }
        let maximum = match end_range {
            Some((_, max)) => max.saturating_sub(1),
            None => start_max
                .checked_add(
                    bound
                        .ok_or("nox: dynamic loop requires bounded N")?
                        .saturating_sub(1),
                )
                .ok_or("native loop candidate overflow")?,
        };
        if maximum > u64::from(u32::MAX) || start_max > u64::from(u32::MAX) {
            return Err("native loop candidate exceeds U32".into());
        }
        let start_formula = self.expr(start)?;
        // Compile in the surrounding lexical scope: a same-named loop index
        // must not capture its own end expression.
        let end_formula = self.expr(end)?;
        let count = if end_range.is_some() {
            // The end is an immutable specialized bound, evaluated once.
            seq(
                nox_cons(self.slot(slots.index), end_formula.clone()),
                nox_branch(
                    nox_lt(nox_axis(2), nox_axis(3)),
                    nox_sub(nox_axis(3), nox_axis(2)),
                    nox_unit(),
                ),
            )
        } else {
            nox_quote(Noun::atom(bound.ok_or("missing native loop bound")?))
        };
        let initialize = seq(
            self.set(slots.index, start_formula)?,
            self.set(slots.remaining, count)?,
        );
        self.scopes.push(BTreeMap::new());
        self.bind(
            name,
            Local {
                slot: slots.index,
                ty: Type::U32,
                range: Some((start_min, maximum)),
            },
        )?;
        let body = self.block(body);
        self.scopes.pop();
        let body = body?;
        let candidate = if end_range.is_some() {
            body
        } else {
            nox_branch(
                nox_lt(self.slot(slots.index), end_formula),
                body,
                keep(nox_axis(1)),
            )
        };
        let remaining = self.slot(slots.remaining);
        // Finish before incrementing after the last candidate (u32::MAX is
        // valid). Return flows bypass this entire continuation.
        let decrement = self.set(
            slots.remaining,
            nox_sub(remaining.clone(), nox_quote(Noun::atom(1))),
        )?;
        let increment = self.set(
            slots.index,
            nox_add(self.slot(slots.index), nox_quote(Noun::atom(1))),
        )?;
        let recur = self.plan.invoke(&code, nox_axis(1))?;
        let next = seq(
            decrement,
            nox_branch(
                nox_eq(remaining.clone(), nox_unit()),
                keep(nox_axis(1)),
                seq(increment, recur),
            ),
        );
        let iteration = nox_branch(
            nox_eq(remaining, nox_unit()),
            keep(nox_axis(1)),
            layout::then(candidate, next),
        );
        self.emitted.insert(code.clone(), iteration);
        Ok(seq(initialize, self.plan.invoke(&code, nox_axis(1))?))
    }

    fn empty_loops(&mut self, block: &Block) {
        for stmt in &block.stmts {
            match &stmt.node {
                Stmt::For { body, .. } => {
                    self.emitted.insert(
                        Code::Loop(
                            self.function.definition.name.node.clone(),
                            stmt.span.start,
                            stmt.span.end,
                        ),
                        keep(nox_axis(1)),
                    );
                    self.empty_loops(&body.node);
                }
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.empty_loops(&then_block.node);
                    if let Some(b) = else_block {
                        self.empty_loops(&b.node);
                    }
                }
                Stmt::Match { arms, .. } => {
                    for arm in arms {
                        self.empty_loops(&arm.body.node);
                    }
                }
                _ => {}
            }
        }
    }
}
