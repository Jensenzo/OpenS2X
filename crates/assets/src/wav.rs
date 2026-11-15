//! WAV files as the game ships them (`data/sound`): most are 16-bit PCM,
//! some are IMA ADPCM (format 0x11), which this module turns into 16-bit
//! PCM so any player can use them. The standard formats; nothing game
//! specific.

/// IMA ADPCM step sizes.
const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

struct Channel {
    predictor: i32,
    index: i32,
}

impl Channel {
    fn decode(&mut self, nibble: u8) -> i16 {
        let step = STEPS[self.index as usize];
        let mut diff = step >> 3;
        if nibble & 1 != 0 {
            diff += step >> 2;
        }
        if nibble & 2 != 0 {
            diff += step >> 1;
        }
        if nibble & 4 != 0 {
            diff += step;
        }
        if nibble & 8 != 0 {
            self.predictor -= diff;
        } else {
            self.predictor += diff;
        }
        self.predictor = self.predictor.clamp(-32768, 32767);
        self.index = (self.index + INDEX[usize::from(nibble & 15)]).clamp(0, 88);
        self.predictor as i16
    }
}

/// The `fmt ` chunk and the data of a RIFF WAVE file.
struct Parts<'a> {
    format: u16,
    channels: u16,
    rate: u32,
    block_align: u16,
    data: &'a [u8],
}

fn parts(file: &[u8]) -> Option<Parts<'_>> {
    if file.len() < 12 || &file[0..4] != b"RIFF" || &file[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut fmt = None;
    let mut data = None;
    while at + 8 <= file.len() {
        let id = &file[at..at + 4];
        let len = u32::from_le_bytes(file[at + 4..at + 8].try_into().ok()?) as usize;
        let body = file.get(at + 8..(at + 8 + len).min(file.len()))?;
        match id {
            b"fmt " if body.len() >= 16 => fmt = Some(body),
            b"data" => data = Some(body),
            _ => {}
        }
        at += 8 + len + (len & 1);
    }
    let fmt = fmt?;
    let u16_at = |i: usize| u16::from_le_bytes([fmt[i], fmt[i + 1]]);
    Some(Parts {
        format: u16_at(0),
        channels: u16_at(2),
        rate: u32::from_le_bytes([fmt[4], fmt[5], fmt[6], fmt[7]]),
        block_align: u16_at(12),
        data: data?,
    })
}

/// A 16-bit PCM WAVE file of `samples` (interleaved).
pub fn pcm_wav(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend(b"RIFF");
    out.extend((36 + data_len).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(channels.to_le_bytes());
    out.extend(rate.to_le_bytes());
    out.extend((rate * u32::from(channels) * 2).to_le_bytes());
    out.extend((channels * 2).to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data_len.to_le_bytes());
    for s in samples {
        out.extend(s.to_le_bytes());
    }
    out
}

/// Decode IMA ADPCM blocks: per channel a 4-byte header (predictor, step
/// index), then the nibbles, low first, in 4-byte groups per channel.
fn decode_ima(data: &[u8], channels: usize, block_align: usize) -> Vec<i16> {
    let mut out = Vec::new();
    if channels == 0 || block_align < 4 * channels {
        return out;
    }
    for block in data.chunks(block_align) {
        if block.len() < 4 * channels {
            break;
        }
        let mut chans: Vec<Channel> = (0..channels)
            .map(|c| {
                let h = &block[4 * c..4 * c + 4];
                Channel {
                    predictor: i32::from(i16::from_le_bytes([h[0], h[1]])),
                    index: i32::from(h[2]).clamp(0, 88),
                }
            })
            .collect();
        for c in &chans {
            out.push(c.predictor as i16);
        }
        let body = &block[4 * channels..];
        // Groups of 4 bytes (8 samples) per channel, interleaved.
        for group in body.chunks(4 * channels) {
            if group.len() < 4 * channels {
                break;
            }
            let mut frame = vec![[0i16; 8]; channels];
            for (c, chan) in chans.iter_mut().enumerate() {
                for k in 0..4 {
                    let byte = group[4 * c + k];
                    frame[c][2 * k] = chan.decode(byte & 15);
                    frame[c][2 * k + 1] = chan.decode(byte >> 4);
                }
            }
            for i in 0..8 {
                for f in &frame {
                    out.push(f[i]);
                }
            }
        }
    }
    out
}

/// A WAVE file any player can decode: IMA ADPCM is turned into 16-bit
/// PCM, anything else (and anything not a WAVE file) is returned as it
/// is.
pub fn playable(file: &[u8]) -> Vec<u8> {
    match parts(file) {
        Some(p) if p.format == 0x11 => {
            let samples = decode_ima(p.data, usize::from(p.channels), usize::from(p.block_align));
            pcm_wav(&samples, p.channels, p.rate)
        }
        _ => file.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ima_file(block: &[u8], channels: u16) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend(b"RIFF");
        f.extend(0u32.to_le_bytes());
        f.extend(b"WAVEfmt ");
        f.extend(20u32.to_le_bytes());
        f.extend(0x11u16.to_le_bytes());
        f.extend(channels.to_le_bytes());
        f.extend(8000u32.to_le_bytes());
        f.extend(0u32.to_le_bytes());
        f.extend((block.len() as u16).to_le_bytes());
        f.extend(4u16.to_le_bytes());
        f.extend(2u16.to_le_bytes());
        f.extend(0u16.to_le_bytes());
        f.extend(b"data");
        f.extend((block.len() as u32).to_le_bytes());
        f.extend(block);
        f
    }

    /// A mono block: the header's predictor is the first sample, then
    /// eight samples per four bytes; nibble 0 adds step / 8, nibble 8
    /// subtracts it.
    // spec: FMT-3.10, SND-1.3
    #[test]
    fn decodes_ima_adpcm() {
        // Predictor 100, step index 0 (step 7), then nibbles 0, 8, ...
        let block = [100, 0, 0, 0, 0x80, 0x00, 0x00, 0x00];
        let wav = playable(&ima_file(&block, 1));
        let p = parts(&wav).unwrap();
        assert_eq!((p.format, p.channels, p.rate), (1, 1, 8000));
        let s: Vec<i16> = p
            .data
            .chunks(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect();
        assert_eq!(s.len(), 9);
        assert_eq!(&s[..3], &[100, 100, 100]);
        // A PCM file passes unchanged.
        assert_eq!(playable(&wav), wav);
    }
}
