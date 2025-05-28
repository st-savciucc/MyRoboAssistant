//! Funcţii ajutătoare (deocamdată doar PCM → WAV)

pub fn pcm_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");           // sub-chunk id
    wav.extend_from_slice(&16u32.to_le_bytes());  // fmt size
    wav.extend_from_slice(&1u16.to_le_bytes());   // PCM
    wav.extend_from_slice(&1u16.to_le_bytes());   // mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte-rate
    wav.extend_from_slice(&2u16.to_le_bytes());   // block align
    wav.extend_from_slice(&16u16.to_le_bytes());  // bits/sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in samples { wav.extend_from_slice(&s.to_le_bytes()); }
    wav
}
