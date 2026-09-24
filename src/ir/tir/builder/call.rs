// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Function call dispatch: intrinsic resolution and user-defined calls.

use crate::ast::*;
use crate::span::Spanned;
use crate::tir::TIROp;
use crate::typecheck::MonoInstance;

use super::TIRBuilder;

impl TIRBuilder {
    /// Emit a function call (intrinsic or user-defined).
    pub(crate) fn build_call(
        &mut self,
        name: &str,
        generic_args: &[Spanned<ArraySize>],
        args: &[Spanned<Expr>],
    ) {
        // Evaluate arguments — each pushes a temp.
        for arg in args {
            self.build_expr(&arg.node);
        }

        // Pop all arg temps from the model.
        let arg_count = args.len();
        for _ in 0..arg_count {
            self.stack.pop();
        }

        // Resolve intrinsic name.
        let resolved_name = self.intrinsic_map.get(name).cloned();
        if resolved_name.is_none()
            && (self.fn_return_widths.contains_key(name) || self.generic_fn_defs.contains_key(name))
        {
            self.build_user_call(name, generic_args);
            return;
        }
        let effective_name = resolved_name.as_deref().unwrap_or(name);
        if let Some(&(inputs, outputs)) = self.target_intrinsics.get(effective_name) {
            self.emit_and_push(
                TIROp::TargetCall {
                    name: effective_name.to_string(),
                    inputs,
                    outputs,
                },
                outputs,
            );
            return;
        }
        if let Some((operation, width)) = self.stream_operation(effective_name) {
            self.emit_and_push(operation, width);
            return;
        }

        match effective_name {
            // ── Assertions ──
            "assert" => {
                self.ops.push(TIROp::Assert(1));
                self.push_temp(0);
            }
            "assert_eq" => {
                self.ops.push(TIROp::Eq);
                self.ops.push(TIROp::Assert(1));
                self.push_temp(0);
            }
            "assert_digest" => {
                self.emit_digest_assert();
                self.push_temp(0);
            }

            // ── Field operations ──
            "field_add" => {
                self.ops.push(TIROp::Add);
                self.push_temp(1);
            }
            "field_mul" => {
                self.ops.push(TIROp::Mul);
                self.push_temp(1);
            }
            "inv" => {
                self.ops.push(TIROp::Invert);
                self.push_temp(1);
            }
            "neg" => {
                self.ops.push(TIROp::Neg);
                self.push_temp(1);
            }
            "sub" => {
                self.ops.push(TIROp::Sub);
                self.push_temp(1);
            }

            // ── U32 operations ──
            "split" => {
                self.ops.push(TIROp::Split);
                self.push_temp(2);
            }
            "log2" => {
                self.ops.push(TIROp::Log2);
                self.push_temp(1);
            }
            "pow" => {
                self.ops.push(TIROp::Pow);
                self.push_temp(1);
            }
            "popcount" => {
                self.ops.push(TIROp::PopCount);
                self.push_temp(1);
            }

            // ── Hash operations ──
            "hash" => {
                self.ops.push(TIROp::Hash {
                    width: self.target_config.digest_width,
                });
                self.push_temp(self.target_config.digest_width);
            }
            "sponge_init" => {
                self.ops.push(TIROp::SpongeInit);
                self.push_temp(0);
            }
            "sponge_absorb" => {
                self.ops.push(TIROp::SpongeAbsorb);
                self.push_temp(0);
            }
            "sponge_squeeze" => {
                self.emit_and_push(TIROp::SpongeSqueeze, self.target_config.hash_rate);
            }
            "sponge_absorb_mem" => {
                self.ops.push(TIROp::SpongeLoad);
                self.push_temp(0);
            }

            // ── Merkle ──
            "merkle_step" => {
                self.emit_and_push(TIROp::MerkleStep, self.target_config.digest_width + 1);
            }
            "merkle_step_mem" => {
                self.emit_and_push(TIROp::MerkleLoad, self.target_config.digest_width + 2);
            }

            // ── RAM ──
            "ram_read" => {
                self.ops.push(TIROp::RamRead { width: 1 });
                self.push_temp(1);
            }
            "ram_write" => {
                self.ops.push(TIROp::RamWrite { width: 1 });
                self.push_temp(0);
            }
            "ram_read_block" => {
                self.ops.push(TIROp::RamRead {
                    width: self.target_config.digest_width,
                });
                self.push_temp(self.target_config.digest_width);
            }
            "ram_write_block" => {
                self.ops.push(TIROp::RamWrite {
                    width: self.target_config.digest_width,
                });
                self.push_temp(0);
            }

            // ── Conversion ──
            "as_u32" => {
                self.emit_checked_u32();
                self.push_temp(1);
            }
            "as_field" => {
                self.push_temp(1);
            }

            // ── XField ──
            "xfield" => {
                self.push_temp(self.target_config.xfield_width);
            }
            "xinvert" => {
                self.ops.push(TIROp::ExtInvert);
                self.push_temp(self.target_config.xfield_width);
            }
            "xx_dot_step" => {
                self.emit_and_push(TIROp::FoldExt, self.target_config.xfield_width + 2);
            }
            "xb_dot_step" => {
                self.emit_and_push(TIROp::FoldBase, self.target_config.xfield_width + 2);
            }

            // ── User-defined function ──
            _ => {
                self.build_user_call(name, generic_args);
            }
        }
    }

