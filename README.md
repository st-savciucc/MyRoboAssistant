# MyRoboAssistant

Interactive robotic assistant built in Rust on a Raspberry Pi Pico 2W.

## Requirements

- Rust (1.60+)
- `cargo` (comes with Rust)
- `embassy` async framework
- Hardware:
  - Raspberry Pi Pico 2W
  - Display (TFT/LCD)
  - Microphone (I2S)
  - Motors + driver
  - Proximity sensors, camera, speaker, battery pack

## Project Structure

```
MyRoboAssistant/
├── Cargo.toml         # manifest & dependencies
├── src/
│   └── main.rs        # entry point
├── README.md          # this file
└── docs/              # optional extra docs
```

## Quick Start

```bash
# clone your repo
git clone https://github.com/st-savciucc/MyRoboAssistant.git
cd MyRoboAssistant

# build & run on host (tests, examples)
cargo build --release

# to flash onto Pico 2W, use your preferred tool:
# e.g., probe-rs or picotool
# probe-rs run --chip RP2040 --target thumbv6m-none-eabi

```

## License

MIT License © 2025 Savchuk Kostiantyn
