//! I2S PCM-to-PDM TX example.
//!
//! This example demonstrates how to use the I2S peripheral in PCM-to-PDM TX
//! mode. It continuously transmits PDM data generated from PCM samples via DMA.
//!
//! The following wiring is assumed:
//! - GPIO3 -> PDM data output (dout)
//! - GPIO4 -> PDM data output 2 (dout2) (ESP32-S3 two-line DAC mode only)
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

#[main]
fn main() -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let delay = Delay::new();

    let (_, _, tx_buffer, tx_descriptors) = dma_buffers!(0, BUFFER_SIZE);

    // Fill the buffer with a 1kHz square wave for easy verification.
    // At 48kHz sample rate, one period is 48 samples.
    // Connect the output to headphones via a low-pass filter and a DC blocking
    // capacitor to hear the tone.
    const SQUARE_WAVE_FREQ: usize = 1000;
    const SAMPLES_PER_PERIOD: usize = 48000 / SQUARE_WAVE_FREQ; // 48

    for (i, chunk) in tx_buffer.chunks_exact_mut(4).enumerate() {
        let sample: i16 = if (i % SAMPLES_PER_PERIOD) < (SAMPLES_PER_PERIOD / 2) {
            i16::MAX // positive half
        } else {
            i16::MIN // negative half
        };
        let bytes = sample.to_le_bytes();
        // Stereo: same sample on both channels (little-endian)
        chunk[0] = bytes[0]; // left low
        chunk[1] = bytes[1]; // left high
        chunk[2] = bytes[0]; // right low
        chunk[3] = bytes[1]; // right high
    }

    let i2s = I2s::new_pcm_to_pdm_tx(
        peripherals.I2S0,
        peripherals.DMA_CH0,
        PcmToPdmTxConfig::default()
            .with_sample_rate(Rate::from_hz(48000))
            .with_data_format(DataFormat::Data16Channel16)
            .with_slot_mode(Channels::STEREO)
            .with_line_mode(PcmToPdmTxLineMode::TwoLineDac),
    )
    .unwrap();

    cfg_if::cfg_if! {
        if #[cfg(feature = "esp32s3")] {
            let mut i2s_tx = i2s
                .i2s_tx
                .with_dout(peripherals.GPIO3)
                .with_dout2(peripherals.GPIO4)
                .build(tx_descriptors);
        } else {
            let mut i2s_tx = i2s
                .i2s_tx
                .with_dout(peripherals.GPIO3)
                .build(tx_descriptors);
        }
    }

    let _transfer = i2s_tx.write_dma_circular(&tx_buffer).unwrap();

    info!("PCM-to-PDM TX transfer started");

    loop {
        delay.delay(Duration::from_secs(5));
        info!("Transmitting...");
    }
}
