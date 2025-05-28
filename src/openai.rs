use anyhow::{bail, Context, Result};
use embedded_svc::http::client::{Client, Request, Response};
use esp_idf_svc::{
    http::client::{Configuration as HttpCfg, EspHttpConnection},
    sys::esp_crt_bundle_attach,
    io::Write,
};
use core::str;
use std::vec::Vec;

const OPENAI_KEY: &str = env!("OPENAI_API_KEY");
const WHISPER_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const CHAT_URL:    &str = "https://api.openai.com/v1/chat/completions";

fn pcm_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    let riff_len = 36 + data_len;

    wav.extend(b"RIFF");
    wav.extend(&riff_len.to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(&16u32.to_le_bytes());
    wav.extend(&1u16.to_le_bytes());
    wav.extend(&1u16.to_le_bytes());
    wav.extend(&sample_rate.to_le_bytes());
    wav.extend(&(sample_rate * 2).to_le_bytes());
    wav.extend(&2u16.to_le_bytes());
    wav.extend(&16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(&data_len.to_le_bytes());
    for s in samples {
        wav.extend(&s.to_le_bytes());
    }
    wav
}

fn whisper_inner(wav: &[u8], language: &str) -> Result<String> {
    let bnd  = "ESP32BOUNDARY";
    let mut body = Vec::<u8>::new();
    let add = |buf: &mut Vec<u8>, s: &str| buf.extend_from_slice(s.as_bytes());

    add(&mut body, &format!("--{bnd}\r\n"));
    add(&mut body, "Content-Disposition: form-data; name=\"model\"\r\n\r\nwhisper-1\r\n");

    add(&mut body, &format!("--{bnd}\r\n"));
    add(&mut body, &format!("Content-Disposition: form-data; name=\"language\"\r\n\r\n{language}\r\n"));

    add(&mut body, &format!("--{bnd}\r\n"));
    add(
        &mut body,
        "Content-Disposition: form-data; name=\"file\"; filename=\"audio.wav\"\r\n",
    );
    add(&mut body, "Content-Type: audio/wav\r\n\r\n");
    body.extend_from_slice(wav);
    add(&mut body, "\r\n");
    add(&mut body, &format!("--{bnd}--\r\n"));

    let conn = EspHttpConnection::new(&HttpCfg {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(conn);

    let auth  = format!("Bearer {}", OPENAI_KEY);
    let ctype = format!("multipart/form-data; boundary={bnd}");
    let clen  = body.len().to_string();
    let headers = [
        ("Authorization",  auth.as_str()),
        ("Content-Type",   ctype.as_str()),
        ("Content-Length", clen.as_str()),
    ];

    let mut req: Request<_> = client.post(WHISPER_URL, &headers)?;
    req.write_all(&body)?;
    let mut resp: Response<_> = req.submit()?;
    if resp.status() != 200 {
        bail!("Whisper HTTP {}", resp.status());
    }

    let mut out = Vec::<u8>::new();
    let mut buf = [0u8; 512];
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 { break; }
        out.extend_from_slice(&buf[..n]);
    }
    Ok(core::str::from_utf8(&out)?.trim().into())
}

pub fn whisper_transcribe(pcm: &[i16], language: &str) -> Result<String> {
    let wav = pcm_to_wav(pcm, 16_000);
    whisper_inner(&wav, language)
}

pub fn whisper_wav(wav: &[u8], language: &str) -> Result<String> {
    whisper_inner(wav, language)
}

pub fn chat(prompt: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": "gpt-3.5-turbo",
        "messages": [{"role":"user","content":prompt}]
    })
    .to_string();

    let conn = EspHttpConnection::new(&HttpCfg {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_crt_bundle_attach),
        ..Default::default()
    })?;
    let mut client = Client::wrap(conn);

    let auth = format!("Bearer {}", OPENAI_KEY);
    let clen = body.len().to_string();
    let headers = [
        ("Authorization",  auth.as_str()),
        ("Content-Type",   "application/json"),
        ("Content-Length", clen.as_str()),
    ];

    let mut req = client.post(CHAT_URL, &headers)?;
    req.write_all(body.as_bytes())?;
    let mut resp = req.submit()?;
    if resp.status() != 200 {
        bail!("ChatGPT HTTP {}", resp.status());
    }

    let mut json = Vec::<u8>::new();
    let mut buf = [0u8; 512];
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 { break; }
        json.extend_from_slice(&buf[..n]);
    }
    let v: serde_json::Value = serde_json::from_slice(&json)?;
    let reply = v["choices"][0]["message"]["content"]
        .as_str()
        .context("bad json")?;
    Ok(reply.trim().into())
}
