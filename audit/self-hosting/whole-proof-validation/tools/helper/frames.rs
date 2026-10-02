//! Preserve compressed frame payloads while checking and rebuilding full chains.
use super::{bad, exact, DECODED, WIRE};
use joy_rs::structured::certificate::transport::Writer;
use miniz_oxide::inflate::{
    core::{decompress, inflate_flags, DecompressorOxide},
    TINFLStatus,
};
use std::io::{self, Read, Write};

pub const CHUNK: usize = 65536;
pub struct Frame {
    pub header: [u8; 50],
    pub payload: Vec<u8>,
    pub digest: [u8; 32],
}

fn digest(domain: &[u8], head: &[u8], body: &[u8]) -> [u8; 32] {
    let mut h = hemera::Hasher::new();
    h.update(domain).update(head).update(body);
    *h.finalize().as_bytes()
}

fn seed(context: &[u8; 32]) -> [u8; 32] {
    digest(
        b"joy/compiler-certificate/context/v1\0",
        b"JOYSC001",
        context,
    )
}

impl Frame {
    pub fn decoded_len(&self) -> usize {
        u32::from_le_bytes([
            self.header[10],
            self.header[11],
            self.header[12],
            self.header[13],
        ]) as usize
    }
    pub fn terminal(&self) -> bool {
        self.header[8] == 1
    }
    pub fn raw(&self, writer: &mut impl Write) -> io::Result<()> {
        writer.write_all(&self.header)?;
        writer.write_all(&self.payload)?;
        writer.write_all(&self.digest)
    }
    pub fn decoded(&self) -> io::Result<Vec<u8>> {
        if self.header[9] == 0 {
            return Ok(self.payload.clone());
        }
        let mut out = vec![0; self.decoded_len() + 1];
        let (status, consumed, produced) = decompress(
            &mut DecompressorOxide::new(),
            &self.payload,
            &mut out,
            0,
            inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF,
        );
        if status != TINFLStatus::Done
            || consumed != self.payload.len()
            || produced != self.decoded_len()
        {
            return Err(bad("strict frame decompression"));
        }
        out.truncate(produced);
        Ok(out)
    }
    pub fn from_decoded(bytes: &[u8]) -> io::Result<Self> {
        if bytes.is_empty() || bytes.len() > CHUNK {
            return Err(bad("encode one nonempty frame"));
        }
        let mut writer = Writer::new(Vec::new(), [0; 32], super::limits())?;
        writer.write_all(bytes)?;
        let (raw, _) = writer.finish()?;
        let header: [u8; 50] = raw[40..90].try_into().map_err(|_| bad("encoded header"))?;
        let size = u32::from_le_bytes(header[14..18].try_into().map_err(|_| bad("encoded size"))?)
            as usize;
        Ok(Self {
            header,
            payload: raw[90..90 + size].to_vec(),
            digest: [0; 32],
        })
    }
    pub fn end() -> Self {
        let mut header = [0; 50];
        header[8] = 1;
        Self {
            header,
            payload: Vec::new(),
            digest: [0; 32],
        }
    }
}

pub struct Input<R> {
    reader: R,
    pub context: [u8; 32],
    previous: [u8; 32],
    pub frames: u64,
    pub decoded: u64,
    pub wire: u64,
    done: bool,
}

impl<R: Read> Input<R> {
    pub fn new(mut reader: R) -> io::Result<Self> {
        if exact::<8>(&mut reader)? != *b"JOYSC001" {
            return Err(bad("magic"));
        }
        let context = exact(&mut reader)?;
        Ok(Self {
            reader,
            context,
            previous: seed(&context),
            frames: 0,
            decoded: 0,
            wire: 40,
            done: false,
        })
    }
    pub fn next(&mut self) -> io::Result<Option<Frame>> {
        if self.done {
            return Ok(None);
        }
        let header = exact::<50>(&mut self.reader)?;
        let sequence = u64::from_le_bytes(header[..8].try_into().map_err(|_| bad("sequence"))?);
        let decoded =
            u32::from_le_bytes(header[10..14].try_into().map_err(|_| bad("decoded"))?) as usize;
        let encoded =
            u32::from_le_bytes(header[14..18].try_into().map_err(|_| bad("encoded"))?) as usize;
        if sequence != self.frames || header[18..] != self.previous {
            return Err(bad("original frame order/chain"));
        }
        let terminal = match header[8] {
            0 if (1..=CHUNK).contains(&decoded) => false,
            1 if decoded == 0 && encoded == 0 && header[9] == 0 => true,
            _ => return Err(bad("original frame kind")),
        };
        match header[9] {
            0 if encoded == decoded => (),
            1 if encoded > 0 && encoded < decoded => (),
            _ => return Err(bad("original codec size")),
        }
        self.wire = self
            .wire
            .checked_add(82 + encoded as u64)
            .filter(|n| *n <= WIRE)
            .ok_or_else(|| bad("wire cap"))?;
        self.decoded = self
            .decoded
            .checked_add(decoded as u64)
            .filter(|n| *n <= DECODED)
            .ok_or_else(|| bad("decoded cap"))?;
        self.frames += 1;
        let mut payload = vec![0; encoded];
        self.reader.read_exact(&mut payload)?;
        let claimed = exact::<32>(&mut self.reader)?;
        if claimed != digest(b"joy/compiler-certificate/frame/v1\0", &header, &payload) {
            return Err(bad("original frame digest"));
        }
        self.previous = claimed;
        if terminal {
            if self.reader.read(&mut [0])? != 0 {
                return Err(bad("original trailing bytes"));
            }
            self.done = true;
        }
        Ok(Some(Frame {
            header,
            payload,
            digest: claimed,
        }))
    }
}

