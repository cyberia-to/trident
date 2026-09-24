//! Checked cons-list reads and persistent zipper edits, with shared helper code.
use super::*;

fn q(n: u64) -> Noun {
    nox_quote(Noun::atom(n))
}
fn invoke(plan: &Plan, code: Code, frame: Noun) -> LowerResult {
    plan.invoke(&code, layout::subject(nox_axis(2), frame))
}

pub(super) fn helpers(plan: &Plan) -> Result<BTreeMap<Code, Noun>, String> {
    let mut helpers = BTreeMap::new();
    if plan.codes.contains_key(&Code::Get) {
        // Get frame: [list index]. Caller has already checked its static length.
        let get = nox_branch(
            nox_eq(nox_axis(15), q(0)),
            nox_axis(28),
            invoke(
                plan,
                Code::Get,
                nox_cons(nox_axis(29), nox_sub(nox_axis(15), q(1))),
            )?,
        );
        helpers.insert(Code::Get, get);
    }
    if !plan.codes.contains_key(&Code::Edit) {
        return Ok(helpers);
    }
    // Edit frame: [target [path [replacement zipper]]]. Each path step is
    // [mode index]: mode 0 selects a binary child, 1 a list element, 2 a digest.
    // Zipper: [[direction sibling] remaining], terminated by zero.
    let finish = invoke(plan, Code::Ascend, nox_cons(nox_axis(62), nox_axis(63)))?;
    let mode = nox_axis(120); // path.head.head
    let index = nox_axis(121); // path.head.tail
    let rest = nox_axis(61); // path.tail
    let descend = |right: bool, path: Noun| -> LowerResult {
        let target = nox_axis(if right { 29 } else { 28 });
        let sibling = nox_axis(if right { 28 } else { 29 });
        let zipper = nox_cons(nox_cons(q(u64::from(right)), sibling), nox_axis(63));
        invoke(
            plan,
            Code::Edit,
            nox_cons(target, nox_cons(path, nox_cons(nox_axis(62), zipper))),
        )
    };
    let binary = nox_branch(
        nox_eq(index.clone(), q(0)),
        descend(false, rest.clone())?,
        descend(true, rest.clone())?,
    );
    let list = nox_branch(
        nox_eq(index.clone(), q(0)),
        descend(false, rest.clone())?,
        descend(
            true,
            nox_cons(nox_cons(q(1), nox_sub(index.clone(), q(1))), rest.clone()),
        )?,
    );
    let digest_path = nox_cons(nox_cons(q(0), nox_and(index.clone(), q(1))), rest);
    let digest = nox_branch(
        nox_lt(index, q(2)),
        descend(false, digest_path.clone())?,
        descend(true, digest_path)?,
    );
    let edit = nox_branch(
        nox_eq(nox_axis(30), q(0)),
        finish,
        nox_branch(
            nox_eq(mode.clone(), q(0)),
            binary,
            nox_branch(nox_eq(mode, q(1)), list, digest),
        ),
    );
    // Ascend frame: [value zipper].
    let value = nox_branch(
        nox_eq(nox_axis(60), q(0)),
        nox_cons(nox_axis(14), nox_axis(61)),
        nox_cons(nox_axis(61), nox_axis(14)),
    );
    let ascend = nox_branch(
        nox_eq(nox_axis(15), q(0)),
        nox_axis(14),
        invoke(plan, Code::Ascend, nox_cons(value, nox_axis(31)))?,
    );
    helpers.extend([(Code::Edit, edit), (Code::Ascend, ascend)]);
    Ok(helpers)
}

impl Compiler<'_> {
    fn length(&self, ty: &Type) -> Result<u64, String> {
        match ty {
            Type::Digest => Ok(4),
            Type::Array(_, size) => self
                .owner
                .entry_size(size)
                .ok_or_else(|| "unresolved native array length".into()),
            _ => Err("native index requires array or Digest".into()),
        }
    }
    pub(super) fn read_index(&self, base: Noun, index: Noun, ty: &Type) -> LowerResult {
        let len = self.length(ty)?;
        // Materialize the base and index once; the shared get helper retains
        // the same private subject shape as callable source functions.
        let subject = layout::subject(nox_axis(2), nox_cons(base, index));
        let read = if matches!(ty, Type::Digest) {
            let mut choice = nox_axis(59); // base digest limb 3
            for i in (0..3).rev() {
                choice = nox_branch(nox_eq(nox_axis(15), q(i)), nox_axis(56 + i), choice);
            }
            choice
        } else {
            self.plan.invoke(&Code::Get, nox_axis(1))?
        };
        Ok(seq(
            subject,
            nox_branch(nox_lt(nox_axis(15), q(len)), read, nox_crash()),
        ))
    }
    fn step(&mut self, index: &Expr, ty: &Type) -> LowerResult {
        let len = self.length(ty)?;
        let index = self.expr(index)?;
        Ok(seq(
            index,
            nox_branch(
                nox_lt(nox_axis(1), q(len)),
                nox_cons(
                    q(if matches!(ty, Type::Digest) { 2 } else { 1 }),
                    nox_axis(1),
                ),
                nox_crash(),
            ),
        ))
    }
    fn place(&mut self, place: &Place, steps: &mut Vec<Noun>) -> Result<(usize, Type), String> {
        match place {
            Place::Var(name) => {
                let mut parts = name.split('.');
                let first = parts.next().ok_or("empty assignment target")?;
                let local = self
                    .local(first)
                    .cloned()
                    .ok_or_else(|| format!("unknown native assignment {name}"))?;
                let mut ty = local.ty;
                for field in parts {
                    let (i, next) = self.owner.field_index(&ty, field)?;
                    steps.push(nox_cons(q(1), q(i as u64)));
                    ty = next;
                }
                Ok((local.slot, ty))
            }
            Place::FieldAccess(base, field) => {
                let (slot, ty) = self.place(&base.node, steps)?;
                let (i, next) = self.owner.field_index(&ty, &field.node)?;
                steps.push(nox_cons(q(1), q(i as u64)));
                Ok((slot, next))
            }
            Place::Index(base, index) => {
                let (slot, ty) = self.place(&base.node, steps)?;
                steps.push(self.step(&index.node, &ty)?);
                let next = match ty {
                    Type::Array(t, _) => *t,
                    Type::Digest => Type::Field,
                    _ => return Err("invalid native indexed assignment".into()),
                };
                Ok((slot, next))
            }
        }
    }
    pub(super) fn assign(&mut self, place: &Place, value: &Expr) -> LowerResult {
        let mut steps = Vec::new();
        let (slot, _) = self.place(place, &mut steps)?;
        let value = self.expr(value)?;
        if steps.is_empty() {
            return self.set(slot, value);
        }
        // All indices are evaluated once, in path order, before the RHS. The
        // zipper reconstructs only this variable; other frame slots survive.
        let frame = nox_cons(
            self.slot(slot),
            nox_cons(cons_list(steps), nox_cons(value, q(0))),
        );
        let edited = invoke(self.plan, Code::Edit, frame)?;
        self.set(slot, edited)
    }
}
