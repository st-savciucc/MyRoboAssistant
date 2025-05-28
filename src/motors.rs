use esp_idf_hal::{
    ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution, LEDC},
    gpio::{Gpio18, Gpio19, Gpio21, Gpio22},
    units::KiloHertz,
    prelude::*,
};
use core::ptr;
#[derive(Copy, Clone)]
pub enum MotorId {
    Left,
    Right,
}

pub struct L9110S<'d> {
    m1_a: LedcDriver<'d>,
    m1_b: LedcDriver<'d>,
    m2_a: LedcDriver<'d>,
    m2_b: LedcDriver<'d>,
}

impl<'d> L9110S<'d> {
    pub fn new(
        ledc: &mut LEDC,
        m1_a: Gpio18, m1_b: Gpio19,
        m2_a: Gpio21, m2_b: Gpio22,
    ) -> anyhow::Result<Self> {
       let timer = LedcTimerDriver::new(
           unsafe { ptr::read(&ledc.timer1) },
           &TimerConfig::default()
               .frequency(20.kHz().into())
               .resolution(Resolution::Bits10),
       )?;

        let m1_a = LedcDriver::new(unsafe { ptr::read(&ledc.channel2) }, &timer, m1_a)?;
        let m1_b = LedcDriver::new(unsafe { ptr::read(&ledc.channel3) }, &timer, m1_b)?;
        let m2_a = LedcDriver::new(unsafe { ptr::read(&ledc.channel4) }, &timer, m2_a)?;
        let m2_b = LedcDriver::new(unsafe { ptr::read(&ledc.channel5) }, &timer, m2_b)?;

        Ok(Self { m1_a, m1_b, m2_a, m2_b })
    }

    /// speed ∈ [-100, 100] (%)
    pub fn drive(&mut self, id: MotorId, speed: i8) -> anyhow::Result<()> {
        let duty_max = self.m1_a.get_max_duty();
        let duty = ((speed.abs() as u32).min(100) * duty_max) / 100;

        let (fwd, rev) = match id {
            MotorId::Left  => (&mut self.m1_a, &mut self.m1_b),
            MotorId::Right => (&mut self.m2_a, &mut self.m2_b),
        };

        if speed > 0 {
            fwd.set_duty(duty)?;
            rev.set_duty(0)?;
        } else if speed < 0 {
            fwd.set_duty(0)?;
            rev.set_duty(duty)?;
        } else {
            fwd.set_duty(0)?;
            rev.set_duty(0)?;
        }
        Ok(())
    }
}
