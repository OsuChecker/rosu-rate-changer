# Osu Rate Changer Library

A Rust library for modifying osu! beatmap and audio speeds with high precision.

## Library Features

- Rate modification for .osu files and associated audio
- Audio speed adjustment with quality preservation
- Comprehensive error handling with `eyre`
- Safe file path handling
- Cross-platform compatibility

## Installation

Add this to your `Cargo.toml`:
```toml
[dependencies]
osu_rate_changer = "0.1.0"
```

## API Usage

```rust
use osu_rate_changer::{change_osu_speed, change_audio_speed};
use eyre::Result;

// Modify both beatmap and audio (Recommended)
fn rate_map(osu_file_path: &str, rate: f32) -> Result<()> {
}

// Modify only audio
fn change_audio_speed(input_path: &str, output_path: &str, rate: f32) -> Result<()> {
}

// Modify only .osu file
pub fn change_osu_speed(map: &mut Beatmap, input_path: &str, rate: f32, audio_path: &str) -> eyre::Result<()> {
}
```

## Technical Specifications

### Audio Processing Parameters
```rust
const SINC_LEN: usize = 256;
const F_CUTOFF: f64 = 0.95;
const OVERSAMPLING_FACTOR: usize = 256;
const BUFFER_SIZE: usize = 1152;
const DEFAULT_SAMPLE_RATE: u32 = 44100;
const BITS_PER_SAMPLE: u16 = 16;
```

## Roadmap & Known Issues

1. **Audio Processing**
   - **Issues:**
     - Limited audio compression with current OGG format
     - High memory usage with large files
   - **Improvements:**
     - Implement Opus-OGG support via audiopus
     - Add stream processing for memory efficiency
     - Make audio processing parameters configurable
2. **Timing & Map Features**
   - **Issues:**
     - Breakpoint alignment causing unexpected breaks during gameplay
     - Inaccurate BPM display in editor
   - **Improvements:**
     - Enhance timing point synchronization algorithm
     - Implement precise breakpoint calculations
     - Better rate scaling for extreme values


## Contributing

Contributions are welcome!<br>
Create an issue or contact me on discord : Osef0760  

## License
GNU license : just mention me and you are good to use how you want