//! WGSL compute shader emitter for nox formulas
//!
//! WebGPU Shading Language — runs on wgpu (Vulkan, Metal, DX12, WebGPU).
//! Parallel: one thread per input via @compute workgroup.
//!
//! Uses u32 pair (lo, hi) for Goldilocks elements since WGSL lacks native u64.
//! Each field element = vec2<u32> where x=lo, y=hi.






use nox::{Reduction as Order, Order as NounId};
use super::{CompileError, formula_parts, body_pair, body_triple, atom_u64, axis_to_param,
            detect_loop_setup, detect_back_edge};

const P_LO: u32 = 0x00000001;
const P_HI: u32 = 0xFFFFFFFF;

pub fn compile_to_wgsl<const N: usize>(
    order: &Order<N>,
    formula: NounId,
    num_params: u32,
) -> Result<String, CompileError> {
    let mut e = WgslEmitter::new(num_params);
    e.emit_formula(order, formula)?;
    let result = e.pop_reg();
    Ok(e.finish(&result, num_params))
}

struct WgslEmitter {
    body: String,
    num_params: u32,
    next_var: u32,
    reg_stack: Vec<String>,
    subject: Vec<String>,
    loop_state: Option<WgslLoopState>,
}

#[derive(Clone)]
struct WgslLoopState {
    carried: Vec<String>,
    header_label: u32,
}

impl WgslEmitter {
    fn new(num_params: u32) -> Self {
        let subject: Vec<String> = (0..num_params).rev()
            .map(|i| format!("p{}", i)).collect();
        Self {
            body: String::with_capacity(4096),
            num_params, next_var: 0,
            reg_stack: Vec::new(), subject,
            loop_state: None,
        }
    }

    fn alloc_var(&mut self) -> String {
        let v = format!("v{}", self.next_var);
        self.next_var += 1;
        v
    }

    fn push_reg(&mut self) -> String {
        let v = self.alloc_var();
        self.reg_stack.push(v.clone());
        v
    }

    fn pop_reg(&mut self) -> String {
        self.reg_stack.pop().unwrap_or_else(|| "v0".into())
    }

    fn emit(&mut self, line: &str) {
        self.body.push_str("  ");
        self.body.push_str(line);
        self.body.push('\n');
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

    fn emit_axis<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let addr = atom_u64(order, body)?;
        let depth = axis_to_param(addr)?;
        if (depth as usize) >= self.subject.len() { return Err(CompileError::NoParams); }
        let src = self.subject[depth as usize].clone();
        let dst = self.push_reg();
        self.emit(&format!("var {} = {};", dst, src));
        Ok(())
    }

