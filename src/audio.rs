use anyhow::{anyhow, Result};
use log::{error, info};
use std::{
    sync::mpsc::{Receiver, Sender}
};

use crate::openai;

pub fn transcribe_and_chat(wav: &[u8]) -> Result<(String, String)> {
    let text = openai::whisper_wav(wav, "ro")?;
    info!("📜 Whisper: {}", text);
    
    let text_for_chat = text.clone();
    let reply = std::thread::spawn(move || openai::chat(&text_for_chat))
        .join()
        .map_err(|e| anyhow!("Eroare thread: {:?}", e))??;
    
    info!("🤖 ChatGPT: {}", reply);
    Ok((text, reply))
}


pub fn audio_task(rx: Receiver<Vec<u8>>, tx: Sender<(String,String)>) {
    while let Ok(wav) = rx.recv() {
        info!("audio_task: {} B", wav.len());

        match transcribe_and_chat(&wav) {
            Ok(pair)   => { let _ = tx.send(pair); }
            Err(error) => {
                error!("OpenAI: {error:?}");
                let _ = tx.send((
                    "Eroare".into(),
                    format!("Nu pot contacta OpenAI: {error}")
                ));
            }
        }
    }
}
