// Reference used for this module:
// http://www.libpng.org/pub/png/spec/1.2/PNG-Structure.html

use eyre::{ensure, Result};
use std::io::{self, Read, Write};

pub struct PngStreamSplitter<T>(T);

impl<R: Read> PngStreamSplitter<R> {
    pub fn new(reader: R) -> Self {
        Self(reader)
    }

    pub fn write_next<W: Write>(&mut self, mut writer: W) -> Result<()> {
        const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

        let reader = &mut self.0;
        let mut buf = [0u8; 8];

        // Read magic
        reader.read_exact(&mut buf)?;
        ensure!(buf == PNG_MAGIC, "invalid png magic");
        writer.write_all(&buf)?;

        loop {
            // Read chunk length and type
            reader.read_exact(&mut buf[..])?;
            let chunk_len = u32::from_be_bytes(buf[0..4].try_into().unwrap());
            let chunk_iend = &buf[4..8] == b"IEND";
            writer.write_all(&buf)?;

            // Copy chunk data (chunk_len bytes) and CRC footer (4 bytes)
            let bytes_to_copy = (chunk_len + 4) as u64;
            let copied = io::copy(&mut reader.by_ref().take(bytes_to_copy), &mut writer)?;
            ensure!(copied == bytes_to_copy, "invalid png chunk");

            if chunk_iend {
                break;
            }
        }

        writer.flush()?;

        Ok(())
    }
}
