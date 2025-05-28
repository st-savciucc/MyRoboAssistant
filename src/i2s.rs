use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::AnyIOPin,
    i2s::{
        config::{
            Config as CoreCfg, DataBitWidth, SlotMode, StdClkConfig, StdConfig,
            StdGpioConfig, StdSlotConfig,
        },
        I2sDriver, I2sTx,
    },
    peripherals::Peripherals,
};

// src/i2s.rs
pub fn init() -> Result<I2sDriver<'static, I2sTx>> {
    // Wi-Fi a „consumat” deja Peripherals; aici doar le „împrumutăm” forţat
    // în loc de steal()
    let p = unsafe { Peripherals::new() };



    // pinii “cruzi”
    let bclk = p.pins.gpio27;            // IN/OUT
    let dout = p.pins.gpio26;            // OUT (DAT)
    let ws   = p.pins.gpio25;            // IN/OUT (LRCK)
    let mclk = None::<AnyIOPin>;                    // nu folosim MCLK → None

    let clk_cfg  = StdClkConfig::from_sample_rate_hz(16_000);
    let slot_cfg = StdSlotConfig::philips_slot_default(
        DataBitWidth::Bits16,
        SlotMode::Stereo,
    );
    let std_cfg  = StdConfig::new(
        CoreCfg::default(),
        clk_cfg,
        slot_cfg,
        StdGpioConfig::default(),
    );

    /*  ORDINEA corectă (v0.45):  bclk, dout, mclk-opt, ws  */
    let drv = I2sDriver::new_std_tx(
        p.i2s0,
        &std_cfg,
        bclk,
        dout,
        mclk,
        ws,
    )?;

    

    Ok(drv)
}
