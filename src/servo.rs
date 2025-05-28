use esp_idf_hal::{
    ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution, LEDC},
    gpio::{Gpio16, Gpio17},
    prelude::*,
};
use core::{mem, ptr}; 
#[derive(Copy, Clone)]
pub enum ServoId {
    Left,
    Right,
}

pub struct DualServo<'d> {
    ch1: LedcDriver<'d>,
    ch2: LedcDriver<'d>,
}

impl<'d> DualServo<'d> {
    pub fn new(
        ledc: &mut LEDC,
        gpio16: Gpio16,
        gpio17: Gpio17,
    ) -> anyhow::Result<Self> {
        // timer de 50 Hz
        let timer = LedcTimerDriver::new(
            unsafe { ptr::read(&ledc.timer0) },               // ⚠️  mut-take
            &TimerConfig::default()
                .frequency(50.Hz())
                .resolution(Resolution::Bits15),
        )?;

        let ch1 = LedcDriver::new(unsafe { ptr::read(&ledc.channel0) }, &timer, gpio16)?;
        let ch2 = LedcDriver::new(unsafe { ptr::read(&ledc.channel1) }, &timer, gpio17)?;

        Ok(Self { ch1, ch2 })
    }

    /// setează unghiul servo (0-180°)
    pub fn set_angle(&mut self, id: ServoId, deg: f32) -> anyhow::Result<()> {
        let duty_max = self.ch1.get_max_duty();      // 20 ms perioadă
        let duty_min = duty_max / 20;                // ≈1 ms  (0°)
        let duty_max_servo = duty_max / 10;          // ≈2 ms (180°)

        let duty = duty_min
            + ((deg.clamp(0.0, 180.0) / 180.0) * (duty_max_servo - duty_min) as f32) as u32;

        match id {
            ServoId::Left  => self.ch1.set_duty(duty)?,
            ServoId::Right => self.ch2.set_duty(duty)?,
        }
        Ok(())
    }
}
