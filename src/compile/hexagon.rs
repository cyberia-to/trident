//! Qualcomm Hexagon DSP text emitter for nox formulas
//!
//! Compiles nox formulas to Hexagon assembly text. Hexagon is a VLIW
//! DSP found in every Qualcomm Snapdragon — 4-wide issue, 32 GPRs
//! (R0-R31), 64-bit register pairs (R1:0, R3:2, etc.), predicate
//! registers P0-P3.
//!
//! Phase 1: 32-bit atom values only. No Goldilocks reduction.
//!
//! Register allocation (Hexagon ABI):
//!   R0-R5:   function arguments (up to 6 params)
//!   R0-R1:   return value (R1:0 for 64-bit)
//!   R6-R15:  scratch registers for intermediates
//!   R16-R27: callee-saved
//!   R28=sp, R29=fp, R30=lr, R31=pc
//!   P0-P3:   predicate registers
//!
//! Hexagon uses VLIW packets: up to 4 instructions grouped in `{ }`.
//! Since encoding VLIW packets into binary is complex (parallel
//! execution bits, duplex instructions, etc.), we emit assembly text
//! and let the assembler handle packet formation.
//!
//! Packet rules:
//! - Up to 4 instructions per packet
//! - No two instructions in a packet can write the same register
//! - We emit single-instruction packets for correctness (Phase 1);
//!   the assembler / optimizer can pack them later



use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const MAX_PARAMS: u32 = 6; // Hexagon ABI: R0-R5

/// Compile a nox formula to Hexagon assembly text.
pub fn compile_to_hexagon<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    if num_params > MAX_PARAMS {
        return Err(CompileError::NoParams);
    }
    let mut e = HexagonEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

struct HexagonEmitter {
    body: String,
    num_params: u32,
    next_scratch: u32,
    next_pred: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    /// Subject model: maps depth -> register name.
    subject: Vec<String>,
    loop_state: Option<HexagonLoopState>,
}

#[derive(Clone)]
struct HexagonLoopState {
    carried: Vec<String>,
    formula_reg: String,
    header_label: String,
}

// Scratch registers: R6-R15 (10 available)
const SCRATCH_BASE: u32 = 6;
const SCRATCH_COUNT: u32 = 10;

