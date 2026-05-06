# Agent Context: PCM-to-PDM TX Mode for esp-hal I2S

## Project Overview

This is `esp-hal`, the Rust HAL for Espressif ESP32 microcontrollers. We are contributing a new feature: **I2S PCM-to-PDM TX mode**.

## Current Branch

- **Branch**: `i2s-pcm2pdm`
- **PR Branch**: `i2s-pcm2pdm-pr`
- **Target PR**: `i2s-pcm2pdm-pr` → `main`
- **Status**: Single squashed commit ready for review

### Branch Strategy

| Branch | Purpose | Commits |
|--------|---------|---------|
| `i2s-pcm2pdm` | Development history (preserved) | Multiple (2+) |
| `i2s-pcm2pdm-pr` | Clean PR for upstream review | Single squashed commit |

To recreate the PR branch after making fixes to `i2s-pcm2pdm`:

```bash
git checkout i2s-pcm2pdm-pr
git reset --hard main
git merge --squash i2s-pcm2pdm
git commit -m "feat(i2s): add PCM-to-PDM TX mode support

- Add \`I2s::new_pcm_to_pdm_tx()\` and \`PcmToPdmTxConfig\`
- Support ESP32-C3, C5, C6, C61, H2, S3 (i2s_version != \"1\")
- Add \`bclk_div: Option<u32>\` for automatic/manual BCLK divider selection
- Add example \`examples/peripheral/i2s/pcm_to_pdm/\`
- Update CHANGELOG"
git push -f origin i2s-pcm2pdm-pr
```

## Feature: PCM-to-PDM TX Mode

### What it does
The I2S peripheral can convert PCM audio data to PDM (Pulse Density Modulation) signals in hardware. This enables:
- Direct PDM speaker/headphone drive without external DAC
- One-line or two-line DAC output
- Stereo/mono configuration

### Supported Chips

| Chip | i2s_version | Clock Config | Two-Line DAC | Hardware Verified |
|------|-------------|--------------|--------------|-------------------|
| ESP32-C3 | "2" | I2S registers | ✅ | Code-level |
| ESP32-S3 | "2" | I2S registers | ✅ | **Yes (Two-Line)** |
| ESP32-C6 | "2" | PCR peripheral | ✅ | Code-level |
| ESP32-H2 | "3" | PCR peripheral | ✅ | Code-level |
| ESP32-C5 | "3" | PCR peripheral | ✅ | Code-level |
| ESP32-C61 | "3" | PCR peripheral | ✅ | Code-level |

*ESP32 and ESP32-S2 (i2s_version = "1") are NOT supported.*

### Key API

```rust
let i2s = I2s::new_pcm_to_pdm_tx(
    peripherals.I2S0,  // I2S1 NOT supported
    peripherals.DMA_CH0,
    PcmToPdmTxConfig::default()
        .with_sample_rate(Rate::from_hz(48000))
        .with_line_mode(PcmToPdmTxLineMode::TwoLineDac),
)?;

let mut i2s_tx = i2s
    .i2s_tx
    .with_dout(peripherals.GPIO3)
    .with_dout2(peripherals.GPIO4)  // ESP32-S3 only for now
    .build(tx_descriptors);
```

### Architecture Decisions

1. **`#[cfg(not(i2s_version = "1"))]` gating**
   - Used instead of explicit chip lists (`any(esp32c3, ...)`)
   - Consistent with upstream esp-hal conventions
   - Automatically includes future chips with i2s_version != "1"

2. **`bclk_div: Option<u32>`**
   - `None` (default): automatic search [8, 64] for optimal clock accuracy
   - `Some(x)`: manual fixed divider (must be >= 8)
   - Eliminates ambiguity of the previous `auto_bclk_div + bclk_div` combination

3. **Clock configuration paths**
   - ESP32-C3, ESP32-S3: Direct I2S register block (`tx_clkm_conf`, `tx_clkm_div_conf`)
   - ESP32-C6, ESP32-H2, ESP32-C5, ESP32-C61: PCR peripheral (`i2s_tx_clkm_conf`, `i2s_tx_clkm_div_conf`)

4. **`tx_bck_div_num` register location**
   - ESP32-C6: `tx_conf1`
   - All others (C3, S3, H2, C5, C61): `tx_conf`

5. **ESP-IDF workarounds (C3/S3 only)**
   - Double-division workaround (always)
   - PDM TX noise reduction (opt-in via `pdm_tx_noise_reduction`)
   - MCLK binding to TX clock (always)
   - Disable BCK/WS sharing (always)

