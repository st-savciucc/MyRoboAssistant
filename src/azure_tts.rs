use anyhow::{bail, Result};
use esp_idf_hal::io::Write as _; 
use embedded_svc::http::{client::Client, Method};
use esp_idf_svc::{
    hal::i2s::{I2sDriver, I2sTx},            
    http::client::{Configuration as HttpCfg, EspHttpConnection},
    sys::{TickType_t, esp_crt_bundle_attach},
};

const KEY:    &str = "6LjHtBn01z3moL6F7CLSOo0l72XWlbQSSB8oD55uBtGfDV528injJQQJ99BEACYeBjFXJ3w3AAAYACOGabqX";
const REGION: &str = "eastus";

pub fn tts_and_play(i2s: &mut I2sDriver<'static, I2sTx>, text: &str) -> Result<()> {
    let ssml = format!(
        r#"<speak version="1.0" xml:lang="ro-RO">
               <voice name="ro-RO-AlinaNeural">{}</voice>
           </speak>"#,
        text
    );
    log::debug!("📤 Construiesc SSML pentru textul: {:?}", text);

    let url = format!(
        "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
        REGION
    );
    log::debug!("🔌 Deschid conexiune TLS către {url}");

    let mut cfg = HttpCfg::default();
    cfg.crt_bundle_attach = Some(esp_crt_bundle_attach);
    cfg.buffer_size = Some(2048);

    cfg.buffer_size_tx = Some(2048);

    let conn = EspHttpConnection::new(&cfg)?;
    let mut hc  = Client::wrap(conn);

    log::debug!("➡️  Trimit request POST + headere...");
    let mut req = hc.request(
        Method::Post,
        &url,
        &[
            ("Ocp-Apim-Subscription-Key", KEY),
            ("Content-Type", "application/ssml+xml"),
            ("X-Microsoft-OutputFormat", "raw-16khz-16bit-mono-pcm"),
        ],
    )?;

    log::debug!("✉️  Scriu corpul SSML ({} B)…", ssml.len());
    req.write_all(ssml.as_bytes())?;

    log::debug!("⏳ Aştept răspuns Azure…");
    let mut resp = req.submit()?;

    // 5️⃣  Verificare status
    if resp.status() != 200 {
        bail!("Azure TTS HTTP {}", resp.status());
    }

    log::debug!("✅ HTTP {} primit – streaming audio începe", resp.status());
    let mut buf = [0u8; 1024];
    let mut total = 0usize;    

    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 { break; }
        total += n;
        log::trace!("🎧 chunk {} B (total {} KB)", n, total / 1024);

        amplify_in_place(&mut buf[..n], VOLUME_GAIN);

        i2s.write(&buf[..n], TickType_t::MAX)?;  
    }

    log::debug!("🏁 Streaming terminat – {} KB redat", total / 1024);
    Ok(())
}

const VOLUME_GAIN: f32 = 2.0;

fn amplify_in_place(buf: &mut [u8], gain: f32) {
    if gain == 1.0 { return; }    
    for chunk in buf.chunks_exact_mut(2) {   
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]) as f32;
        let mut amp  = sample * gain;
        if amp >  32767.0 { amp =  32767.0; } 
        if amp < -32768.0 { amp = -32768.0; }
        let out = amp as i16;
        let bytes = out.to_le_bytes();
        chunk[0] = bytes[0];
        chunk[1] = bytes[1];
    }
}