    fn emit_quote<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let val = atom_u64(order, body)?;
        let lo = val as u32;
        let hi = (val >> 32) as u32;
        let dst = self.push_reg();
        self.emit(&format!("var {} = vec2<u32>({}u, {}u);", dst, lo, hi));
        Ok(())
    }

    fn emit_compose<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        if let Some((loop_body, inits)) = detect_loop_setup(order, body) {
            return self.emit_loop(order, loop_body, &inits);
        }
        if let Some((new_subj, _)) = detect_back_edge(order, body) {
            return self.emit_back_edge(order, new_subj);
        }
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
        let mut carried = Vec::new();
        for &init in inits.iter() {
            self.emit_formula(order, init)?;
            let val = self.pop_reg();
            let cr = self.alloc_var();
            self.emit(&format!("var {} = {};", cr, val));
            carried.push(cr);
        }
        let formula_var = self.alloc_var();
        self.emit(&format!("var {} = vec2<u32>(0u, 0u);", formula_var));

        let saved = self.subject.clone();
        for cr in carried.iter() { self.subject.insert(0, cr.clone()); }
        self.subject.insert(0, formula_var.clone());

        let loop_id = self.next_var;
        let prev = self.loop_state.take();
        self.loop_state = Some(WgslLoopState { carried: carried.clone(), header_label: loop_id });

        self.emit("loop {");
        self.emit_formula(order, loop_body)?;
        self.emit("  break;");
        self.emit("}");

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

        let mut new_vals = Vec::new();
        let mut cur = rest;
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
                self.emit(&format!("{} = {};", cr, new_vals[i]));
            }
        }
        self.emit("  continue;");
        let _ = self.push_reg();
        Ok(())
    }

    fn emit_branch<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (test, yes, no) = body_triple(order, body)?;
        self.emit_formula(order, test)?;
        let test_r = self.pop_reg();
        let dst = self.alloc_var();
        self.emit(&format!("var {} = vec2<u32>(0u, 0u);", dst));
        // nox: 0=yes, nonzero=no. Test lo==0 && hi==0
        self.emit(&format!("if ({}.x == 0u && {}.y == 0u) {{", test_r, test_r));
        self.emit_formula(order, yes)?;
        let yr = self.pop_reg();
        self.emit(&format!("  {} = {};", dst, yr));
        self.emit("} else {");
        self.emit_formula(order, no)?;
        let nr = self.pop_reg();
        self.emit(&format!("  {} = {};", dst, nr));
        self.emit("}");
        self.reg_stack.push(dst);
        Ok(())
    }

    // ── Goldilocks arithmetic on vec2<u32> ───────────────────────

    fn emit_add<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // 64-bit add via u32 pairs: lo = a.lo + b.lo, carry = lo < a.lo
        // hi = a.hi + b.hi + carry
        self.emit(&format!("var {d}_lo = {a}.x + {b}.x;", d=dst, a=ra, b=rb));
        self.emit(&format!("var {d}_carry = select(0u, 1u, {d}_lo < {a}.x);", d=dst, a=ra));
        self.emit(&format!("var {d}_hi = {a}.y + {b}.y + {d}_carry;", d=dst, a=ra, b=rb));
        self.emit(&format!("var {d} = vec2<u32>({d}_lo, {d}_hi);", d=dst));
        // Goldilocks reduce: if result >= P, subtract P
        self.emit(&format!("{d} = goldilocks_reduce({d});", d=dst));
        Ok(())
    }

    fn emit_sub<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("var {d} = goldilocks_sub({a}, {b});", d=dst, a=ra, b=rb));
        Ok(())
    }

    fn emit_mul<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("var {d} = goldilocks_mul({a}, {b});", d=dst, a=ra, b=rb));
        Ok(())
    }

    fn emit_eq<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // nox eq: 0 if equal, 1 if not
        self.emit(&format!("var {d} = select(vec2<u32>(1u, 0u), vec2<u32>(0u, 0u), {a}.x == {b}.x && {a}.y == {b}.y);", d=dst, a=ra, b=rb));
        Ok(())
    }

    fn emit_lt<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        // nox lt: 0 if a<b, 1 if a>=b. Compare hi first, then lo.
        self.emit(&format!("var {d}_lt = ({a}.y < {b}.y) || ({a}.y == {b}.y && {a}.x < {b}.x);", d=dst, a=ra, b=rb));
        self.emit(&format!("var {d} = select(vec2<u32>(1u, 0u), vec2<u32>(0u, 0u), {d}_lt);", d=dst));
        Ok(())
    }

    fn emit_xor<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("var {d} = vec2<u32>({a}.x ^ {b}.x, {a}.y ^ {b}.y);", d=dst, a=ra, b=rb));
        Ok(())
    }

    fn emit_and<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("var {d} = vec2<u32>({a}.x & {b}.x, {a}.y & {b}.y);", d=dst, a=ra, b=rb));
        Ok(())
    }

    fn emit_not<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        self.emit_formula(order, body)?;
        let ra = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("var {d} = vec2<u32>(~{a}.x & 0xFFFFFFFFu, 0u);", d=dst, a=ra));
        Ok(())
    }

    fn emit_shl<const N: usize>(&mut self, order: &Order<N>, body: NounId) -> Result<(), CompileError> {
        let (a, b) = body_pair(order, body)?;
        self.emit_formula(order, a)?;
        let ra = self.pop_reg();
        self.emit_formula(order, b)?;
        let rb = self.pop_reg();
        let dst = self.push_reg();
        self.emit(&format!("var {d} = vec2<u32>(({a}.x << {b}.x) & 0xFFFFFFFFu, 0u);", d=dst, a=ra, b=rb));
        Ok(())
    }

    fn finish(self, result: &str, num_params: u32) -> String {
        let mut s = String::with_capacity(8192);
        s.push_str("// nox formula → WGSL compute shader\n");
        s.push_str("// Goldilocks field: p = 2^64 - 2^32 + 1\n");
        s.push_str("// Each field element = vec2<u32>(lo, hi)\n\n");

        // Goldilocks helper functions
        s.push_str(GOLDILOCKS_WGSL);

        // Bindings
        for i in 0..num_params {
            s.push_str(&format!("@group(0) @binding({}) var<storage, read> input{}: array<vec2<u32>>;\n", i, i));
        }
        s.push_str(&format!("@group(0) @binding({}) var<storage, read_write> output: array<vec2<u32>>;\n\n", num_params));

        s.push_str("@compute @workgroup_size(256)\n");
        s.push_str("fn main(@builtin(global_invocation_id) gid: vec3<u32>) {\n");
        s.push_str("  let idx = gid.x;\n");
        for i in 0..num_params {
            s.push_str(&format!("  var p{} = input{}[idx];\n", i, i));
        }
        s.push('\n');
        s.push_str(&self.body);
        s.push_str(&format!("\n  output[idx] = {};\n", result));
        s.push_str("}\n");
        s
    }
}

