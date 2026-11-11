//! 24-bit PCM WAV file generation.

#![allow(dead_code)]

const PCM: u16 = 1;
const BITS_24: u16 = 24;
const SCALE: i32 = 8_388_607;

/// Produce a RIFF/WAVE file: PCM (format 1), 2 channels, 24-bit, little-endian.
///
/// Samples are clamped to -1..=1 and scaled by 8_388_607. `left` and `right` must be
/// the same length.
pub fn wav_24bit(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8> {
    let n = left.len();
    assert_eq!(left.len(), right.len(), "left and right must be same length");
    let sample_data_size = n as u32 * 6; // 2 channels * 3 bytes per sample
    let file_size = 4 + // "WAVE"
        24 + // fmt chunk: 8 (header) + 16 (fmt data)
        8 + // data chunk header
        sample_data_size;

    let mut buf = Vec::with_capacity(file_size as usize);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&file_size.to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // fmt size
    buf.extend_from_slice(&PCM.to_le_bytes());    // format
    buf.extend_from_slice(&2u16.to_le_bytes());   // channels
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * 6u32;
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&6u16.to_le_bytes()); // block align
    buf.extend_from_slice(&BITS_24.to_le_bytes());

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&sample_data_size.to_le_bytes());

    for i in 0..n {
        for channel in [&left[i], &right[i]] {
            let s = channel.clamp(-1.0, 1.0) * SCALE as f32;
            let v = s.round() as i32;
            buf.push((v & 0xFF) as u8);
            buf.push(((v >> 8) & 0xFF) as u8);
            buf.push(((v >> 16) & 0xFF) as u8);
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_at(b: &[u8], i: usize) -> u16 {
        u16::from_le_bytes([b[i], b[i + 1]])
    }
    fn u32_at(b: &[u8], i: usize) -> u32 {
        u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
    }

    /// The three bytes of sample `k`, channel `c` (0 = left, 1 = right): the header is
    /// 44 bytes, each frame is 6 bytes, left first.
    fn sample_bytes(buf: &[u8], k: usize, c: usize) -> [u8; 3] {
        let i = 44 + 6 * k + 3 * c;
        [buf[i], buf[i + 1], buf[i + 2]]
    }

    #[test]
    fn header_is_44_bytes_with_spec_fields() {
        let n = 5;
        let buf = wav_24bit(&vec![0.0; n], &vec![0.0; n], 48_000);
        assert_eq!(&buf[0..4], b"RIFF");
        assert_eq!(u32_at(&buf, 4), 36 + 6 * n as u32, "RIFF size");
        assert_eq!(&buf[8..12], b"WAVE");
        assert_eq!(&buf[12..16], b"fmt ");
        assert_eq!(u32_at(&buf, 16), 16, "fmt size");
        assert_eq!(u16_at(&buf, 20), 1, "PCM");
        assert_eq!(u16_at(&buf, 22), 2, "channels");
        assert_eq!(u32_at(&buf, 24), 48_000, "sample rate");
        assert_eq!(u32_at(&buf, 28), 48_000 * 6, "byte rate");
        assert_eq!(u16_at(&buf, 32), 6, "block align");
        assert_eq!(u16_at(&buf, 34), 24, "bits per sample");
        assert_eq!(&buf[36..40], b"data");
        assert_eq!(u32_at(&buf, 40), 6 * n as u32, "data size");
        assert_eq!(buf.len(), 44 + 6 * n);
    }

    /// Expected values, worked out by hand: the three bytes are the little-endian low 24
    /// bits of `round(clamp(x) * 8_388_607)`.
    /// +1.0  ->  8388607 = 0x007FFFFF -> FF FF 7F
    /// +2.0  ->  clamps to +1.0       -> FF FF 7F
    ///  0.5  ->  round(4194303.5) = 4194304 = 0x00400000 -> 00 00 40
    ///  0.0  ->  0 -> 00 00 00
    /// -1.0  -> -8388607 = 0xFF800001 (two's complement) -> 01 00 80
    /// -3.5  ->  clamps to -1.0       -> 01 00 80
    #[test]
    fn samples_are_little_endian_24bit_and_clamped() {
        let left = [1.0f32, 2.0, 0.5, 0.0, -1.0, -3.5];
        let right = [0.0f32; 6];
        let buf = wav_24bit(&left, &right, 44_100);
        assert_eq!(sample_bytes(&buf, 0, 0), [0xFF, 0xFF, 0x7F], "+1.0");
        assert_eq!(sample_bytes(&buf, 1, 0), [0xFF, 0xFF, 0x7F], "+2.0 clamped");
        assert_eq!(sample_bytes(&buf, 2, 0), [0x00, 0x00, 0x40], "0.5");
        assert_eq!(sample_bytes(&buf, 3, 0), [0x00, 0x00, 0x00], "0.0");
        assert_eq!(sample_bytes(&buf, 4, 0), [0x01, 0x00, 0x80], "-1.0");
        assert_eq!(sample_bytes(&buf, 5, 0), [0x01, 0x00, 0x80], "-3.5 clamped");
        for k in 0..6 {
            assert_eq!(sample_bytes(&buf, k, 1), [0x00, 0x00, 0x00], "right {k}");
        }
    }

    #[test]
    fn file_length_is_header_plus_six_bytes_per_frame() {
        let n = 1000;
        let buf = wav_24bit(&vec![0.0; n], &vec![0.0; n], 44_100);
        assert_eq!(buf.len(), 44 + 6 * n);
        assert!(buf[44..].iter().all(|&b| b == 0), "silence encodes as zero bytes");
    }
}
