//! Independent streaming decoders, following VCW's Symphonia ALAC validation.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::Path,
};
use symphonia::core::{
    audio::RawSampleBuffer,
    codecs::{CODEC_TYPE_AAC, CODEC_TYPE_MP3, CODEC_TYPE_VORBIS, DecoderOptions},
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub(super) fn pcm_hash(path: &Path) -> Result<String> {
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }
    let stream = MediaSourceStream::new(Box::new(source(path)?), Default::default());
    let mut format = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )?
        .format;
    let track = format.default_track().context("Missing audio track")?;
    let id = track.id;
    // Lossy containers describe presentation duration, not padded decoder frames.
    let expected = if matches!(
        track.codec_params.codec,
        CODEC_TYPE_AAC | CODEC_TYPE_MP3 | CODEC_TYPE_VORBIS
    ) {
        None
    } else {
        track.codec_params.n_frames
    };
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions { verify: true })?;
    let mut hash = Sha256::new();
    let mut samples = None;
    let mut spec = None;
    let mut frames = 0u64;
    loop {
        crate::shutdown::check()?;
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(error)) if error.kind() == ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = decoder.decode(&packet)?;
        let current = *decoded.spec();
        ensure!(
            spec.is_none_or(|spec| spec == current),
            "Audio layout changed during decoding"
        );
        spec = Some(current);
        frames += decoded.frames() as u64;
        let buffer = samples
            .get_or_insert_with(|| RawSampleBuffer::<f64>::new(decoded.capacity() as u64, current));
        if buffer.capacity() < decoded.capacity() * current.channels.count() {
            *buffer = RawSampleBuffer::new(decoded.capacity() as u64, current);
        }
        buffer.copy_interleaved_ref(decoded);
        // RawSampleBuffer exposes interleaved native-endian bytes without a second copy.
        #[cfg(target_endian = "little")]
        hash.update(buffer.as_bytes());
        #[cfg(target_endian = "big")]
        for sample in buffer.as_bytes().chunks_exact(8) {
            let mut little: [u8; 8] = sample.try_into()?;
            little.reverse();
            hash.update(little);
        }
    }
    ensure!(frames > 0, "Audio contains no decoded frames");
    if let Some(expected) = expected {
        ensure!(
            frames == expected,
            "Incomplete audio: decoded {frames} of {expected} frames"
        );
    }
    ensure!(
        decoder.finalize().verify_ok != Some(false),
        "Audio checksum verification failed"
    );
    let spec = spec.context("Missing decoded layout")?;
    Ok(format!(
        "{},{}:SHA256={:x}",
        spec.rate,
        spec.channels.count(),
        hash.finalize()
    ))
}

// Symphonia 0.5 counts AIFF SSND's eight-byte offset/header as audio. Present a
// corrected chunk length to the decoder, never modifying the stored file.
struct Source {
    file: File,
    patch: Option<(u64, [u8; 4])>,
}
fn source(path: &Path) -> Result<Source> {
    let mut file = File::open(path)?;
    let mut header = [0; 12];
    let n = file.read(&mut header)?;
    let mut patch = None;
    if n == 12 && &header[..4] == b"FORM" && matches!(&header[8..], b"AIFF" | b"AIFC") {
        let len = file.metadata()?.len();
        let mut pos = 12u64;
        while pos + 8 <= len {
            file.seek(SeekFrom::Start(pos))?;
            let mut chunk = [0; 8];
            file.read_exact(&mut chunk)?;
            let size = u32::from_be_bytes(chunk[4..].try_into()?);
            ensure!(
                pos + 8 + u64::from(size) + u64::from(size % 2) <= len,
                "Truncated AIFF chunk"
            );
            if &chunk[..4] == b"SSND" {
                ensure!(size >= 8, "Invalid AIFF sound chunk");
                patch = Some((pos + 4, (size - 8).to_be_bytes()));
                break;
            }
            pos += 8 + u64::from(size) + u64::from(size % 2);
        }
        ensure!(patch.is_some(), "Missing AIFF sound chunk");
    }
    file.rewind()?;
    Ok(Source { file, patch })
}
impl Read for Source {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        if self.patch.is_none() {
            return self.file.read(bytes);
        }
        let start = self.file.stream_position()?;
        let n = self.file.read(bytes)?;
        if let Some((offset, patch)) = self.patch {
            for (index, byte) in patch.iter().enumerate() {
                let pos = offset + index as u64;
                if pos >= start && pos - start < n as u64 {
                    bytes[(pos - start) as usize] = *byte;
                }
            }
        }
        Ok(n)
    }
}
impl Seek for Source {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.file.seek(pos)
    }
}
impl symphonia::core::io::MediaSource for Source {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        self.file.metadata().ok().map(|m| m.len())
    }
}