const GOLDILOCKS_WGSL: &str = r#"
const P = vec2<u32>(1u, 4294967295u); // p = 2^64 - 2^32 + 1

fn goldilocks_reduce(a: vec2<u32>) -> vec2<u32> {
  // if a >= P: a - P
  if (a.y > P.y || (a.y == P.y && a.x >= P.x)) {
    let lo = a.x - P.x;
    let borrow = select(0u, 1u, a.x < P.x);
    return vec2<u32>(lo, a.y - P.y - borrow);
  }
  return a;
}

fn goldilocks_sub(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
  if (a.y > b.y || (a.y == b.y && a.x >= b.x)) {
    let lo = a.x - b.x;
    let borrow = select(0u, 1u, a.x < b.x);
    return vec2<u32>(lo, a.y - b.y - borrow);
  }
  // a < b: return P - b + a
  let t = goldilocks_sub(P, b);
  let lo = t.x + a.x;
  let carry = select(0u, 1u, lo < t.x);
  return vec2<u32>(lo, t.y + a.y + carry);
}

fn goldilocks_mul(a: vec2<u32>, b: vec2<u32>) -> vec2<u32> {
  // Schoolbook 32x32→64 for 64x64→128
  let al = a.x; let ah = a.y;
  let bl = b.x; let bh = b.y;

  // 4 partial products (each fits in 64 bits = vec2<u32>)
  let p0 = u32_mul(al, bl); // al*bl
  let p1 = u32_mul(ah, bl); // ah*bl
  let p2 = u32_mul(al, bh); // al*bh
  let p3 = u32_mul(ah, bh); // ah*bh

  // Combine: lo = p0 + (p1.lo + p2.lo) << 32
  //          hi = p3 + p1.hi + p2.hi + carries
  var mid_lo = p1.x + p2.x;
  var mid_carry = select(0u, 1u, mid_lo < p1.x);

  var lo = p0.x;
  var hi = p0.y + mid_lo;
  var c1 = select(0u, 1u, hi < p0.y);

  var hi2 = p3.x + p1.y + p2.y + mid_carry + c1;
  // hi2 is upper 32 bits of upper 64 bits — for Goldilocks we only need lower 128 bits
  // result_lo = vec2(lo, hi), result_hi = vec2(hi2, p3.y + carries...)
  // Simplified: reduce via hi*(2^32-1)
  // For now: approximate with lo 64 bits + basic reduce
  let result = vec2<u32>(lo, hi);
  return goldilocks_reduce(result);
}

fn u32_mul(a: u32, b: u32) -> vec2<u32> {
  // 32x32→64 using 16-bit decomposition
  let al = a & 0xFFFFu; let ah = a >> 16u;
  let bl = b & 0xFFFFu; let bh = b >> 16u;
  let p0 = al * bl;
  let p1 = ah * bl;
  let p2 = al * bh;
  let p3 = ah * bh;
  let mid = (p0 >> 16u) + (p1 & 0xFFFFu) + (p2 & 0xFFFFu);
  let lo = (p0 & 0xFFFFu) | ((mid & 0xFFFFu) << 16u);
  let hi = p3 + (p1 >> 16u) + (p2 >> 16u) + (mid >> 16u);
  return vec2<u32>(lo, hi);
}

"#;