## Important Files

- `esp-hal/src/i2s/master.rs` — Main implementation (≈450 lines added)
- `examples/peripheral/i2s/pcm_to_pdm/src/main.rs` — Example with 1kHz square wave
- `esp-hal/CHANGELOG.md` — Entry in `[Unreleased] > Added`

## Build Commands

```bash
# Check library
cd esp-hal && cargo check --features esp32c6 --target riscv32imac-unknown-none-elf

# Build example for specific chip
cargo xtask build examples pcm_to_pdm --chip esp32c6
cargo xtask build examples pcm_to_pdm --chip esp32c3
cargo xtask build examples pcm_to_pdm --chip esp32h2
cargo xtask build examples pcm_to_pdm --chip esp32s3  # uses +esp toolchain

# Format
cargo xtask fmt-packages
```

## Reviewer Q&A Prepared

### Why does `new_pcm_to_pdm_tx` return a full `I2s` struct instead of just TX?
Because `I2s` struct requires both `i2s_rx` and `i2s_tx` fields for API consistency. RX is disabled via register configuration (`rx_tdm_en = 0`, `rx_pdm_en = 0`) to prevent interference.

### Why `Option<u32>` for `bclk_div`?
To express "automatic vs manual" at the type level. Previous design had separate `bclk_div` and `auto_bclk_div` fields which could create ambiguous configurations.

### Why `#[cfg(not(i2s_version = "1"))]` instead of chip lists?
Upstream convention. `i2s_version` metadata distinguishes chip generations:
- "1": ESP32, ESP32-S2 (no PCM-to-PDM TX hardware)
- "2": ESP32-C3, ESP32-C6, ESP32-S3
- "3": ESP32-C5, ESP32-C61, ESP32-H2, ESP32-P4 (P4 base I2S driver not yet supported)

### Why is `with_dout2` gated by `#[cfg(esp32s3)]`?
**Historical note**: This was the original implementation. We discovered that ALL `i2s_version != "1"` chips support two-line DAC. However, signal names differ:
- C3/C6/H2/C5/C61: `I2SO_SD1`
- S3: `I2S0O_SD1`

The current code handles this with `cfg_if` in the `dout2_signal()` implementation.

## Testing Notes

- **Hardware verified**: ESP32-S3 with headphones via LPF + DC blocking capacitor
- **Example output**: 1kHz square wave (audible tone for easy verification)
- **Stereo**: Same sample on both channels
- **Wiring**: GPIO3 (dout), GPIO4 (dout2 for S3 two-line mode)

## PR Checklist Status

- [x] Example added
- [x] `cargo xtask fmt-packages` executed
- [x] CHANGELOG updated
- [x] `#[instability::unstable]` on new public API
- [x] `i2s_version` cfg convention followed
- [x] All 4 primary chips build successfully
- [x] Documentation updated

## Known Limitations

1. **I2S1 not supported** — returns `ConfigError::PcmToPdmTxNotSupported`
2. **No BCLK/WS pins** — PDM TX generates clock internally
3. **Simplex TX only** — Full-duplex not verified
4. **ESP32-C5/C61** — Code-level support only, no hardware verification
5. **ESP32-P4** — Base I2S driver not yet supported in esp-hal (`device.i2s.support_status = "not_supported"` in metadata). P4 has 3 I2S instances (I2S0/1/2), uses non-PCR clock config (`i2s.clock_configured_by_pcr = false`), and requires base I2S porting (including 3-instance handling) before PCM-to-PDM TX can work.

## Maintenance Notes

### CHANGELOG Formatting
- Keep each entry concise; do not cram multiple concepts into one sentence
- Use sub-bullets (indented `-`) for related API additions within the same feature
- Avoid inline code blocks that make lines excessively long

### PR Branch Maintenance
After fixing anything on `i2s-pcm2pdm` (e.g., CHANGELOG rewording), always recreate the squashed `i2s-pcm2pdm-pr` branch and force-push:
```bash
git checkout i2s-pcm2pdm-pr
git reset --hard main
git merge --squash i2s-pcm2pdm
git commit -m "..."
git push -f origin i2s-pcm2pdm-pr
```

## Related Archive Docs

- `/home/totto/Documents/Rust/_Archive/lx7-defmt2/I2S_PDM_TX_IMPLEMENTATION_en.md`
- `/home/totto/Documents/Rust/_Archive/lx7-defmt2/I2S_PDM_TX_IMPLEMENTATION_ja.md`
- `/home/totto/Desktop/PR.txt` — PR template