    /// Emit only the call/intrinsic opcode for a pass-through function.
    /// Does NOT evaluate arguments or touch the stack model — the caller's
    /// params are already in place on the real stack.
    pub(crate) fn emit_call_only(
        &mut self,
        name: &str,
        generic_args: &[Spanned<ArraySize>],
        _arg_count: usize,
    ) {
        let resolved_name = self.intrinsic_map.get(name).cloned();
        if resolved_name.is_none()
            && (self.fn_return_widths.contains_key(name) || self.generic_fn_defs.contains_key(name))
        {
            let label = self.resolve_call_label(name, generic_args);
            self.ops.push(TIROp::Call(label));
            return;
        }
        let effective_name = resolved_name.as_deref().unwrap_or(name);
        if let Some(&(inputs, outputs)) = self.target_intrinsics.get(effective_name) {
            self.ops.push(TIROp::TargetCall {
                name: effective_name.to_string(),
                inputs,
                outputs,
            });
            return;
        }
        if let Some((operation, _)) = self.stream_operation(effective_name) {
            self.ops.push(operation);
            return;
        }

        match effective_name {
            "hash" => {
                self.ops.push(TIROp::Hash {
                    width: self.target_config.digest_width,
                });
            }
            "sponge_init" => self.ops.push(TIROp::SpongeInit),
            "sponge_absorb" => self.ops.push(TIROp::SpongeAbsorb),
            "sponge_squeeze" => self.ops.push(TIROp::SpongeSqueeze),
            "sponge_absorb_mem" => self.ops.push(TIROp::SpongeLoad),
            "assert" => self.ops.push(TIROp::Assert(1)),
            "assert_eq" => {
                self.ops.push(TIROp::Eq);
                self.ops.push(TIROp::Assert(1));
            }
            "as_u32" => self.emit_checked_u32(),
            "as_field" | "xfield" => {}
            "split" => self.ops.push(TIROp::Split),
            "log2" => self.ops.push(TIROp::Log2),
            "pow" => self.ops.push(TIROp::Pow),
            "popcount" => self.ops.push(TIROp::PopCount),
            "inv" => self.ops.push(TIROp::Invert),
            "neg" => self.ops.push(TIROp::Neg),
            "sub" => self.ops.push(TIROp::Sub),
            "field_add" => self.ops.push(TIROp::Add),
            "field_mul" => self.ops.push(TIROp::Mul),
            "ram_read" => self.ops.push(TIROp::RamRead { width: 1 }),
            "ram_write" => self.ops.push(TIROp::RamWrite { width: 1 }),
            "ram_read_block" => self.ops.push(TIROp::RamRead {
                width: self.target_config.digest_width,
            }),
            "ram_write_block" => self.ops.push(TIROp::RamWrite {
                width: self.target_config.digest_width,
            }),
            "merkle_step" => self.ops.push(TIROp::MerkleStep),
            "merkle_step_mem" => self.ops.push(TIROp::MerkleLoad),
            "xinvert" => self.ops.push(TIROp::ExtInvert),
            "xx_dot_step" => self.ops.push(TIROp::FoldExt),
            "xb_dot_step" => self.ops.push(TIROp::FoldBase),
            "assert_digest" => {
                self.emit_digest_assert();
            }
            _ => {
                // User-defined call — resolve label the same way as
                // build_user_call but skip stack model updates.
                let call_label = self.resolve_call_label(name, generic_args);
                self.ops.push(TIROp::Call(call_label));
            }
        }
    }

