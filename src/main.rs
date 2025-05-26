/*
    Continuăm după confirmare
    Când acest minimal compilează şi rulează, ne mutăm la:

    Etapa 2 – iniţializare I2S-ADC 8 kHz şi afişarea energiei pe consolă (fără VAD, fără HTTP).
    Etapa 3 – VAD simplu (START/STOP) şi stocarea clipului PCM.
    Etapa 4 – cerere HTTPS la Whisper (folosind doar EspHttpConnection).
    Etapa 5 – cerere HTTPS la GPT şi afişare răspuns.
    Astfel prindem rapid eventualele schimbări de API pe fiecare bucată.
*/

// ===================== main.rs =====================
// VAD static thresholds + stocare clip PCM (heap) + task dedicat cu 8 kB stack
//  • Wi‑Fi inițial
//  • ADC one‑shot 8 kHz, 80 eșantioane / cadru (≈10 ms)
//  • VAD static: START când energia ≥ STATIC_START, STOP când energia ≤ STATIC_STOP_MAX pentru ≥ STOP_SILENCE_MS
//  • Clip max 1 s (8 000 eşantioane) într‑un Vec<i16> alocat în heap
//  • Rulează în propriul FreeRTOS task (stack 8 kB)

use anyhow::Result;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        adc::{
            attenuation::DB_11,
            oneshot::{config::AdcChannelConfig, AdcChannelDriver, AdcDriver},
        },
        peripherals::Peripherals,
    },
    nvs::EspDefaultNvsPartition,
    sys::link_patches,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
    log::EspLogger,
};
use heapless::{Deque, String};
use log::info;
use std::{thread, time::Duration};
mod openai;

// ==== parametri ==== 
const SAMPLE_RATE_HZ: u32      = 8_000;
const FRAME_SAMPLES: usize     = 80;
const START_FRAMES: usize      = 5;
const STOP_SILENCE_MS: u32     = 400;
const PRE_BUFFER_MS: u32       = 100;
const MAX_CLIP_MS: u32         = 1_000;
const MAX_CLIP_SAMP: usize     = (MAX_CLIP_MS as usize * SAMPLE_RATE_HZ as usize) / 1000;
const STATIC_START_ENERGY: u32 = 950;
const STATIC_STOP_MAX: u32     = 900;

fn app() -> Result<()> {
    // Wi‑Fi init
    let per   = Peripherals::take()?;
    let sys   = EspSystemEventLoop::take()?;
    let nvs   = EspDefaultNvsPartition::take()?;
    let modem = per.modem;

    let mut ssid: String<32> = String::new(); ssid.push_str("Constantin").unwrap();
    let mut pass: String<64> = String::new(); pass.push_str("11111112").unwrap();
    let wifi_drv = EspWifi::new(modem, sys.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(wifi_drv, sys)?;
    wifi.set_configuration(&Configuration::Client(
        ClientConfiguration { ssid, password: pass, ..Default::default() },
    ))?;
    wifi.start()?; 
    match wifi.connect() {
        Ok(_) => log::info!("Wi-Fi conectat!"),
        Err(e) => {
            log::error!("Nu m-am conectat: {:?}", e);
            // nu dăm panic! – rămânem într-o buclă
            loop { std::thread::sleep(Duration::from_secs(1)); }
        }
    }
    
    wifi.wait_netif_up()?;
    info!("IP: {:?}", wifi.wifi().sta_netif().get_ip_info()?);

    // ADC setup
    let adc = AdcDriver::new(per.adc1)?;
    let cfg = AdcChannelConfig { attenuation: DB_11, ..Default::default() };
    let mut mic = AdcChannelDriver::new(&adc, per.pins.gpio34, &cfg)?;

    let frame_delay = Duration::from_millis(10);

    // VAD state
    let mut voice_active = false;
    let mut above_cnt    = 0usize;
    let mut silence_ms   = 0u32;

    const PRE_CAP: usize = (PRE_BUFFER_MS as usize / 10) * FRAME_SAMPLES;
    let mut pre_buf: Deque<i16, PRE_CAP> = Deque::new();
    let mut clip: Vec<i16> = Vec::with_capacity(MAX_CLIP_SAMP);

        loop {
        // ===== 1. Citește un cadru de 10 ms (80 eşantioane) =====
        let mut sum_u32 = 0u32;
        let mut frame = [0i16; FRAME_SAMPLES];

        for s in frame.iter_mut() {
            let val = adc.read(&mut mic)? as u32;
            *s       = val as i16;
            sum_u32 += val;
        }
        let energy = sum_u32 / FRAME_SAMPLES as u32;

        // ===== 2. Logica VAD START / STOP =====
        if !voice_active {
            // ---- START ----
            if energy >= STATIC_START_ENERGY {
                above_cnt += 1;
                if above_cnt >= START_FRAMES {
                    voice_active = true;
                    info!("START voce – energy={} ({} frames)", energy, START_FRAMES);
                    // mută pre-bufferul în clip
                    while let Some(s) = pre_buf.pop_front() {
                        clip.push(s);
                    }
                }
            } else {
                above_cnt = 0;
                // alimentează pre-bufferul circular
                for &s in frame.iter() {
                    if pre_buf.push_back(s).is_err() {
                        pre_buf.pop_front();           // scapă cel mai vechi
                        pre_buf.push_back(s).ok();
                    }
                }
            }
        } else {
            // ---- Înregistrare activă ----
            for &s in frame.iter() {
                if clip.len() < MAX_CLIP_SAMP {
                    clip.push(s);
                }
            }

            // ---- STOP ----
            if energy <= STATIC_STOP_MAX {
                silence_ms += 10;
                if silence_ms >= STOP_SILENCE_MS {
                    voice_active = false;
                    let dur = clip.len() as u32 * 1000 / SAMPLE_RATE_HZ;
                    info!("STOP voce – {} ms ({} samples)", dur, clip.len());

                    // ===== 3. Trimite clipul la Whisper & ChatGPT =====
                    if let Ok(text) = openai::whisper_transcribe(&clip) {
                        info!("Transcript: \"{text}\"");
                        if let Ok(reply) = openai::chat(&text) {
                            info!("ChatGPT > {reply}");
                        }
                    }

                    // curăță starea
                    clip.clear();
                    silence_ms = 0;
                    above_cnt  = 0;
                }
            } else {
                silence_ms = 0;
            }
        }

        // ===== 4. Rată fixă 8 kHz =====
        thread::sleep(frame_delay);
    }

}

fn main() {
    link_patches(); EspLogger::initialize_default();
    std::thread::Builder::new()
        .name("voice-task".into())
        .stack_size(8*1024)
        .spawn(|| if let Err(e)=app(){ log::error!("app err: {:?}", e); })
        .unwrap();
    loop { thread::sleep(Duration::from_secs(60)); }
}
