use anyhow::{anyhow, Result};
use embedded_svc::{
    http::{Method},
    io::{Read as IoRead, Write as IoWrite},
};

use std::sync::{Arc, Mutex};  
use esp_idf_svc::http::server::{EspHttpServer, Request, Connection};
use include_dir::{include_dir, Dir};
use std::sync::mpsc::{Receiver, Sender};

use esp_idf_svc::hal::i2s::{I2sDriver, I2sTx};



static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

fn mime_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html",
        "js"   => "text/javascript",
        "wav"  => "audio/wav",
        _      => "application/octet-stream",
    }
}

pub fn register_handlers(
    srv: &mut EspHttpServer,
    tx_audio: Sender<Vec<u8>>,
    rx_audio: Arc<Mutex<Receiver<(String, String)>>>,
    i2s_ref: Arc<Mutex<I2sDriver<'static, I2sTx>>>, 
    tx_tts: Sender<String>,
) -> anyhow::Result<()>{
    /* -------- GET / (şi alte fişiere statice) ----------------------- */
    srv.fn_handler("/", Method::Get, |req| -> Result<()> {
        send_static(req, "index.html")
    })?;

    srv.fn_handler("/index.html", Method::Get, |req| -> Result<()> {
        send_static(req, "index.html")
    })?;

    srv.fn_handler("/wav-encoder.js", Method::Get, |req| -> Result<()> {
        send_static(req, "wav-encoder.js")
    })?;

    srv.fn_handler("/transcribe", Method::Options, |req| -> Result<()> {
        let headers = &[
            ("Access-Control-Allow-Origin",  "*"),
            ("Access-Control-Allow-Methods", "POST, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type"),
        ];
        let mut resp = req.into_response(204, None::<&str>, headers)?;
        resp.flush()?;
        Ok(())
    })?;

srv.fn_handler("/hello", Method::Get, |req| -> Result<()> {
    let headers = &[("Access-Control-Allow-Origin", "*")];
    let mut resp = req.into_response(200, None::<&str>, headers)?;
    IoWrite::write_all(&mut resp, b"Hello received!")?;
    log::info!("Ping /hello – OK");
    Ok(())
})?;
srv.fn_handler("/send_text", Method::Post, {
    let tx_tts = tx_tts.clone();
    move |mut req| -> anyhow::Result<()> {
        const HDRS: &[(&str, &str)] = &[
            ("Access-Control-Allow-Origin",  "*"),
            ("Access-Control-Allow-Methods", "POST, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type"),
        ];

        let mut buf = [0u8; 512];
        let n = embedded_svc::io::Read::read(&mut req, &mut buf)?;
        let txt = core::str::from_utf8(&buf[..n]).unwrap_or("").trim().to_owned();

        log::info!("📝 Text primit de la browser: \"{txt}\"");

        let _ = tx_tts.send(txt);
        let mut resp = req.into_response(202, None::<&str>, HDRS)?;
        embedded_svc::io::Write::write_all(&mut resp, b"ACCEPTED")?;
        Ok(())
    }
})?;




const MAX_WAV: usize = 1024 * 1024;  

#[link_section = ".external_ram.bss"]     
static mut WAV_BUF: [u8; MAX_WAV] = [0; MAX_WAV]; 

srv.fn_handler(
    "/transcribe",
    Method::Post,
    move |mut req| -> Result<()> {
        let len = req
            .header("Content-Length")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);

        if !(1..=MAX_WAV).contains(&len) {
            let status = if len == 0 { 411 } else { 413 };
            let mut resp = req.into_status_response(status)?;  
            resp.flush()?;
            return Ok(());
        }

        let wav = unsafe { &mut WAV_BUF[..len] };
        let mut off = 0;
        while off < len {
            off += IoRead::read(&mut req, &mut wav[off..])?;
        }

        tx_audio.send(wav[..len].into())?;  

        let (text, reply) = {
            let guard = rx_audio.lock().unwrap();
            guard.recv()?
        };


        let mut resp = req.into_response(200, None::<&str>, &[
            ("Content-Type","application/json"),
            ("Access-Control-Allow-Origin","*")
        ])?;
        write!(resp, r#"{{"transcript":"{}","reply":"{}"}}"#, text, reply)?;
        Ok(())
    }
)?;


    Ok(())
}

use esp_idf_hal::io::{ErrorType}; 

fn send_static<C>(req: Request<C>, path: &str) -> Result<()>
where
    C: Connection + IoWrite + ErrorType,
    <C as ErrorType>::Error: std::error::Error + Send + Sync + 'static,
{
    let file = STATIC_DIR
        .get_file(path)
        .ok_or_else(|| anyhow!("file not found"))?;

    let mime = mime_for(path);
    let headers = &[
        ("Content-Type", mime),
        ("Access-Control-Allow-Origin", "*"),
    ];

    let mut resp = req.into_response(200, None::<&str>, headers)?;
    IoWrite::write_all(&mut resp, file.contents())?;
    Ok(())
}