    fn emit_checked_u32(&mut self) {
        // Split leaves [high, low]; preserve low and require high == 0.
        self.ops.extend([
            TIROp::Split,
            TIROp::Swap(1),
            TIROp::Push(0),
            TIROp::Eq,
            TIROp::Assert(1),
        ]);
    }

    fn stream_operation(&self, name: &str) -> Option<(TIROp, u32)> {
        for prefix in ["pub_read", "pub_write", "divine"] {
            let Some(suffix) = name.strip_prefix(prefix) else {
                continue;
            };
            let width = if suffix.is_empty() {
                1
            } else {
                suffix.parse::<u32>().ok()?
            };
            let valid = if prefix == "divine" {
                width == 1
                    || width == self.target_config.digest_width
                    || width == self.target_config.xfield_width
            } else {
                width > 0 && width <= self.target_config.digest_width
            };
            if !valid || width == 0 {
                return None;
            }
            return Some(match prefix {
                "pub_read" => (TIROp::ReadIo(width), width),
                "pub_write" => (TIROp::WriteIo(width), 0),
                _ => (TIROp::Hint(width), width),
            });
        }
        None
    }

    fn emit_digest_assert(&mut self) {
        let width = self.target_config.digest_width;
        if width == 1 {
            self.ops.extend([TIROp::Eq, TIROp::Assert(1)]);
        } else {
            self.ops.extend([TIROp::Assert(width), TIROp::Pop(width)]);
        }
    }

    /// Resolve a user-defined call name to its TASM label.
    /// Returns `(call_label, base_name)` where `base_name` is used for
    /// return width lookup.
    fn resolve_call_label(&mut self, name: &str, generic_args: &[Spanned<ArraySize>]) -> String {
        let is_generic = self.generic_fn_defs.contains_key(name);

        if is_generic {
            let size_args: Vec<u64> = if !generic_args.is_empty() {
                generic_args
                    .iter()
                    .map(|ga| ga.node.eval(&self.current_subs))
                    .collect()
            } else if !self.current_subs.is_empty() {
                if let Some(gdef) = self.generic_fn_defs.get(name) {
                    gdef.type_params
                        .iter()
                        .map(|p| self.current_subs.get(&p.node).copied().unwrap_or(0))
                        .collect()
                } else {
                    vec![]
                }
            } else {
                let idx = self.call_resolution_idx;
                if idx < self.call_resolutions.len() && self.call_resolutions[idx].name == name {
                    self.call_resolution_idx += 1;
                    self.call_resolutions[idx].size_args.clone()
                } else {
                    let mut found = vec![];
                    for (i, res) in self.call_resolutions.iter().enumerate() {
                        if i >= self.call_resolution_idx && res.name == name {
                            self.call_resolution_idx = i + 1;
                            found = res.size_args.clone();
                            break;
                        }
                    }
                    found
                }
            };
            let inst = MonoInstance {
                name: name.to_string(),
                size_args,
            };
            inst.mangled_name()
        } else if name.contains('.') {
            let canonical = self.qualified_function(name);
            let (full_module, fn_name) = canonical
                .rsplit_once('.')
                .expect("qualified call has module");
            let mangled = full_module.replace('.', "_");
            // @ prefix marks cross-module calls so the linker doesn't re-prefix them
            format!("@{}__{}", mangled, fn_name)
        } else {
            name.to_string()
        }
    }