impl HexagonEmitter {
    fn new(num_params: u32) -> Self {
        // Subject: params in reverse order (last param = depth 0)
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("R{}", i))
            .collect();
        Self {
            body: String::with_capacity(2048),
            num_params,
            next_scratch: 0,
            next_pred: 0,
            next_label: 0,
            reg_stack: Vec::new(),
            subject,
            loop_state: None,
        }
    }

    fn alloc_scratch(&mut self) -> String {
        let r = format!("R{}", SCRATCH_BASE + (self.next_scratch % SCRATCH_COUNT));
        self.next_scratch += 1;
        r
    }

    fn alloc_pred(&mut self) -> String {
        let p = format!("P{}", self.next_pred % 4);
        self.next_pred += 1;
        p
    }

    fn alloc_label(&mut self) -> String {
        let l = format!(".L{}", self.next_label);
        self.next_label += 1;
        l
    }

    fn push_reg(&mut self) -> String {
        let r = self.alloc_scratch();
        self.reg_stack.push(r.clone());
        r
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "R6".to_string())
    }

    /// Emit a single-instruction packet.
    fn emit_packet(&mut self, insn: &str) {
        self.body.push_str("    { ");
        self.body.push_str(insn);
        self.body.push_str(" }\n");
    }

    /// Emit a multi-instruction packet (up to 4).
    fn emit_packet_multi(&mut self, insns: &[&str]) {
        self.body.push_str("    {\n");
        for insn in insns {
            self.body.push_str("        ");
            self.body.push_str(insn);
            self.body.push('\n');
        }
        self.body.push_str("    }\n");
    }

    fn emit_label(&mut self, label: &str) {
        self.body.push_str(label);
        self.body.push_str(":\n");
    }

    fn emit_formula<const N: usize>(&mut self, order: &Order<N>, formula: NounId) -> Result<(), CompileError> {
        let (tag, body) = formula_parts(order, formula)?;
        match tag {
            0 => self.emit_axis(order, body),
            1 => self.emit_quote(order, body),
            2 => self.emit_compose(order, body),
            4 => self.emit_branch(order, body),
            5 => self.emit_add(order, body),
            6 => self.emit_sub(order, body),
            7 => self.emit_mul(order, body),
            9 => self.emit_eq(order, body),
            10 => self.emit_lt(order, body),
            11 => self.emit_xor(order, body),
            12 => self.emit_and(order, body),
            13 => self.emit_not(order, body),
            14 => self.emit_shl(order, body),
            _ => Err(CompileError::UnsupportedPattern(tag)),
        }
    }

    // ── pattern emitters ──────────────────────────────────────────

    fn emit_axis<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() { return Err(CompileError::NoParams); }
        let src = self.subject[depth as usize].clone();
        let dst = self.push_reg();
        self.emit_packet(&format!("{} = {}", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let dst = self.push_reg();
        if val <= 0x7FFF {
            // Small immediate: fits in transfer-immediate
            self.emit_packet(&format!("{} = #{}", dst, val));
        } else {
            // Large 32-bit immediate: high half + low half
            let lo = val & 0xFFFF;
            let hi = (val >> 16) & 0xFFFF;
            self.emit_packet(&format!("{} = ##0x{:X}", dst, val as u32));
            // Hexagon extended constant (##) handles full 32-bit
            let _ = (lo, hi); // used by ## encoding
        }
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        if let Some((loop_body, inits)) = detect_loop_setup(order, body) {
            return self.emit_loop(order, loop_body, &inits);
        }
        if let Some((new_subj, _)) = detect_back_edge(order, body) {
            return self.emit_back_edge(order, new_subj);
        }
        // Let-binding
        let (a_formula, b_formula) = body_pair(order, body)?;
        let (a_tag, a_body) = formula_parts(order, a_formula)?;
        if a_tag != 3 { return Err(CompileError::UnsupportedPattern(2)); }
        let (value_formula, identity) = body_pair(order, a_body)?;
        let (id_tag, id_body) = formula_parts(order, identity)?;
        if id_tag != 0 || atom_u64(order, id_body)? != 1 {
            return Err(CompileError::UnsupportedPattern(2));
        }
        let (b_tag, body_formula) = formula_parts(order, b_formula)?;
        if b_tag != 1 { return Err(CompileError::UnsupportedPattern(2)); }

        self.emit_formula(order, value_formula)?;
        let val = self.pop_reg();
        self.subject.insert(0, val);
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result
    }

    fn emit_loop<const N: usize>(
        &mut self, order: &Order<N>, loop_body: NounId, inits: &[NounId],
    ) -> Result<(), CompileError> {
        let formula_reg = self.alloc_scratch();
        self.emit_packet(&format!("{} = #0", formula_reg));

        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = self.alloc_scratch();
            self.emit_packet(&format!("{} = {}", cr, val));
            carried.push(cr);
        }

        let saved = self.subject.clone();
        for cr in carried.iter() {
            self.subject.insert(0, cr.clone());
        }
        self.subject.insert(0, formula_reg.clone());

        let header = self.alloc_label();
        let prev = self.loop_state.take();
        self.loop_state = Some(HexagonLoopState {
            carried: carried.clone(),
            formula_reg: formula_reg.clone(),
            header_label: header.clone(),
        });

        self.emit_label(&header);
        self.emit_formula(order, loop_body)?;

        self.loop_state = prev;
        self.subject = saved;
        Ok(())
    }

    fn emit_back_edge<const N: usize>(
        &mut self, order: &Order<N>, new_subj: NounId,
    ) -> Result<(), CompileError> {
        let ls = self.loop_state.as_ref()
            .ok_or(CompileError::UnsupportedPattern(2))?.clone();

        let (tag, cons_body) = formula_parts(order, new_subj)?;
        if tag != 3 { return Err(CompileError::UnsupportedPattern(2)); }
        let (_, rest) = body_pair(order, cons_body)?;

        let mut cur = rest;
        let mut new_vals = Vec::new();
        for _ in ls.carried.iter() {
            let (tag, cb) = formula_parts(order, cur)?;
            if tag != 3 { break; }
            let (val_formula, tail) = body_pair(order, cb)?;
            self.emit_formula(order, val_formula)?;
            new_vals.push(self.pop_reg());
            cur = tail;
        }
        for (i, cr) in ls.carried.iter().enumerate() {
            if i < new_vals.len() {
                self.emit_packet(&format!("{} = {}", cr, new_vals[i]));
            }
        }

        self.emit_packet(&format!("jump {}", ls.header_label));
        let _ = self.push_reg(); // dummy for stack balance
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let pred = self.alloc_pred();
        let lbl_no = self.alloc_label();
        let lbl_end = self.alloc_label();
        let dst = self.alloc_scratch();

        // nox: 0=yes, nonzero=no
        self.emit_packet(&format!("{} = cmp.eq({}, #0)", pred, test_r));
        self.emit_packet(&format!("if (!{}) jump:t {}", pred, lbl_no));

        // yes path (test==0)
        self.emit_formula(order, yes)?;
        let yes_r = self.pop_reg();
        self.emit_packet(&format!("{} = {}", dst, yes_r));
        self.emit_packet(&format!("jump {}", lbl_end));

        // no path
        self.emit_label(&lbl_no);
        self.emit_formula(order, no)?;
        let no_r = self.pop_reg();
        self.emit_packet(&format!("{} = {}", dst, no_r));

        self.emit_label(&lbl_end);
        self.reg_stack.push(dst);
        Ok(())
    }

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // 32-bit wrapping add
        self.emit_packet(&format!("{} = add({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // 32-bit wrapping sub
        self.emit_packet(&format!("{} = sub({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // MPY: 32x32 → lower 32 bits
        self.emit_packet(&format!("{} = mpyi({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let pred = self.alloc_pred();
        // nox eq: 0 if equal, 1 if not
        // Compare, then use predicated mux
        self.emit_packet(&format!("{} = cmp.eq({}, {})", pred, ra, rb));
        self.emit_packet_multi(&[
            &format!("if ({}) {} = #0", pred, dst),
            &format!("if (!{}) {} = #1", pred, dst),
        ]);
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        let pred = self.alloc_pred();
        // nox lt: 0 if a<b (unsigned), 1 if a>=b
        self.emit_packet(&format!("{} = cmp.gtu({}, {})", pred, ra, rb));
        // P is true when ra > rb (strictly greater, unsigned)
        // So: a < b → !(a >= b) → !(a > b || a == b)
        // Use cmp.gtu: true if ra > rb
        // if ra < rb: pred=false → dst=0 ✓
        // if ra >= rb: need dst=1
        // Actually: use two predicates or mux
        let pred2 = self.alloc_pred();
        self.emit_packet(&format!("{} = cmp.eq({}, {})", pred2, ra, rb));
        // a >= b means (a > b) OR (a == b)
        self.emit_packet(&format!("{} = or({}, {})", pred, pred, pred2));
        self.emit_packet_multi(&[
            &format!("if ({}) {} = #1", pred, dst),
            &format!("if (!{}) {} = #0", pred, dst),
        ]);
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit_packet(&format!("{} = xor({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit_packet(&format!("{} = and({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_packet(&format!("{} = not({})", dst, ra));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // ASL = arithmetic/logical shift left
        self.emit_packet(&format!("{} = asl({}, {})", dst, ra, rb));
        Ok(())
    }

    // ── output ─────────────────────────────────────────────────────

    fn finish(self, result: &str) -> String {
        let mut out = String::with_capacity(4096);

        // Header
        out.push_str("// Hexagon VLIW assembly — generated by trident\n");
        out.push_str("// Target: Qualcomm Hexagon V60+\n\n");

        out.push_str("    .text\n");
        out.push_str("    .globl main\n");
        out.push_str("    .type main, @function\n");
        out.push_str("main:\n");

        // Body (already formatted with packets)
        out.push_str(&self.body);

        // Move result to R0 if not already there
        if result != "R0" {
            out.push_str(&format!("    {{ R0 = {} }}\n", result));
        }

        // Return
        out.push_str("    { jumpr R31 }\n");
        out
    }
}
