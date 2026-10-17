use super::*;
use miniz_oxide::deflate::core::{
    compress, create_comp_flags_from_zip_params, CompressorOxide, TDEFLFlush, TDEFLStatus,
};

pub struct Writer<W> {
    output: W,
    state: State,
    pending: Vec<u8>,
    compressed: Vec<u8>,
    compressor: CompressorOxide,
}

impl<W: Write> Writer<W> {
    pub fn new(mut output: W, context: [u8; 32], limits: Limits) -> io::Result<Self> {
        let state = State::new(limits, context)?;
        let mut pending = buffer(CHUNK)?;
        pending.clear();
        let compressed = buffer(CHUNK)?;
        // miniz's fixed internal state has an infallible constructor. Payload
        // buffers are fallible and reused; see the explicit transport contract.
        let compressor = CompressorOxide::new(create_comp_flags_from_zip_params(1, 0, 0));
        output.write_all(MAGIC)?;
        output.write_all(&context)?;
        Ok(Self {
            output,
            state,
            pending,
            compressed,
            compressor,
        })
    }

    pub fn stats(&self) -> Stats {
        self.state.stats
    }

    fn emit(&mut self, terminal: bool) -> io::Result<()> {
        self.state.usable()?;
        // Poison before any fallible work; only a fully written frame clears it.
        self.state.failed = true;
        self.state.next(0, 0)?;
        let compressed_len = self.compress()?;
        let (codec, payload) = if let Some(len) = compressed_len {
            (1, &self.compressed[..len])
        } else {
            (0, self.pending.as_slice())
        };
        let next = self.state.next(self.pending.len(), payload.len())?;
        let mut header = [0; HEADER];
        header[..8].copy_from_slice(&self.state.stats.frames.to_le_bytes());
        header[8] = u8::from(terminal);
        header[9] = codec;
        header[10..14].copy_from_slice(&(self.pending.len() as u32).to_le_bytes());
        header[14..18].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        header[18..].copy_from_slice(&self.state.previous);
        let digest = frame_digest(&header, payload);
        self.output.write_all(&header)?;
        self.output.write_all(payload)?;
        self.output.write_all(&digest)?;
        self.state.stats = next;
        self.state.previous = digest;
        self.state.failed = false;
        self.pending.clear();
        Ok(())
    }

    fn compress(&mut self) -> io::Result<Option<usize>> {
        let limit = self.pending.len().saturating_sub(1);
        if limit == 0 {
            return Ok(None);
        }
        self.compressor.reset();
        let (mut read, mut written) = (0, 0);
        loop {
            let (status, consumed, produced) = compress(
                &mut self.compressor,
                &self.pending[read..],
                &mut self.compressed[written..limit],
                TDEFLFlush::Finish,
            );
            if consumed > self.pending.len() - read || produced > limit - written {
                return Err(invalid("certificate compressor bounds"));
            }
            read += consumed;
            written += produced;
            match status {
                TDEFLStatus::Done if read == self.pending.len() => return Ok(Some(written)),
                TDEFLStatus::Okay if written == limit => return Ok(None),
                TDEFLStatus::Okay if consumed != 0 || produced != 0 => (),
                _ => return Err(invalid("certificate compression failed")),
            }
        }
    }

    pub fn finish(mut self) -> io::Result<(W, Stats)> {
        self.flush()?;
        self.emit(true)?;
        self.output.flush()?;
        Ok((self.output, self.state.stats))
    }
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.is_empty() {
            return Ok(0);
        }
        self.state.usable()?;
        let n = input.len().min(CHUNK - self.pending.len());
        let total = self
            .state
            .stats
            .decoded_bytes
            .checked_add((self.pending.len() + n) as u64);
        if total.is_none_or(|n| n > self.state.limits.decoded_bytes) {
            self.state.failed = true;
            return Err(invalid("certificate decoded allowance exhausted"));
        }
        self.pending.extend_from_slice(&input[..n]);
        if self.pending.len() == CHUNK {
            self.emit(false)?;
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.state.usable()?;
        if !self.pending.is_empty() {
            self.emit(false)?;
        }
        if let Err(error) = self.output.flush() {
            self.state.failed = true;
            return Err(error);
        }
        Ok(())
    }
}
