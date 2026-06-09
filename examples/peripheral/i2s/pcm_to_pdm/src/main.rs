//! I2S PCM-to-PDM TX example.
//!
//! This example demonstrates how to use the I2S peripheral in PCM-to-PDM TX
//! mode. It continuously transmits PDM data generated from PCM samples via DMA.
//!
//! The following wiring is assumed:
//! - GPIO3 -> PDM data output (dout)
//!
//! Connect this pin to a low-pass filter (RC filter) followed by a DC
//! blocking capacitor (HPF) and an amplifier to drive headphones or a
//! speaker.
//!
//! To use two-line DAC mode, set the line mode to `TwoLineDac` and add
//! `.with_dout2(peripherals.GPIO4)` when building the TX channel.

#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma_buffers,
    i2s::master::{I2s, PcmToPdmTxConfig},
    main,
    time::Duration,
};
use log::info;

esp_bootloader_esp_idf::esp_app_desc!();

const BUFFER_SIZE: usize = 48000;

const SINE_WAVE_FREQ: usize = 1000;
const SAMPLES_PER_PERIOD: usize = 48000 / SINE_WAVE_FREQ; // 48

// 48 samples sine wave lookup table for 1kHz at 48kHz sample rate
const SINE_TABLE: [i16; 48] = [
    0, 4277, 8481, 12539, 16383, 19947, 23170, 25996, 28377, 30273, 31650, 32487, 32767, 32487,
    31650, 30273, 28377, 25996, 23170, 19947, 16383, 12539, 8481, 4277, 0, -4277, -8481, -12539,
    -16383, -19947, -23170, -25996, -28377, -30273, -31650, -32487, -32767, -32487, -31650, -30273,
    -28377, -25996, -23170, -19947, -16384, -12539, -8481, -4277,
];

fn fill_buffer(tx_buffer: &mut [u8]) {
    for (i, chunk) in tx_buffer.chunks_exact_mut(2).enumerate() {
        let sample = SINE_TABLE[i % SAMPLES_PER_PERIOD];
        chunk.copy_from_slice(&sample.to_le_bytes());
    }
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = esp_hal::init(esp_hal::Config::default());

    let delay = Delay::new();

    let (_, _, tx_buffer, tx_descriptors) = dma_buffers!(0, BUFFER_SIZE);

    fill_buffer(tx_buffer);

    let i2s = I2s::new_pcm_to_pdm_tx(
        peripherals.I2S0,
        peripherals.DMA_CH0.into(),
        PcmToPdmTxConfig::default(),
    )
    .unwrap();

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