    /// Emit a call to a user-defined (non-intrinsic) function.
    fn build_user_call(&mut self, name: &str, generic_args: &[Spanned<ArraySize>]) {
        let call_label = self.resolve_call_label(name, generic_args);

        // For return width lookup, use the base name (without module prefix).
        let base_name = if name.contains('.') && !self.generic_fn_defs.contains_key(name) {
            name.rsplitn(2, '.').next().unwrap_or(name).to_string()
        } else {
            call_label.clone()
        };

        let ret_width = self
            .fn_return_types
            .get(&self.qualified_function(name))
            .map(|ty| self.type_width(ty))
            .or_else(|| self.fn_return_widths.get(&base_name).copied())
            .unwrap_or(0);
        if ret_width > 0 {
            self.emit_and_push(TIROp::Call(call_label), ret_width);
        } else {
            self.ops.push(TIROp::Call(call_label));
            self.push_temp(0);
        }
    }
}

#[cfg(test)]
mod abi_tests {
    use super::*;

    #[test]
    fn intrinsic_results_follow_the_typechecker_selected_abi() {
        for (digest, extension) in [(1, 2), (4, 2), (7, 4)] {
            let mut target = crate::target::TerrainConfig::triton();
            target.digest_width = digest;
            target.xfield_width = extension;
            let checker = crate::typecheck::TypeChecker::with_target(target.clone());
            for name in [
                "ram_read_block",
                "merkle_step",
                "merkle_step_mem",
                "xfield",
                "xinvert",
                "xx_dot_step",
                "xb_dot_step",
            ] {
                let signature = &checker.functions[name];
                let mut builder = TIRBuilder::new(target.clone());
                builder.build_call(name, &[], &[]);
                assert_eq!(
                    builder.stack.stack_depth(),
                    signature.return_ty.width().unwrap(),
                    "{name}, digest={digest}, extension={extension}"
                );
                if name == "ram_read_block" {
                    assert!(
                        matches!(builder.ops.as_slice(), [TIROp::RamRead { width }] if *width == digest)
                    );
                }
            }
            let mut builder = TIRBuilder::new(target);
            builder.emit_call_only("assert_digest", &[], 2);
            if digest == 1 {
                assert!(matches!(
                    builder.ops.as_slice(),
                    [TIROp::Eq, TIROp::Assert(1)]
                ));
            } else {
                assert!(
                    matches!(builder.ops.as_slice(), [TIROp::Assert(n), TIROp::Pop(m)] if *n == digest && *m == digest)
                );
            }
        }
    }

    #[test]
    fn stream_intrinsics_cover_the_declared_target_widths_in_both_paths() {
        let mut target = crate::target::TerrainConfig::triton();
        target.digest_width = 9;
        target.xfield_width = 2;
        let checker = crate::typecheck::TypeChecker::with_target(target.clone());
        for (name, signature) in checker.functions.iter().filter(|(name, _)| {
            name.starts_with("pub_read")
                || name.starts_with("pub_write")
                || name.starts_with("divine")
        }) {
            let mut regular = TIRBuilder::new(target.clone());
            regular.build_call(name, &[], &[]);
            let mut pass_through = TIRBuilder::new(target.clone());
            pass_through.emit_call_only(name, &[], signature.params.len());
            assert_eq!(
                regular.stack.stack_depth(),
                signature.return_ty.width().unwrap(),
                "{name}"
            );
            assert_eq!(
                format!("{:?}", regular.ops),
                format!("{:?}", pass_through.ops),
                "{name}"
            );
            assert!(
                !regular.ops.iter().any(|op| matches!(op, TIROp::Call(_))),
                "{name}"
            );
        }
    }
}
