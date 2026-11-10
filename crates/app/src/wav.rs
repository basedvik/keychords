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

    fn parse_header(buf: &[u8]) -> (u32, u16, u16, u32, u16, u32) {
        assert_eq!(&buf[0..4], b"RIFF");
        let riff_size = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(&buf[8..12], b"WAVE");
        assert_eq!(&buf[12..16], b"fmt ");
        let _fmt_size = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        let fmt = u16::from_le_bytes([buf[20], buf[21]]);
        let channels = u16::from_le_bytes([buf[22], buf[23]]);
        let sr = u32::from_le_bytes([buf[24], buf[25], buf[26], buf[27]]);
        let _byte_rate = u32::from_le_bytes([buf[28], buf[29], buf[30], buf[31]]);
        let _block_align = u16::from_le_bytes([buf[32], buf[33]]);
        let bits = u16::from_le_bytes([buf[34], buf[35]]);
        let data_off = 36;
        let data_tag = &buf[data_off..data_off + 4];
        assert_eq!(data_tag, b"data");
        let data_size = u32::from_le_bytes([
            buf[data_off + 4], buf[data_off + 5], buf[data_off + 6], buf[data_off + 7],
        ]);
        (riff_size, fmt, channels, sr, bits, data_size)
    }

    #[test]
    fn wav_header_fields_are_correct() {
        let buf = wav_24bit(&[0.0], &[0.0], 48000);
        let (riff_size, fmt, channels, sr, bits, data_size) = parse_header(&buf);
        assert_eq!(riff_size, 42); // 4 (WAVE) + 24 (fmt) + 8 + 6 (data) = 42
        assert_eq!(fmt, 1);
        assert_eq!(channels, 2);
        assert_eq!(sr, 48000);
        assert_eq!(bits, 24);
        assert_eq!(data_size, 6); // 2 ch * 3 bytes * 1 sample
    }

    #[test]
    fn wav_data_size_matches_samples() {
        let n = 1000;
        let buf = wav_24bit(&vec![0.0; n], &vec![0.0; n], 44100);
        let (_, _, _, _, _, data_size) = parse_header(&buf);
        assert_eq!(data_size, (n * 6) as u32);
    }

    #[test]
    fn wav_24bit_samples_are_little_endian_3_bytes() {
        let left = [1.0 / 8_388_607.0, 0.0, -1.0 / 8_388_607.0];
        let right = [0.0; 3];
        let buf = wav_24bit(&left, &right, 48000);

        // Parse header to find data offset
        let data_off = 40;
        let data_size = u32::from_le_bytes([
            buf[data_off + 4], buf[data_off + 5], buf[data_off + 6], buf[data_off + 7],
        ]) as usize;
        let data = &buf[data_off + 8..data_off + 8 + data_size];

        // Sample 0: left = 1.0/SCALE = 1, right = 0
        // Left: 01 00 00 (LE 24-bit for value 1)
        assert_eq!(&data[0..3], &[0x01, 0x00, 0x00], "left sample 0");
        assert_eq!(&data[3..6], &[0x00, 0x00, 0x00], "right sample 0");

        // Sample 1: both 0
        assert_eq!(&data[6..9], &[0x00, 0x00, 0x00], "left sample 1");
        assert_eq!(&data[9..12], &[0x00, 0x00, 0x00], "right sample 1");

        // Sample 2: left = -1/SCALE = -1
        // -1 in 24-bit two's complement LE: FF FF FF
        assert_eq!(&data[12..15], &[0xFF, 0xFF, 0xFF], "left sample 2");
        assert_eq!(&data[15..18], &[0x00, 0x00, 0x00], "right sample 2");
    }

    #[test]
    fn wav_clamps_values_above_1() {
        let left = [2.0, -3.5];
        let right = [0.0; 2];
        let buf = wav_24bit(&left, &right, 48000);

        let data_off = 40;
        let data = &buf[data_off + 8..];

        // 2.0 clamped to 1.0 => SCALE = 8388607
        assert_eq!(&data[0..3], &[0x7F, 0xFF, 0x7F], "clamped +2.0");
        assert_eq!(&data[3..6], &[0x00, 0x00, 0x00], "right 0");

        // -3.5 clamped to -1.0 => -8388607 = 0x800401 in 24-bit two's complement
        // In 24-bit two's complement: 0x800401
        // LE: 01 04 80
        assert_eq!(&data[6..9], &[0x01, 0x04, 0x80], "clamped -3.5");
        assert_eq!(&data[9..12], &[0x00, 0x00, 0x00], "right 0");
    }

    #[test]
    fn wav_zero_input_produces_silent_data() {
        let n = 480;
        let buf = wav_24bit(&vec![0.0; n], &vec![0.0; n], 48000);
        let data_off = 40;
        let data_size = u32::from_le_bytes([
            buf[data_off + 4], buf[data_off + 5], buf[data_off + 6], buf[data_off + 7],
        ]) as usize;
        for b in &buf[data_off + 8..data_off + 8 + data_size] {
            assert_eq!(*b, 0, "all data bytes should be zero");
        }
    }

    #[test]
    fn wav_file_length_is_correct() {
        let n = 4800;
        let buf = wav_24bit(&vec![0.0; n], &vec![0.0; n], 48000);
        let expected = 44 + (n as u32 * 6) as usize;
        assert_eq!(buf.len(), expected);
    }
}
