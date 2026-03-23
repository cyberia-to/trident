// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! StackState: concrete TASM execution on Goldilocks field values.

use crate::field::goldilocks::Goldilocks;
use crate::field::PrimeField;

/// Stack state after executing a TASM sequence.
/// Tracks side-channel logs alongside the stack so verification can
/// detect removal/substitution of I/O, assertion, and divine ops.
#[derive(Clone, Debug)]
pub struct StackState {
    pub stack: Vec<u64>,
    pub error: bool,
    pub halted: bool,
    pub io_output: Vec<u64>,
    pub divine_log: Vec<usize>,
    pub assert_log: Vec<u64>,
    pub assert_vector_log: Vec<Vec<u64>>,
}

impl StackState {
    pub fn new(initial: Vec<u64>) -> Self {
        Self {
            stack: initial,
            error: false,
            halted: false,
            io_output: Vec::new(),
            divine_log: Vec::new(),
            assert_log: Vec::new(),
            assert_vector_log: Vec::new(),
        }
    }

    /// Execute a sequence of TASM lines. Stops on error or halt.
    pub fn execute(&mut self, lines: &[String]) {
        for line in lines {
            if self.error || self.halted {
                return;
            }
            self.execute_line(line);
        }
    }

    /// Check if execution completed without errors.
    pub fn is_valid(&self) -> bool {
        !self.error
    }

