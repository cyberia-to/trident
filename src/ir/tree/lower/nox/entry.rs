//! Marshal the flat public input into the source entry's typed noun subject.
use super::*;

#[derive(Clone)]
enum Shape {
    Scalar(Option<u64>),
    List(Vec<Shape>),
    Digest,
}

impl NoxCompiler {
    pub(super) fn adapt_entry(&self, function: &FnDef, body: Noun) -> LowerResult {
        let mut words = 0;
        let mut nodes = 0;
        let shapes = function
            .params
            .iter()
            .map(|parameter| self.entry_shape(&parameter.ty.node, 0, &mut words, &mut nodes))
            .collect::<Result<Vec<_>, _>>()?;
        if words + u32::from(self.reads_state) > MAX_SUBJECT_DEPTH {
            return Err("nox: typed entry exceeds the public input word limit".into());
        }
        let base = if self.reads_state { 3 } else { 1 };
        let mut next = 0;
        let mut subject = nox_unit();
        for shape in &shapes {
            subject = nox_cons(shape.formula(base, words, &mut next), subject);
        }
        if self.reads_state {
            subject = nox_cons(nox_axis(2), subject);
        }
        // Rebuilding forces every leaf check, including unused parameters.
        // The original list must end in exactly zero after the declared words.
        let remainder = nox_axis(axis_compose(base, tail_axis(words)));
        let check = nox_branch(nox_eq(remainder, nox_unit()), nox_unit(), nox_crash());
        // Sequence the scalar assertion before the body; do not invent an
        // atom-valued error arm for a potentially aggregate-valued result.
        Ok(seq(
            nox_cons(check, nox_axis(1)),
            seq(nox_axis(3), seq(subject, body)),
        ))
    }

    fn entry_shape(
        &self,
        ty: &ast::Type,
        depth: usize,
        words: &mut u32,
        nodes: &mut usize,
    ) -> Result<Shape, String> {
        *nodes += 1;
        if depth > 16 || *nodes > 4096 {
            return Err("nox: typed entry aggregate layout is too deep or large".into());
        }
        let shape = match ty {
            ast::Type::Field | ast::Type::Bool | ast::Type::U32 => {
                *words += 1;
                Shape::Scalar(match ty {
                    ast::Type::Bool => Some(2),
                    ast::Type::U32 => Some(1u64 << 32),
                    _ => None,
                })
            }
            ast::Type::Digest => {
                *words += 4;
                Shape::Digest
            }
            ast::Type::XField => {
                return Err("nox: XField entry has no implemented representation".into());
            }
            ast::Type::Tuple(types) => self.entry_list(types.iter(), depth, words, nodes)?,
            ast::Type::Array(inner, size) => {
                let size = self
                    .entry_size(size)
                    .ok_or("nox: unresolved entry array length")?;
                if size > u64::from(MAX_SUBJECT_DEPTH) {
                    return Err("nox: entry aggregate exceeds the element limit".into());
                }
                self.entry_list(
                    std::iter::repeat_n(inner.as_ref(), size as usize),
                    depth,
                    words,
                    nodes,
                )?
            }
            ast::Type::Named(path) => {
                let fields = self
                    .structs
                    .get(&path.as_dotted())
                    .ok_or_else(|| format!("nox: unresolved entry struct {}", path.as_dotted()))?;
                self.entry_list(fields.iter().map(|(_, ty)| ty), depth, words, nodes)?
            }
        };
        if *words > MAX_SUBJECT_DEPTH {
            return Err("nox: typed entry exceeds the public input word limit".into());
        }
        Ok(shape)
    }

    fn entry_list<'a>(
        &self,
        types: impl Iterator<Item = &'a ast::Type>,
        depth: usize,
        words: &mut u32,
        nodes: &mut usize,
    ) -> Result<Shape, String> {
        let mut shapes = Vec::new();
        for ty in types {
            if shapes.len() >= MAX_SUBJECT_DEPTH as usize {
                return Err("nox: entry aggregate exceeds the element limit".into());
            }
            shapes.push(self.entry_shape(ty, depth + 1, words, nodes)?);
        }
        Ok(Shape::List(shapes))
    }

    fn entry_size(&self, size: &ast::ArraySize) -> Option<u64> {
        match size {
            ast::ArraySize::Literal(n) => Some(*n),
            ast::ArraySize::Param(name) => self.constants.get(&self.symbol(name)).copied(),
            ast::ArraySize::Add(a, b) => self.entry_size(a)?.checked_add(self.entry_size(b)?),
            ast::ArraySize::Mul(a, b) => self.entry_size(a)?.checked_mul(self.entry_size(b)?),
        }
    }
}

impl Shape {
    fn formula(&self, base: u64, words: u32, next: &mut u32) -> Noun {
        match self {
            Self::Scalar(bound) => {
                let value = nox_axis(axis_compose(base, stack_axis(words - 1 - *next)));
                *next += 1;
                match bound {
                    Some(bound) => nox_branch(
                        nox_lt(value.clone(), nox_quote(Noun::atom(*bound))),
                        value,
                        nox_crash(),
                    ),
                    // Add zero forces an atom, even when the function ignores it.
                    None => nox_add(value, nox_unit()),
                }
            }
            Self::List(shapes) => cons_list(
                shapes
                    .iter()
                    .map(|s| s.formula(base, words, next))
                    .collect(),
            ),
            Self::Digest => {
                let mut leaf = || Self::Scalar(None).formula(base, words, next);
                let a = leaf();
                let b = leaf();
                let c = leaf();
                let d = leaf();
                nox_cons(nox_cons(a, b), nox_cons(c, d))
            }
        }
    }
}
