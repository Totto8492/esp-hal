//! I2S PCM-to-PDM TX example.
//!
//! This example demonstrates how to use the I2S peripheral in PCM-to-PDM TX
//! mode. It continuously transmits PDM data generated from PCM samples via DMA.
//!
//! The following wiring is assumed:
//! - GPIO3 -> PDM data output (dout)
//! - GPIO4 -> PDM data output 2 (dout2) (two-line DAC mode only)
//!
//! Connect these pins to a low-pass filter (RC filter) followed by a DC
//! blocking capacitor (HPF) and an amplifier to drive headphones or a
//! speaker.

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma_buffers,
    i2s::master::{Channels, DataFormat, I2s, PcmToPdmTxConfig, PcmToPdmTxLineMode},
    main,
    time::{Duration, Rate},
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

const BUFFER_SIZE: usize = 48000;

const SINE_WAVE_FREQ: usize = 1000;
const SAMPLES_PER_PERIOD: usize = 48000 / SINE_WAVE_FREQ; // 48

#[cfg(any(
    feature = "esp32c3",
    feature = "esp32c6",
    feature = "esp32h2",
    feature = "esp32s3"
))]
const NUM_CHANNELS: usize = 2;

#[cfg(any(feature = "esp32c5", feature = "esp32c61"))]
const NUM_CHANNELS: usize = 1;

// 48 samples sine wave lookup table for 1kHz at 48kHz sample rate
const SINE_TABLE: [i16; 48] = [
    0, 4277, 8481, 12539, 16383, 19947, 23170, 25996, 28377, 30273, 31650, 32487, 32767, 32487,
    31650, 30273, 28377, 25996, 23170, 19947, 16383, 12539, 8481, 4277, 0, -4277, -8481, -12539,
    -16383, -19947, -23170, -25996, -28377, -30273, -31650, -32487, -32767, -32487, -31650,
    -30273, -28377, -25996, -23170, -19947, -16384, -12539, -8481, -4277,
];

fn fill_buffer(tx_buffer: &mut [u8]) {
    for (i, chunk) in tx_buffer.chunks_exact_mut(2 * NUM_CHANNELS).enumerate() {
        let sample = SINE_TABLE[i % SAMPLES_PER_PERIOD];
        let bytes = sample.to_le_bytes();
        for ch in 0..NUM_CHANNELS {
            chunk[ch * 2] = bytes[0];
            chunk[ch * 2 + 1] = bytes[1];
        }
    }
}

fn pdm_config() -> PcmToPdmTxConfig {
    let config = PcmToPdmTxConfig::default()
        .with_sample_rate(Rate::from_hz(48000))
        .with_data_format(DataFormat::Data16Channel16);

    #[cfg(any(
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2",
        feature = "esp32s3"
    ))]
    let config = config
        .with_slot_mode(Channels::STEREO)
        .with_line_mode(PcmToPdmTxLineMode::TwoLineDac);

    #[cfg(any(feature = "esp32c5", feature = "esp32c61"))]
    let config = config
        .with_slot_mode(Channels::MONO)
        .with_line_mode(PcmToPdmTxLineMode::OneLineDac);

    config
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let delay = Delay::new();

    let (_, _, tx_buffer, tx_descriptors) = dma_buffers!(0, BUFFER_SIZE);

    // Fill the buffer with a 1kHz sine wave for easy verification.
    // At 48kHz sample rate, one period is 48 samples.
    // Connect the output to headphones via a low-pass filter and a DC blocking
    // capacitor to hear the tone.
    fill_buffer(tx_buffer);

    let i2s = I2s::new_pcm_to_pdm_tx(peripherals.I2S0, peripherals.DMA_CH0, pdm_config()).unwrap();

    #[cfg(any(
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2",
        feature = "esp32s3"
    ))]
    let mut i2s_tx = i2s
        .i2s_tx
        .with_dout(peripherals.GPIO3)
        .with_dout2(peripherals.GPIO4)
        .build(tx_descriptors);

    #[cfg(any(feature = "esp32c5", feature = "esp32c61"))]
    let mut i2s_tx = i2s
        .i2s_tx
        .with_dout(peripherals.GPIO3)
        .build(tx_descriptors);

    let _transfer = i2s_tx.write_dma_circular(&tx_buffer).unwrap();

    info!("PCM-to-PDM TX transfer started");

    loop {
        delay.delay(Duration::from_secs(5));
        info!("Transmitting...");
    }
}