    /// Execute a single TASM instruction line.
    pub fn execute_line(&mut self, line: &str) {
        let t = line.trim();
        if t.is_empty() || t.starts_with("//") || t.ends_with(':') {
            return;
        }
        let parts: Vec<&str> = t.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        let op = parts[0];
        let arg = parts.get(1).and_then(|s| s.parse::<i64>().ok());
        let arg_u = parts.get(1).and_then(|s| s.parse::<u64>().ok());

        match op {
            // --- Literals ---
            "push" => {
                let val = if let Some(v) = arg {
                    if v < 0 {
                        Goldilocks::from_u64(0)
                            .sub(Goldilocks::from_u64((-v) as u64))
                            .to_u64()
                    } else {
                        Goldilocks::from_u64(v as u64).to_u64()
                    }
                } else if let Some(v) = arg_u {
                    // Large positive literal (exceeds i64 range)
                    Goldilocks::from_u64(v).to_u64()
                } else {
                    0
                };
                self.stack.push(val);
            }

            // --- Stack manipulation ---
            "pop" => {
                let n = arg_u.unwrap_or(1) as usize;
                if self.stack.len() < n {
                    self.error = true;
                    return;
                }
                self.stack.truncate(self.stack.len() - n);
            }
            "dup" => {
                let depth = arg_u.unwrap_or(0) as usize;
                if self.stack.len() <= depth {
                    self.error = true;
                    return;
                }
                let idx = self.stack.len() - 1 - depth;
                let val = self.stack[idx];
                self.stack.push(val);
            }
            "swap" => {
                let depth = arg_u.unwrap_or(1) as usize;
                if depth == 0 || self.stack.len() <= depth {
                    self.error = true;
                    return;
                }
                let top = self.stack.len() - 1;
                self.stack.swap(top, top - depth);
            }
            "pick" => {
                let depth = arg_u.unwrap_or(0) as usize;
                if self.stack.len() <= depth {
                    self.error = true;
                    return;
                }
                let idx = self.stack.len() - 1 - depth;
                let val = self.stack.remove(idx);
                self.stack.push(val);
            }
            "place" => {
                let depth = arg_u.unwrap_or(0) as usize;
                if self.stack.is_empty() || self.stack.len() <= depth {
                    self.error = true;
                    return;
                }
                let val = self.stack.pop().unwrap();
                let idx = self.stack.len() - depth;
                self.stack.insert(idx, val);
            }

            // --- Arithmetic (Goldilocks field) ---
            "add" => {
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let b = Goldilocks(self.stack.pop().unwrap());
                let a = Goldilocks(self.stack.pop().unwrap());
                self.stack.push(a.add(b).to_u64());
            }
            "mul" => {
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let b = Goldilocks(self.stack.pop().unwrap());
                let a = Goldilocks(self.stack.pop().unwrap());
                self.stack.push(a.mul(b).to_u64());
            }
            "invert" => {
                // BUG: this implements negation, but Triton VM invert is
                // multiplicative inverse (1/x mod p). Kept as-is for
                // baseline simulation; excluded from ALLOWED candidate list.
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let a = Goldilocks(self.stack.pop().unwrap());
                self.stack.push(a.neg().to_u64());
            }

            // --- Comparison ---
            "eq" => {
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(if a == b { 1 } else { 0 });
            }
            "lt" => {
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(if a < b { 1 } else { 0 });
            }

            // --- Bitwise ---
            "and" => {
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(a & b);
            }
            "xor" => {
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let b = self.stack.pop().unwrap();
                let a = self.stack.pop().unwrap();
                self.stack.push(a ^ b);
            }
            "split" => {
                // x → (hi, lo) where hi = x >> 32, lo = x & 0xFFFFFFFF
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let x = self.stack.pop().unwrap();
                let lo = x & 0xFFFF_FFFF;
                let hi = x >> 32;
                self.stack.push(hi);
                self.stack.push(lo);
            }
            "div_mod" => {
                // (n, d) → (q, r) where q = n/d, r = n%d
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let d = self.stack.pop().unwrap();
                let n = self.stack.pop().unwrap();
                if d == 0 {
                    self.error = true;
                    return;
                }
                self.stack.push(n / d);
                self.stack.push(n % d);
            }
            "pow" => {
                // (base, exp) → base^exp mod p
                if self.stack.len() < 2 {
                    self.error = true;
                    return;
                }
                let exp = self.stack.pop().unwrap();
                let base = Goldilocks(self.stack.pop().unwrap());
                let mut result = Goldilocks::ONE;
                let mut b = base;
                let mut e = exp;
                while e > 0 {
                    if e & 1 == 1 {
                        result = result.mul(b);
                    }
                    b = b.mul(b);
                    e >>= 1;
                }
                self.stack.push(result.to_u64());
            }
            "log_2_floor" => {
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let x = self.stack.pop().unwrap();
                if x == 0 {
                    self.error = true;
                    return;
                }
                self.stack.push(63 - x.leading_zeros() as u64);
            }
            "pop_count" => {
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let x = self.stack.pop().unwrap();
                self.stack.push(x.count_ones() as u64);
            }

            // --- Control (straight-line only) ---
            "nop" => {}
            "halt" => {
                self.halted = true;
                return;
            }
            "assert" => {
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let v = self.stack.pop().unwrap();
                self.assert_log.push(v);
                if v != 1 {
                    self.error = true;
                }
            }
            "assert_vector" => {
                // Assert top 5 elements equal next 5
                if self.stack.len() < 10 {
                    self.error = true;
                    return;
                }
                let len = self.stack.len();
                let asserted: Vec<u64> = (0..5).map(|i| self.stack[len - 1 - i]).collect();
                self.assert_vector_log.push(asserted);
                for i in 0..5 {
                    if self.stack[len - 1 - i] != self.stack[len - 6 - i] {
                        self.error = true;
                        return;
                    }
                }
                // Pop top 5
                self.stack.truncate(len - 5);
            }

            // --- I/O (modeled stack effects, dummy values) ---
            "read_io" => {
                let n = arg_u.unwrap_or(1) as usize;
                for _ in 0..n {
                    self.stack.push(0);
                }
            }
            "write_io" => {
                let n = arg_u.unwrap_or(1) as usize;
                if self.stack.len() < n {
                    self.error = true;
                    return;
                }
                // Log values being written (TOS first = reverse of slice order)
                let start = self.stack.len() - n;
                for i in (start..self.stack.len()).rev() {
                    self.io_output.push(self.stack[i]);
                }
                self.stack.truncate(start);
            }
            "divine" => {
                let n = arg_u.unwrap_or(1) as usize;
                self.divine_log.push(n);
                for _ in 0..n {
                    self.stack.push(0);
                }
            }

            // --- Memory (modeled stack effects) ---
            "read_mem" => {
                // pop address, push N values + adjusted address
                let n = arg_u.unwrap_or(1) as usize;
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let _addr = self.stack.pop().unwrap();
                for _ in 0..n {
                    self.stack.push(0); // dummy values
                }
                self.stack.push(0); // adjusted address
            }
            "write_mem" => {
                // pop N values + address, push adjusted address
                let n = arg_u.unwrap_or(1) as usize;
                if self.stack.len() < n + 1 {
                    self.error = true;
                    return;
                }
                self.stack.truncate(self.stack.len() - n - 1);
                self.stack.push(0); // adjusted address
            }

            // --- Crypto (modeled stack effects only) ---
            "hash" => {
                // pop 10, push 5
                if self.stack.len() < 10 {
                    self.error = true;
                    return;
                }
                self.stack.truncate(self.stack.len() - 10);
                for _ in 0..5 {
                    self.stack.push(0);
                }
            }
            "sponge_init" => {}
            "sponge_absorb" => {
                if self.stack.len() < 10 {
                    self.error = true;
                    return;
                }
                self.stack.truncate(self.stack.len() - 10);
            }
            "sponge_squeeze" => {
                for _ in 0..10 {
                    self.stack.push(0);
                }
            }
            "sponge_absorb_mem" => {
                // Absorb from memory: pop address, push adjusted address
                if self.stack.is_empty() {
                    self.error = true;
                    return;
                }
                let _addr = self.stack.pop().unwrap();
                self.stack.push(0);
            }
            "merkle_step" | "merkle_step_mem" => {
                // Complex stack effects — skip in block verifier
            }

            // --- Extension field (modeled as nops for stack) ---
            "xb_mul" | "x_invert" | "xx_dot_step" | "xb_dot_step" => {}

            // --- Control flow ---
            // return/recurse are stack-transparent for isolated function verification.
            // call/skiz/recurse_or_return require branch simulation — unsimulable.
            "return" | "recurse" => {}
            "call" | "recurse_or_return" | "skiz" => {
                self.error = true;
            }

            // Unknown instruction — ignore (conservative)
            _ => {}
        }
    }
}
