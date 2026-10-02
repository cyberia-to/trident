//! Ordered, context-bound frames with bounded strict decompression.
use std::io::{self, Read, Write};

mod reader;
mod writer;
pub use reader::Reader;
pub use writer::Writer;

const MAGIC: &[u8; 8] = b"JOYSC001";
const CHUNK: usize = 65_536;
const HEADER: usize = 50;
const OVERHEAD: u64 = (HEADER + 32) as u64;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub wire_bytes: u64,
    pub decoded_bytes: u64,
    pub frames: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct Stats {
    pub wire_bytes: u64,
    pub decoded_bytes: u64,
    pub frames: u64,
}

struct State {
    limits: Limits,
    stats: Stats,
    previous: [u8; 32],
    failed: bool,
}

impl State {
    fn new(limits: Limits, context: [u8; 32]) -> io::Result<Self> {
        if limits.wire_bytes < 40 + OVERHEAD || limits.decoded_bytes == 0 || limits.frames == 0 {
            return Err(invalid("certificate transport limits"));
        }
        Ok(Self {
            limits,
            stats: Stats {
                wire_bytes: 40,
                ..Stats::default()
            },
            previous: digest(b"joy/compiler-certificate/context/v1\0", MAGIC, &context),
            failed: false,
        })
    }

    fn usable(&self) -> io::Result<()> {
        if self.failed {
            Err(invalid("certificate transport previously failed"))
        } else {
            Ok(())
        }
    }

    fn next(&self, decoded: usize, encoded: usize) -> io::Result<Stats> {
        let next = Stats {
            wire_bytes: self
                .stats
                .wire_bytes
                .checked_add(OVERHEAD + encoded as u64)
                .ok_or_else(|| invalid("certificate wire count overflow"))?,
            decoded_bytes: self
                .stats
                .decoded_bytes
                .checked_add(decoded as u64)
                .ok_or_else(|| invalid("certificate decoded count overflow"))?,
            frames: self
                .stats
                .frames
                .checked_add(1)
                .ok_or_else(|| invalid("certificate frame count overflow"))?,
        };
        if next.wire_bytes > self.limits.wire_bytes
            || next.decoded_bytes > self.limits.decoded_bytes
            || next.frames > self.limits.frames
        {
            return Err(invalid("certificate transport allowance exhausted"));
        }
        Ok(next)
    }
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn buffer(size: usize) -> io::Result<Vec<u8>> {
    let mut value = Vec::new();
    value
        .try_reserve_exact(size)
        .map_err(|_| io::Error::from(io::ErrorKind::OutOfMemory))?;
    value.resize(size, 0);
    Ok(value)
}

fn digest(domain: &[u8], head: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut h = hemera::Hasher::new();
    h.update(domain).update(head).update(payload);
    *h.finalize().as_bytes()
}

fn frame_digest(head: &[u8; HEADER], payload: &[u8]) -> [u8; 32] {
    digest(b"joy/compiler-certificate/frame/v1\0", head, payload)
}

#[cfg(test)]
mod tests;
