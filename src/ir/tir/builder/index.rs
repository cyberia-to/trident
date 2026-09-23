//! Bounds-checked array selection. A balanced structural tree avoids computed
//! RAM addresses and keeps indices independent of target memory conventions.
use super::*;
use crate::span::Spanned;

pub(super) fn select(
    index_depth: u32,
    start: u32,
    end: u32,
    leaf: &dyn Fn(u32) -> Vec<TIROp>,
) -> Vec<TIROp> {
    if end - start == 1 {
        return leaf(start);
    }
    let mid = start + (end - start) / 2;
    vec![
        TIROp::Push(mid as u64),
        TIROp::Dup(index_depth + 1),
        TIROp::Lt,
        TIROp::IfElse {
            then_body: select(index_depth, start, mid, leaf),
            else_body: select(index_depth, mid, end, leaf),
        },
    ]
}
#[derive(Clone)]
enum Projection {
    Field(String),
    Index(Spanned<Expr>),
}
#[derive(Clone)]
enum Step {
    Field(u32),
    Index { slot: u32, count: u32, width: u32 },
}
fn flatten(place: &Place, steps: &mut Vec<Projection>) -> String {
    match place {
        Place::Var(name) => {
            let mut parts = name.split('.');
            let root = parts.next().unwrap_or_default().to_string();
            steps.extend(parts.map(|p| Projection::Field(p.into())));
            root
        }
        Place::FieldAccess(base, field) => {
            let root = flatten(&base.node, steps);
            steps.push(Projection::Field(field.node.clone()));
            root
        }
        Place::Index(base, index) => {
            let root = flatten(&base.node, steps);
            steps.push(Projection::Index((**index).clone()));
            root
        }
    }
}
fn stores(
    steps: &[Step],
    offset: u32,
    root_depth: u32,
    rhs_width: u32,
    indices: u32,
) -> Vec<TIROp> {
    match steps.split_first() {
        None => (0..rhs_width)
            .flat_map(|_| [TIROp::Swap(root_depth + offset), TIROp::Pop(1)])
            .collect(),
        Some((Step::Field(n), rest)) => stores(rest, offset + n, root_depth, rhs_width, indices),
        Some((Step::Index { slot, count, width }, rest)) => {
            select(rhs_width + indices - 1 - slot, 0, *count, &|i| {
                stores(
                    rest,
                    offset + (count - 1 - i) * width,
                    root_depth,
                    rhs_width,
                    indices,
                )
            })
        }
    }
}
impl TIRBuilder {
    pub(super) fn assert_index_bound(&mut self, count: u32) {
        self.ops.extend([
            TIROp::Push(count as u64),
            TIROp::Dup(1),
            TIROp::Lt,
            TIROp::Assert(1),
        ]);
    }
    pub(super) fn field_type_offset(&self, ty: &Type, name: &str) -> Option<(Type, u32)> {
        let Type::Named(path) = ty else { return None };
        let def = self
            .struct_types
            .get(&self.qualified_name(&path.as_dotted()))?;
        let total = self.type_width(ty);
        let mut used = 0;
        for field in &def.fields {
            let width = self.type_width(&field.ty.node);
            used += width;
            if field.name.node == name {
                return Some((field.ty.node.clone(), total - used));
            }
        }
        None
    }
    pub(super) fn build_projected_store(&mut self, place: &Place, value: &Expr) {
        let mut projections = Vec::new();
        let root = flatten(place, &mut projections);
        let Some(mut ty) = self.var_types.get(&root).cloned() else {
            self.ops.extend([TIROp::Push(0), TIROp::Assert(1)]);
            return;
        };
        self.stack.access_var(&root);
        self.flush_stack_effects();
        let mut steps = Vec::new();
        let mut indices = 0;
        for projection in projections {
            match projection {
                Projection::Field(name) => {
                    let Some((next, offset)) = self.field_type_offset(&ty, &name) else {
                        self.ops.extend([TIROp::Push(0), TIROp::Assert(1)]);
                        return;
                    };
                    ty = next;
                    steps.push(Step::Field(offset));
                }
                Projection::Index(expr) => {
                    let Type::Array(element, size) = ty else {
                        self.ops.extend([TIROp::Push(0), TIROp::Assert(1)]);
                        return;
                    };
                    let count = size.eval(&self.current_subs) as u32;
                    self.build_expr(&expr.node);
                    self.assert_index_bound(count);
                    if count == 0 {
                        return;
                    }
                    steps.push(Step::Index {
                        slot: indices,
                        count,
                        width: self.type_width(&element),
                    });
                    indices += 1;
                    ty = *element;
                }
            }
        }
        self.build_expr(value);
        let width = self.type_width(&ty);
        let Some((depth, _)) = self.find_var_depth_and_width(&root) else {
            self.ops.extend([TIROp::Push(0), TIROp::Assert(1)]);
            return;
        };
        self.ops.extend(stores(&steps, 0, depth, width, indices));
        self.stack.pop();
        self.emit_pop(indices);
        for _ in 0..indices {
            self.stack.pop();
        }
    }
}
