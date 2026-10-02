use super::*;
use miniz_oxide::inflate::{
    core::{decompress, inflate_flags, DecompressorOxide},
    TINFLStatus,
};

pub struct Reader<R> {
    input: R,
    state: State,
    decoded: Vec<u8>,
    position: usize,
    complete: bool,
}

impl<R: Read> Reader<R> {
    pub fn new(mut input: R, context: [u8; 32], limits: Limits) -> io::Result<Self> {
        let state = State::new(limits, context)?;
        let mut preamble = [0; 40];
        input.read_exact(&mut preamble)?;
        if &preamble[..8] != MAGIC || preamble[8..] != context {
            return Err(invalid("certificate format/context mismatch"));
        }
        Ok(Self {
            input,
            state,
            decoded: Vec::new(),
            position: 0,
            complete: false,
        })
    }

    pub fn stats(&self) -> Stats {
        self.state.stats
    }

    fn frame(&mut self) -> io::Result<()> {
        self.state.usable()?;
        self.state.failed = true;
        // The previous frame has been consumed. Release it before allocating
        // encoded and decoded storage for the next frame.
        self.decoded = Vec::new();
        // Check fixed overhead before touching the next untrusted header.
        self.state.next(0, 0)?;
        let mut header = [0; HEADER];
        self.input.read_exact(&mut header)?;
        let sequence = u64::from_le_bytes(header[..8].try_into().map_err(|_| invalid("sequence"))?);
        let decoded =
            u32::from_le_bytes(header[10..14].try_into().map_err(|_| invalid("length"))?) as usize;
        let encoded =
            u32::from_le_bytes(header[14..18].try_into().map_err(|_| invalid("length"))?) as usize;
        if sequence != self.state.stats.frames || header[18..] != self.state.previous {
            return Err(invalid("certificate frame order/chain mismatch"));
        }
        let terminal = match header[8] {
            0 if (1..=CHUNK).contains(&decoded) => false,
            1 if decoded == 0 && encoded == 0 && header[9] == 0 => true,
            _ => return Err(invalid("certificate frame kind/size")),
        };
        match header[9] {
            0 if encoded == decoded => (),
            1 if encoded > 0 && encoded < decoded => (),
            _ => return Err(invalid("certificate frame codec/size")),
        }
        let next = self.state.next(decoded, encoded)?;
        let mut payload = buffer(encoded)?;
        self.input.read_exact(&mut payload)?;
        let mut claimed = [0; 32];
        self.input.read_exact(&mut claimed)?;
        if claimed != frame_digest(&header, &payload) {
            return Err(invalid("certificate frame digest mismatch"));
        }
        self.decoded = if header[9] == 0 {
            payload
        } else {
            let mut output = buffer(decoded + 1)?;
            let (status, consumed, produced) = decompress(
                &mut DecompressorOxide::new(),
                &payload,
                &mut output,
                0,
                inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
            );
            if status != TINFLStatus::Done || consumed != encoded || produced != decoded {
                return Err(invalid("certificate deflate length/end mismatch"));
            }
            output.truncate(decoded);
            output
        };
        self.position = 0;
        if terminal {
            let mut extra = [0; 1];
            loop {
                match self.input.read(&mut extra) {
                    Ok(0) => break,
                    Ok(_) => return Err(invalid("certificate trailing bytes")),
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => (),
                    Err(e) => return Err(e),
                }
            }
        }
        self.state.stats = next;
        self.state.previous = claimed;
        self.complete = terminal;
        self.state.failed = false;
        Ok(())
    }

    /// Require immediate transport completion after the semantic parser ends.
    pub fn finish(mut self) -> io::Result<Stats> {
        let mut extra = [0; 1];
        if self.read(&mut extra)? != 0 {
            return Err(invalid("unconsumed certificate payload"));
        }
        Ok(self.state.stats)
    }
}

impl<R: Read> Read for Reader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        self.state.usable()?;
        if self.position == self.decoded.len() && !self.complete {
            self.frame()?;
        }
        let n = output.len().min(self.decoded.len() - self.position);
        output[..n].copy_from_slice(&self.decoded[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}
