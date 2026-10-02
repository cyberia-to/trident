use super::transport;

#[derive(Clone, Copy, Debug)]
pub struct CertificateLimits {
    pub nouns: u32,
    pub cache_slots: u32,
    pub records: u64,
    pub steps: u64,
    pub wire_bytes: u64,
    pub decoded_bytes: u64,
}
impl Default for CertificateLimits {
    fn default() -> Self {
        Self {
            nouns: 196_608,
            cache_slots: 65_536,
            records: 4_000_000,
            steps: 2_000_000,
            wire_bytes: 64 << 20,
            decoded_bytes: 256 << 20,
        }
    }
}
impl CertificateLimits {
    pub fn validate(self) -> Result<(), String> {
        for (name, value, max) in [
            ("nouns", u64::from(self.nouns), 3_145_728),
            ("cache_slots", u64::from(self.cache_slots), 262_144),
            ("records", self.records, 100_000_000_000),
            ("steps", self.steps, 40_000_000_000),
            ("wire_bytes", self.wire_bytes, 128 << 30),
            ("decoded_bytes", self.decoded_bytes, 1 << 40),
        ] {
            if value == 0 || value > max {
                return Err(format!("certificate limit {name} must be in 1..={max}"));
            }
        }
        Ok(())
    }
    pub(super) fn transport(self) -> transport::Limits {
        transport::Limits {
            wire_bytes: self.wire_bytes,
            decoded_bytes: self.decoded_bytes,
            frames: self.decoded_bytes.saturating_add(1),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Context {
    pub program: [u64; 4],
    pub formula: [u64; 4],
    pub object: [u64; 4],
    pub profile: u8,
    pub budget: u64,
    pub frames: u32,
}
impl Context {
    pub(super) fn digest(self) -> [u8; 32] {
        let mut h = hemera::Hasher::new();
        h.update(b"joy/nox/disclosed-compiler/v1\0");
        for limb in self
            .program
            .into_iter()
            .chain(self.formula)
            .chain(self.object)
        {
            h.update(&limb.to_le_bytes());
        }
        h.update(&[self.profile])
            .update(&self.budget.to_le_bytes())
            .update(&self.frames.to_le_bytes());
        *h.finalize().as_bytes()
    }
}
