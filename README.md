# MyRoboAssistant
Dig. Talk. Emote.

:::info
- **Student:** Savchuk Kostiantyn  
- **Group:** 331CC  
- **Date:** 2025  
- **GitHub Repository:** [MyRoboAssistant](https://github.com/st-savciucc/MyRoboAssistant.git)
:::

## Description
MyRoboAssistant is a mid-sized (approx. 140 cm × 80 cm) mobile robot focused on natural voice interaction and emotional feedback. It suggests useful tips, shows animated emotions on a TFT screen, and can be extended via a companion mobile app.

## Motivation
This project combines my passion for robotics with safe, high-performance embedded Rust. Rust (via the embassy-rs framework) guarantees memory-safe firmware, while the Raspberry Pi Pico 2 W provides built-in Wi-Fi/BLE for easy connectivity.

## Architecture

![Hardware Architecture](./Diagrams/Diagrama-de-arhitectura.svg)

### Hardware Blocks

#### Raspberry Pi Pico 2 W
**Role:** Central microcontroller, manages I/O and overall device logic  
**Connections:**  
- TFT display (SPI)  
- Ultrasonic sensors (GPIO trigger/echo)  
- I2S microphone (I2S data/clk)  
- Speaker via I2S DAC  
- DC motors + L298N driver (GPIO + PWM)  
- SG90 servo (PWM)  

#### TFT Display (ST7735 480×320)
**Interface:** SPI  
**Connections:**  
- SDA (MOSI) → Pico SPI MOSI pin  
- SCL (SCK) → Pico SPI SCK pin  
- DC, RST, CS → separate GPIOs  
**Role:** Shows emotions, status, and menu  

#### DC Motors + Wheels
**Interface:** Powered via L298N driver  
**Connections:**  
- IN1–IN4 → Pico GPIO for direction  
- EN1–EN2 → Pico PWM for speed  
**Role:** Locomotion  

#### L298N Dual Motor Driver
**Interface:** GPIO + PWM  
**Connections:**  
- VCC, GND → battery/charger  
- IN1–IN4, EN1–EN2 → Pico  
- OUT1–OUT4 → motors  
**Role:** Drives the DC motors  

#### SG90 Micro Servo
**Interface:** PWM  
**Connections:**  
- Control → Pico PWM pin  
- Power → 5 V + GND  
**Role:** Arm gesture  

#### Ultrasonic Sensors (HC-SR04)
**Interface:** GPIO  
**Connections:**  
- Trigger → Pico GPIO  
- Echo → Pico GPIO  
**Role:** Proximity detection  

#### I2S Microphone
**Interface:** I2S  
**Connections:**  
- WS, CK, SD → Pico I2S pins  
**Role:** Voice input  

#### I2S DAC (MAX98357A) + Speaker
**Interface:** I2S  
**Connections:**  
- BCLK, LRCLK, DIN → Pico I2S pins  
- Speaker → output of MAX98357A  
**Role:** High-quality audio output  

---

## Weekly Log

| Week | Activities                                   |
|------|----------------------------------------------|
| 1    | Requirements analysis, embassy-rs study      |
| 2    | Motor driver tests, display driver bring-up  |
| …    | _to be filled weekly_                       |

---

## Bill of Materials (BOM)

| Component                                                       | Qty | Purpose / Link                                                                                                                     |
|-----------------------------------------------------------------|-----|------------------------------------------------------------------------------------------------------------------------------------|
| Raspberry Pi Pico 2 W                                           | 1   | Main MCU with Wi-Fi/BLE                                                                                                            |
| [6 V 12 Ah Pb-acid battery + charger](https://www.optimusdigital.ro/en/lead-acid-batteries/10190-lead-acid-battery-6-v-12a.html) | 1   | Portable power                                                                                                                     |
| [Micro metal DC gear-motor](https://www.optimusdigital.ro/en/micro-gearmotors/10348-10001-micro-metal-gearmotor-hpcb-6v.html?search_query=DC+motor&results=1033) | 2   | Robot locomotion                                                                                                                   |
| [L298N dual motor driver](https://www.optimusdigital.ro/en/brushed-motor-drivers/145-l298n-dual-motor-driver.html?search_query=L298N+motor%C2%A0driver&results=4) | 1   | Drives the DC motors                                                                                                               |
| [SG90 micro servo](https://www.optimusdigital.ro/en/servomotors/2261-micro-servo-motor-sg90-180.html?search_query=Servo%C2%A0SG90&results=11)         | 1   | Arm gesture                                                                                                                        |
| [3.5″ TFT 480×320 touchscreen](https://www.optimusdigital.ro/en/lcds/3333-tft-35-480-x-320-adafruit-display-with-touchscreen-for-raspberry-pi.html?search_query=1.8%E2%80%B0%C2%A0TFT&results=66) | 1   | Emotion display                                                                                                                    |
| HC-SR04 ultrasonic sensor                                        | 2   | Proximity                                                                                                                          |
| I2S MEMS microphone                                             | 1   | Voice input                                                                                                                        |
| Mini speaker + AMP                                              | 1   | Audio output                                                                                                                       |

---

## Software Design

| Crate / Lib          | Purpose                         | Link                                              |
|----------------------|---------------------------------|---------------------------------------------------|
| `embassy` / `embassy-net` | Async runtime & networking      | https://crates.io/crates/embassy                  |
| `embedded-hal`       | Hardware abstraction layer      | https://crates.io/crates/embedded-hal             |
| `heapless`           | Fixed-size containers           | https://crates.io/crates/heapless                 |
| `serde` / `serde_json` | Serialization                   | https://crates.io/crates/serde                    |
| `embedded-graphics`  | 2D graphics on TFT              | https://crates.io/crates/embedded-graphics        |
| `rust-voice`         | Speech recognition              | https://crates.io/crates/rust-voice               |

### Main Firmware Tasks
- **comm** – Wi-Fi/BLE connect & messaging  
- **emotion_display** – emoji animation on TFT  
- **motion_control** – motor & servo control  
- **speech** – audio capture & keyword spotting  

![Software Flow](./Diagrams/Diagrama-de-flux-software.svg)

---

## Supporting Files
- `Diagrama-de-arhitectura.svg` – hardware block diagram  
- `Diagrama-de-flux-software.svg` – firmware flowchart  

---

*Signed-off-by: Savchuk Kostiantyn <kostiantyn.savchuk@stud.acs.upb.ro>*