pub struct Output<W> {
    pub writer: W,
    previous: [u8; 32],
    pub frames: u64,
    pub decoded: u64,
    pub wire: u64,
}
impl<W: Write> Output<W> {
    pub fn new(mut writer: W, context: [u8; 32]) -> io::Result<Self> {
        writer.write_all(b"JOYSC001")?;
        writer.write_all(&context)?;
        Ok(Self {
            writer,
            previous: seed(&context),
            frames: 0,
            decoded: 0,
            wire: 40,
        })
    }
    pub fn emit(&mut self, frame: &Frame) -> io::Result<()> {
        self.wire = self
            .wire
            .checked_add(82 + frame.payload.len() as u64)
            .filter(|n| *n <= WIRE + CHUNK as u64)
            .ok_or_else(|| bad("mutation wire cap"))?;
        self.decoded = self
            .decoded
            .checked_add(frame.decoded_len() as u64)
            .filter(|n| *n <= DECODED)
            .ok_or_else(|| bad("mutation decoded cap"))?;
        let mut header = frame.header;
        header[..8].copy_from_slice(&self.frames.to_le_bytes());
        header[18..].copy_from_slice(&self.previous);
        self.previous = digest(
            b"joy/compiler-certificate/frame/v1\0",
            &header,
            &frame.payload,
        );
        self.writer.write_all(&header)?;
        self.writer.write_all(&frame.payload)?;
        self.writer.write_all(&self.previous)?;
        self.frames += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use joy_rs::structured::certificate::transport::Reader;

    fn fixture() -> (Vec<u8>, Vec<u8>) {
        let data: Vec<u8> = (0..CHUNK * 2 + 123).map(|i| (i % 251) as u8).collect();
        let mut writer = Writer::new(Vec::new(), [7; 32], crate::limits()).unwrap();
        writer.write_all(&data).unwrap();
        (writer.finish().unwrap().0, data)
    }

    #[test]
    fn rebuilding_untouched_compressed_frames_is_byte_identical() {
        let (source, data) = fixture();
        let mut input = Input::new(source.as_slice()).unwrap();
        let mut output = Output::new(Vec::new(), input.context).unwrap();
        let mut decoded = Vec::new();
        while let Some(frame) = input.next().unwrap() {
            decoded.extend(frame.decoded().unwrap());
            output.emit(&frame).unwrap();
        }
        assert_eq!(decoded, data);
        assert_eq!(output.writer, source);
        let mut accepted = Reader::new(output.writer.as_slice(), [7; 32], crate::limits()).unwrap();
        let mut read = Vec::new();
        accepted.read_to_end(&mut read).unwrap();
        accepted.finish().unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn rebound_complete_chain_passes_public_transport_under_new_context() {
        let (source, data) = fixture();
        let mut input = Input::new(source.as_slice()).unwrap();
        let mut output = Output::new(Vec::new(), [8; 32]).unwrap();
        while let Some(frame) = input.next().unwrap() {
            output.emit(&frame).unwrap();
        }
        let mut reader = Reader::new(output.writer.as_slice(), [8; 32], crate::limits()).unwrap();
        let mut actual = Vec::new();
        reader.read_to_end(&mut actual).unwrap();
        reader.finish().unwrap();
        assert_eq!(actual, data);
        assert_ne!(source, output.writer);
    }

    #[test]
    fn public_compressor_output_is_reencoded_as_one_checked_frame() {
        for size in [1, CHUNK - 1, CHUNK] {
            let data: Vec<u8> = (0..size).map(|i| (i % 233) as u8).collect();
            let frame = Frame::from_decoded(&data).unwrap();
            assert_eq!(frame.decoded().unwrap(), data);
            let mut output = Output::new(Vec::new(), [0; 32]).unwrap();
            output.emit(&frame).unwrap();
            output.emit(&Frame::end()).unwrap();
            let mut reader =
                Reader::new(output.writer.as_slice(), [0; 32], crate::limits()).unwrap();
            let mut result = Vec::new();
            reader.read_to_end(&mut result).unwrap();
            reader.finish().unwrap();
            assert_eq!(result, data);
        }
    }

    #[test]
    fn source_chain_corruption_is_refused() {
        let (mut source, _) = fixture();
        source[58] ^= 1;
        let mut input = Input::new(source.as_slice()).unwrap();
        assert!(input.next().is_err());
    }
}
