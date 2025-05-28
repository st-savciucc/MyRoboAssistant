// ===================== main.rs =====================
use anyhow::Result;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::EspHttpServer,
    log::EspLogger,
    nvs::EspDefaultNvsPartition,
    sys::link_patches,
    wifi::{BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};

use heapless::String as HString;
use log::{error, info};
use std::{
    fmt::Write as _,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use esp_idf_svc::http::server::Configuration as HttpCfg;
mod audio;
mod http;
mod i2s;
mod openai;
mod azure_tts;

/* ------------ date Wi-Fi -------------------------------------------- */
const STA_SSID: &str = "Constantin)";
const STA_PASS: &str = "11111111";

/* ------------ iniţializare STA -------------------------------------- */
fn init_sta() -> Result<Box<BlockingWifi<EspWifi<'static>>>> {
    let per   = Peripherals::take()?;
    let sys   = EspSystemEventLoop::take()?;
    let nvs   = EspDefaultNvsPartition::take()?;
    let modem = per.modem;

    let mut ssid: HString<32> = HString::new();
    ssid.push_str(STA_SSID).unwrap();
    let mut pass: HString<64> = HString::new();
    pass.push_str(STA_PASS).unwrap();

    let wifi_drv = EspWifi::new(modem, sys.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(wifi_drv, sys)?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid,
        password: pass,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;               // DHCP gata

    // info
    let ip  = wifi.wifi().sta_netif().get_ip_info()?.ip;
    let mac = wifi.wifi().sta_netif().get_mac()?;
    let mut mac_s = HString::<18>::new();
    write!(
        mac_s,
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
    .unwrap();
    info!("IP  : {ip}");
    info!("MAC : {mac_s}");
    println!("Copiază în front-end ➜ const BACKEND_HOST = \"http://{ip}\";");

    Ok(Box::new(wifi))
}
/* -------------------------------------------------------------------- */

fn main() -> Result<()> {
    link_patches();
    EspLogger::initialize_default();

    // 1️⃣  Wi-Fi  (blocant până obţine IP)
    let wifi: &'static mut BlockingWifi<_> = Box::leak(init_sta()?);

    // 2️⃣  I²S + test TTS
    let i2s = Arc::new(std::sync::Mutex::new(i2s::init()?));

    // 3️⃣  canale WAV / text
    let (tx_http2audio, rx_http2audio) = mpsc::channel::<Vec<u8>>();
    let (tx_audio2http, rx_audio2http) = mpsc::channel::<(String, String)>();
    let rx_audio2http = Arc::new(Mutex::new(rx_audio2http));

    /* înainte de canalele WAV / text existente */
    let (tx_tts, rx_tts) = mpsc::channel::<String>();

    const TTS_STACK: usize = 24 * 1024;            // 24 KB – suficient pentru TLS

    {
        let i2s_ref = i2s.clone();
        std::thread::Builder::new()
            .name("tts_worker".into())
            .stack_size(TTS_STACK)                  // 👈 stack mai mare
            .spawn(move || {
                while let Ok(txt) = rx_tts.recv() {
                    log::info!("🔊 TTS worker: \"{txt}\"");
                    let mut i2s = i2s_ref.lock().unwrap();
                    if let Err(e) = azure_tts::tts_and_play(&mut *i2s, &txt) {
                        log::error!("tts_and_play error: {:?}", e);
                    }
                }
            })
            .unwrap();      // dacă nu porneşte vrem panic în build-time
    }

    // task audio
    {
        let rx_audio2http = rx_audio2http.clone();
        thread::spawn(move || {

            let _ = audio::audio_task(rx_http2audio, tx_audio2http);

            drop(rx_audio2http);     // nu se atinge niciodată, dar linter-ul e fericit
        });
    }

    // 4️⃣  HTTP server – retry până porneşte
    loop {
        let cfg = HttpCfg {
            max_uri_handlers: 16,
            stack_size: 8192,
            ..Default::default()
        };

        match EspHttpServer::new(&cfg) {
        Ok(mut server) => {
            if let Err(e) = http::register_handlers(
                &mut server,
                tx_http2audio.clone(),
                rx_audio2http.clone(),
                i2s.clone(),
                tx_tts.clone(),
            ) {
                error!("register_handlers error: {e:?}. Reîncerc în 2 s…");
                thread::sleep(Duration::from_secs(2));
                continue;
            }

            // (2) – înlocuieşte unwrap_or_default()
            use core::net::Ipv4Addr;
            let ip = match wifi.wifi().sta_netif().get_ip_info() {
                Ok(info) => info.ip,
                Err(_)   => Ipv4Addr::new(0, 0, 0, 0),
            };

            info!("HTTP ready – http://{ip}/");
            thread::sleep(Duration::from_secs(1));
            
            Box::leak(Box::new(server));
            break;
        }
        Err(e) => {
            error!("EspHttpServer::new error: {e:?}. Reîncerc în 2 s…");
            thread::sleep(Duration::from_secs(2));
        }
    }
    }


    /* 5️⃣  bucla principală – watchdog Wi-Fi & idle */
    loop {
        //  fără propagare de erori!
        if wifi.is_started().unwrap_or(false)
            && !wifi.is_connected().unwrap_or(false)
        {
            log::warn!("Wi-Fi down – reconectez…");

            // secvenţa de reconectare – nu verifica rezultatul,
            // doar încearcă de câteva ori
            for _ in 0..3 {
                let _ = wifi.stop();
                let _ = wifi.start();
                if wifi.connect().is_ok() {
                    break;
                }
            }
        }

    
    thread::sleep(Duration::from_secs(5));
}

   


}
