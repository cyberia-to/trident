//! Function emission and return cleanup.
use super::*;

impl TIRBuilder {
    pub(crate) fn build_fn(&mut self, func: &FnDef) {
        if func.body.is_none() {
            return;
        }
        let name = func.name.node.clone();
        let param_widths: Vec<u32> = func
            .params
            .iter()
            .map(|p| self.type_width(&p.ty.node))
            .collect();
        let ret_width = func
            .return_ty
            .as_ref()
            .map(|t| self.type_width(&t.node))
            .unwrap_or(0);
        self.build_fn_body(&name, func, &param_widths, ret_width);
    }

    pub(super) fn build_mono_fn(&mut self, func: &FnDef, inst: &MonoInstance) {
        if func.body.is_none() {
            return;
        }
        // Set up substitution context.
        self.current_subs.clear();
        for (param, val) in func.type_params.iter().zip(inst.size_args.iter()) {
            self.current_subs.insert(param.node.clone(), *val);
        }
        let name = inst.mangled_name();
        let param_widths: Vec<u32> = func
            .params
            .iter()
            .map(|p| {
                resolve_type_width_with_subs(&p.ty.node, &self.current_subs, &self.target_config)
            })
            .collect();
        let ret_width = func
            .return_ty
            .as_ref()
            .map(|t| resolve_type_width_with_subs(&t.node, &self.current_subs, &self.target_config))
            .unwrap_or(0);
        self.build_fn_body(&name, func, &param_widths, ret_width);
        self.current_subs.clear();
    }

    /// Detect a pass-through function: body is a single call with all
    /// params forwarded in declaration order. Works for any param width —
    /// the values are already on the stack in the correct position.
    fn detect_pass_through(&self, func: &FnDef, _param_widths: &[u32]) -> bool {
        let body = match &func.body {
            Some(b) => b,
            None => return false,
        };
        if !body.node.stmts.is_empty() {
            return false;
        }
        let tail = match &body.node.tail_expr {
            Some(t) => t,
            None => return false,
        };
        let args = match &tail.node {
            Expr::Call { args, .. } => args,
            _ => return false,
        };
        if args.len() != func.params.len() {
            return false;
        }
        for (arg, param) in args.iter().zip(func.params.iter()) {
            match &arg.node {
                Expr::Var(name) if name == &param.name.node => {}
                _ => return false,
            }
        }
        true
    }

    /// Shared body for `build_fn` and `build_mono_fn`.
    ///
    /// Emits FnStart, registers parameters, compiles the body, cleans up
    /// the stack, and emits Return + FnEnd.
    fn build_fn_body(&mut self, name: &str, func: &FnDef, param_widths: &[u32], ret_width: u32) {
        self.ops.push(TIROp::FnStart(name.to_string()));
        self.stack.clear();
        self.var_types.clear();
        self.struct_layouts.clear();

        // Pass-through optimization: if the body is a single call that
        // forwards all width-1 params in order, skip variable registration
        // and emit only the call instruction.
        if self.detect_pass_through(func, param_widths) {
            let body = func.body.as_ref().unwrap();
            let tail = body.node.tail_expr.as_ref().unwrap();
            if let Expr::Call {
                path,
                generic_args,
                args,
            } = &tail.node
            {
                let call_name = path.node.as_dotted();
                self.emit_call_only(&call_name, generic_args, args.len());
            }
            self.ops.push(TIROp::Return);
            self.ops.push(TIROp::FnEnd);
            self.stack.clear();
            return;
        }

        // Parameters are already on the real stack. Register them in the model.
        for (param, &width) in func.params.iter().zip(param_widths) {
            self.var_types
                .insert(param.name.node.clone(), param.ty.node.clone());
            self.register_struct_layout_from_type(&param.name.node, &param.ty.node);
            self.stack.push_named(&param.name.node, width);
            self.flush_stack_effects();
        }

        let body = func.body.as_ref().expect("caller checked body.is_some()");
        let has_return = func.return_ty.is_some();

        if has_return && ret_width > 1 {
            // Multi-element return: build statements first, then handle
            // the tail expression specially to avoid unnecessary copies.
            for stmt in &body.node.stmts {
                self.build_stmt(&stmt.node);
            }

            if let Some(tail) = &body.node.tail_expr {
                let depth_before_tail = self.stack.stack_depth();

                // Check if tail is a simple variable reference at the top.
                let var_name = match &tail.node {
                    Expr::Var(v) => Some(v.clone()),
                    _ => None,
                };
                let var_info = var_name.as_ref().and_then(|v| {
                    self.stack.access_var(v);
                    self.flush_stack_effects();
                    self.find_var_depth_and_width(v)
                });

                if let Some((depth, width)) = var_info {
                    if width == ret_width && depth == 0 {
                        // Return variable is already at the top of the stack.
                        // Just pop dead elements below it.
                        let dead = depth_before_tail - ret_width;
                        if dead > 0 {
                            self.emit_multi_ret_cleanup(ret_width, dead);
                        }
                        // Skip building the tail expr (no dup needed).
                    } else {
                        self.build_expr(&tail.node);
                        let to_pop = self.stack.stack_depth().saturating_sub(ret_width);
                        if to_pop > 0 {
                            self.emit_multi_ret_cleanup(ret_width, to_pop);
                        }
                    }
                } else {
                    self.build_expr(&tail.node);
                    let to_pop = self.stack.stack_depth().saturating_sub(ret_width);
                    if to_pop > 0 {
                        self.emit_multi_ret_cleanup(ret_width, to_pop);
                    }
                }
            }
        } else {
            // Single-element or void return: use the standard path.
            self.build_block(&body.node);
            let total_width = self.stack.stack_depth();

            if has_return && total_width > 0 {
                let to_pop = total_width.saturating_sub(ret_width);
                if to_pop > 0 && to_pop <= 15 {
                    self.ops.push(TIROp::Swap(to_pop));
                    self.emit_pop(to_pop);
                } else if to_pop > 0 {
                    for _ in 0..to_pop {
                        self.ops.push(TIROp::Swap(1));
                        self.ops.push(TIROp::Pop(1));
                    }
                }
            } else if !has_return {
                self.emit_pop(total_width);
            }
        }

        self.ops.push(TIROp::Return);
        self.ops.push(TIROp::FnEnd);
        self.stack.clear();
    }
}
