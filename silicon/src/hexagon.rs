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
    if e.scratch_overflow {
        return Err(CompileError::ScratchOverflow);
    }
    let result = e.pop_reg();
    Ok(e.finish(&result))
}

struct HexagonEmitter {
    body: String,
    #[allow(dead_code)]
    num_params: u32,
    next_scratch: u32,
    next_pred: u32,
    next_label: u32,
    reg_stack: Vec<String>,
    /// Subject model: maps depth -> register name.
    subject: Vec<String>,
    loop_state: Option<HexagonLoopState>,
    /// Set when a register was handed out while all scratch registers were allocated
    scratch_overflow: bool,
}

#[derive(Clone)]
struct HexagonLoopState {
    carried: Vec<String>,
    #[allow(dead_code)]
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
            scratch_overflow: false,
        }
    }

    fn alloc_scratch(&mut self) -> String {
        if self.next_scratch >= SCRATCH_COUNT {
            self.scratch_overflow = true;
        }
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
        self.next_scratch = self.next_scratch.saturating_sub(1);
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

        // The bound value stays on the register stack while the body runs;
        // afterwards the body result takes its place.
        self.emit_formula(order, value_formula)?;
        let val = self.reg_stack.last().cloned().unwrap_or_else(|| "R6".to_string());
        self.subject.insert(0, val);
        let result = self.emit_formula(order, body_formula);
        self.subject.remove(0);
        result?;
        let res = self.pop_reg();
        self.pop_reg(); // the bound value
        let dst = self.push_reg();
        if dst != res {
            self.emit_packet(&format!("{} = {}", dst, res));
        }
        Ok(())
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
        let mut evaluated = 0;
        for _ in ls.carried.iter() {
            let (tag, cb) = formula_parts(order, cur)?;
            if tag != 3 { break; }
            let (val_formula, tail) = body_pair(order, cb)?;
            self.emit_formula(order, val_formula)?;
            evaluated += 1;
            cur = tail;
        }
        // Every new value stays live until all are computed; pop in reverse.
        let mut new_vals: Vec<String> = (0..evaluated).map(|_| self.pop_reg()).collect();
        new_vals.reverse();
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
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // 32-bit wrapping add
        self.emit_packet(&format!("{} = add({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // 32-bit wrapping sub
        self.emit_packet(&format!("{} = sub({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        // MPY: 32x32 → lower 32 bits
        self.emit_packet(&format!("{} = mpyi({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
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
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
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
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit_packet(&format!("{} = xor({}, {})", dst, ra, rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
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
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let ra = self.pop_reg();
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

#[cfg(test)]
mod anclas_sil1 {
    use super::*;
    use nebu::Goldilocks;

    fn atomo(o: &mut Order<4096>, v: u64) -> NounId { o.atom(Goldilocks::new(v)).unwrap() }
    fn axis(o: &mut Order<4096>, ax: u64) -> NounId {
        let t = atomo(o, 0);
        let a = atomo(o, ax);
        o.pair(t, a).unwrap()
    }
    fn binaria(o: &mut Order<4096>, tag: u64, l: NounId, r: NounId) -> NounId {
        let body = o.pair(l, r).unwrap();
        let t = atomo(o, tag);
        o.pair(t, body).unwrap()
    }
    /// Suma anidada a la derecha con `vivos` hojas vivas a la vez: cada `a` queda
    /// en su registro mientras se emite `b` (vivos=1 es una hoja; vivos=n son n-1 sumas).
    fn suma_viva(o: &mut Order<4096>, vivos: u32) -> NounId {
        let mut f = axis(o, 2);
        for _ in 1..vivos {
            let hoja = axis(o, 2);
            f = binaria(o, 5, hoja, f);
        }
        f
    }
    fn es_scratch_overflow<T>(r: &Result<T, CompileError>) -> bool {
        matches!(r, Err(CompileError::ScratchOverflow))
    }

    #[test]
    fn suma_con_capacidad_mas_uno_vivos_devuelve_scratch_overflow() {
        assert_eq!(SCRATCH_COUNT, 10);
        let mut o = Order::<4096>::new();
        let f = suma_viva(&mut o, SCRATCH_COUNT + 1);
        let r = compile_to_hexagon(&o, f, 1);
        assert!(es_scratch_overflow(&r), "SIL-1.A: 11 valores vivos con 10 scratch deben dar ScratchOverflow, dio {:?}", r.as_ref().map(|c| c.len()));
    }

    #[test]
    fn suma_con_hasta_capacidad_vivos_sigue_compilando() {
        for vivos in [SCRATCH_COUNT - 1, SCRATCH_COUNT] {
            let mut o = Order::<4096>::new();
            let f = suma_viva(&mut o, vivos);
            assert!(compile_to_hexagon(&o, f, 1).is_ok(), "{vivos} vivos caben en {SCRATCH_COUNT} scratch");
        }
    }

    // ── grupo A2: el asignador de hexagon debe liberar al consumir ─────────

    fn cita(o: &mut Order<4096>, v: u64) -> NounId {
        let t = atomo(o, 1);
        let a = atomo(o, v);
        o.pair(t, a).unwrap()
    }

    /// Interprete minimo del texto emitido para formulas sin saltos: ejecuta
    /// `{ Rd = ... }` sobre 32 registros u32 (semantica de 32 bits de la fase 1)
    /// y devuelve R0 al llegar a `jumpr R31`.
    fn ejecuta(asm: &str, params: &[u32]) -> u32 {
        let mut r = [0u32; 32];
        for (i, p) in params.iter().enumerate() {
            r[i] = *p;
        }
        fn reg(s: &str) -> usize { s.trim().trim_start_matches('R').parse().expect("registro") }
        fn val(r: &[u32; 32], s: &str) -> u32 {
            let s = s.trim();
            if let Some(h) = s.strip_prefix("##0x") {
                u32::from_str_radix(h, 16).unwrap()
            } else if let Some(d) = s.strip_prefix('#') {
                d.parse().unwrap()
            } else {
                r[reg(s)]
            }
        }
        for linea in asm.lines() {
            let l = linea.trim().trim_start_matches('{').trim_end_matches('}').trim();
            if l.is_empty() || l.starts_with("//") || l.starts_with('.') || l.ends_with(':') {
                continue;
            }
            if l == "jumpr R31" {
                return r[0];
            }
            let (dst, rhs) = l.split_once('=').expect("asignacion");
            let rhs = rhs.trim();
            let nuevo = if let Some(args) = rhs.strip_suffix(')') {
                let (op, a) = args.split_once('(').expect("operacion");
                let v: Vec<u32> = a.split(',').map(|x| val(&r, x)).collect();
                match op {
                    "add" => v[0].wrapping_add(v[1]),
                    "sub" => v[0].wrapping_sub(v[1]),
                    "mpyi" => v[0].wrapping_mul(v[1]),
                    "xor" => v[0] ^ v[1],
                    "and" => v[0] & v[1],
                    "asl" => v[0].wrapping_shl(v[1]),
                    "not" => !v[0],
                    _ => panic!("operacion no soportada por el interprete: {op}"),
                }
            } else {
                val(&r, rhs)
            };
            r[reg(dst)] = nuevo;
        }
        panic!("el codigo no termina en jumpr R31")
    }

    /// Suma a la derecha de `vivos` citas distintas 1..=vivos, todas vivas a la vez.
    fn suma_viva_citas(o: &mut Order<4096>, vivos: u32) -> NounId {
        let mut f = cita(o, vivos as u64);
        for i in (1..vivos).rev() {
            let hoja = cita(o, i as u64);
            f = binaria(o, 5, hoja, f);
        }
        f
    }

    /// SIL-1, grupo A2: pop_reg no decrementaba next_scratch; tras 10 asignaciones
    /// TOTALES se reciclaba R6 aunque la profundidad viva fuera 2. Con el
    /// overflow de A, una cadena larga a profundidad baja daba Err espurio.
    #[test]
    fn cadena_larga_de_sumas_a_profundidad_baja_compila_y_suma() {
        let mut o = Order::<4096>::new();
        let mut f = cita(&mut o, 1);
        for i in 2..=16u64 {
            let hoja = cita(&mut o, i);
            f = binaria(&mut o, 5, f, hoja);
        }
        let asm = compile_to_hexagon(&o, f, 0).expect("profundidad viva 2: no es overflow");
        assert_eq!(ejecuta(&asm, &[]), 136);
    }

    /// Con capacidad exacta de valores vivos todos los registros son distintos y el resultado es el de referencia.
    #[test]
    fn suma_con_capacidad_vivos_da_el_valor_de_referencia() {
        let mut o = Order::<4096>::new();
        let f = suma_viva_citas(&mut o, SCRATCH_COUNT);
        let asm = compile_to_hexagon(&o, f, 0).expect("capacidad exacta");
        assert_eq!(ejecuta(&asm, &[]), 55);
    }

    /// Guarda: liberar al consumir exige que los operandos sigan vivos mientras se emite el otro.
    /// sub(100, add(7, mul(3, 4))) = 81.
    #[test]
    fn binarios_anidados_dan_el_valor_de_referencia() {
        let mut o = Order::<4096>::new();
        let (c3, c4, c7, c100) = (cita(&mut o, 3), cita(&mut o, 4), cita(&mut o, 7), cita(&mut o, 100));
        let m = binaria(&mut o, 7, c3, c4);
        let s = binaria(&mut o, 5, c7, m);
        let f = binaria(&mut o, 6, c100, s);
        let asm = compile_to_hexagon(&o, f, 0).unwrap();
        assert_eq!(ejecuta(&asm, &[]), 81);
    }

    /// Guarda: add sobre dos parametros no puede leer el mismo registro dos veces.
    #[test]
    fn add_de_dos_parametros_lee_registros_distintos() {
        let mut o = Order::<4096>::new();
        let a0 = axis(&mut o, 2);
        let a1 = axis(&mut o, 6);
        let f = binaria(&mut o, 5, a0, a1);
        let asm = compile_to_hexagon(&o, f, 2).unwrap();
        assert_eq!(ejecuta(&asm, &[1000, 20]), 1020);
        let linea = asm.lines().find(|l| l.contains("= add(")).expect("add");
        let args = linea.split_once("add(").unwrap().1.trim_end_matches([' ', '}', ')']);
        let (ra, rb) = args.split_once(',').unwrap();
        assert_ne!(ra.trim(), rb.trim(), "operandos en el mismo registro: {linea}");
    }

    /// Guarda: el valor ligado por un let sigue vivo mientras se compila el cuerpo.
    /// let x = 3+4 in 5+x = 12.
    #[test]
    fn let_conserva_el_valor_ligado_durante_el_cuerpo() {
        let mut o = Order::<4096>::new();
        let (c3, c4, c5) = (cita(&mut o, 3), cita(&mut o, 4), cita(&mut o, 5));
        let valor = binaria(&mut o, 5, c3, c4);
        let uno = axis(&mut o, 1);
        let t3 = atomo(&mut o, 3);
        let cons = { let b = o.pair(valor, uno).unwrap(); o.pair(t3, b).unwrap() };
        let x = axis(&mut o, 2);
        let cuerpo = binaria(&mut o, 5, c5, x);
        let t1 = atomo(&mut o, 1);
        let cuerpo_citado = o.pair(t1, cuerpo).unwrap();
        let f = binaria(&mut o, 2, cons, cuerpo_citado);
        let asm = compile_to_hexagon(&o, f, 0).expect("let");
        assert_eq!(ejecuta(&asm, &[]), 12);
    }

    /// Guarda: los nuevos valores de un back-edge con dos carried locals salen de registros distintos.
    #[test]
    fn back_edge_con_dos_carried_asigna_desde_registros_distintos() {
        let mut o = Order::<4096>::new();
        // loop: [2 [[3 [[1 0] [3 [i0 [3 [i1 [0 1]]]]]]] [1 back_edge]]]
        let (i0, i1) = (cita(&mut o, 1), cita(&mut o, 2));
        let uno = axis(&mut o, 1);
        let t3 = atomo(&mut o, 3);
        let fin_inits = { let b = o.pair(i1, uno).unwrap(); o.pair(t3, b).unwrap() };
        let inits = { let b = o.pair(i0, fin_inits).unwrap(); o.pair(t3, b).unwrap() };
        let ranura = cita(&mut o, 0);
        let a = { let b = o.pair(ranura, inits).unwrap(); o.pair(t3, b).unwrap() };
        // back-edge: [2 [[3 [[1 0] [3 [v0 [3 [v1 [0 1]]]]]]] [0 2]]]
        let (v0, v1) = (cita(&mut o, 10), cita(&mut o, 20));
        let fin_vals = { let b = o.pair(v1, uno).unwrap(); o.pair(t3, b).unwrap() };
        let vals = { let b = o.pair(v0, fin_vals).unwrap(); o.pair(t3, b).unwrap() };
        let ranura2 = cita(&mut o, 0);
        let nuevo_sujeto = { let b = o.pair(ranura2, vals).unwrap(); o.pair(t3, b).unwrap() };
        let eje = axis(&mut o, 2);
        let back_edge = binaria(&mut o, 2, nuevo_sujeto, eje);
        let t1 = atomo(&mut o, 1);
        let cuerpo = o.pair(t1, back_edge).unwrap();
        let f = binaria(&mut o, 2, a, cuerpo);
        let asm = compile_to_hexagon(&o, f, 0).expect("loop");
        let lineas: Vec<&str> = asm.lines().collect();
        let j = lineas.iter().position(|l| l.contains("jump .L")).expect("back-edge");
        let fuente = |l: &str| l.split_once('=').unwrap().1.trim().trim_end_matches('}').trim().to_string();
        let (s0, s1) = (fuente(lineas[j - 2]), fuente(lineas[j - 1]));
        assert_ne!(s0, s1, "ambos carried se asignan desde {s0}: el segundo valor piso al primero");
    }
}
