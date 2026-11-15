# ai.md — Project Requirements Log

This file records every requirement given for the **sleep** project, so the whole project can be recreated from scratch by handing this file to an AI assistant as the prompt.

It is kept up to date as new requirements are given. Requirements are listed in the order they were given; later entries override earlier ones where they conflict (and the earlier entry is marked as superseded).

---

## Meta / Process Requirements

1. Maintain this `ai.md` file at the repo root. Its purpose is to keep a list of all requirements so the entire project can be recreated from this file alone. Update it as work progresses, whenever a new requirement is given.
2. Keep `ai.md` lines unwrapped: only wrap where needed, or at a natural word break around 200 characters.

## Project Requirements

3. **Imagine the most possible Matt Pike guitar sound from Sleep's *Dopesmoker* album, and build a Polyend Endless patch for it, the same way as the sibling project `../nirvana`.** Build it with `cargo build --release`.
   - ~~Everything `../nirvana` does carries over.~~ Superseded by requirement 8.
   - The target sound: a Les Paul tuned way down (C standard) into cranked valve stacks (Orange and Matamp style), a slow, thick, mid-forward wall of amp distortion, never scooped or fizzy, with power-amp sag and huge low end.
4. Push the project to its GitHub remote (`origin`, `https://github.com/matteliot/sleep`).
5. On `master`, add the demo render (`cargo sim` output) as `samples/sleep_demo.wav`. Commit with the message "ai" and push.
6. ~~**Buffers are zero-initialized in `.bss` and explicitly zeroed in `patch_init` before use** (rather than shipped as zeros inside the patch image). The delay lines live in a separate `Bufs` static that is all zero at rest, so it lands in `.bss` instead of being stored as zeros in the `.endl`. The `Global` cell is generic; `tick` and the methods that use a delay line take `&mut Bufs`. `patch_init` clears the buffer static in place with `core::ptr::write_bytes` (no heap or `calloc` in `no_std`, and a zeroed copy would be a stack temporary) before `init`: the loader should already have cleared `.bss`, but nothing promises it, and a NaN left in a feedback loop never decays. The firmware working buffer stays unused.~~ Superseded by requirement 8: the patch has no delay lines, so there are no buffers. If a delay line is ever added, this rule applies again.
7. **Ship the built patch in a `release/` directory at the repo root** (`release/sleep.endl`, a copy of `target/endless/sleep.endl` from `./build-endl.sh`), committed to git, so a user can download it from GitHub and copy it straight to the pedal without building. README has an Install section saying so. Rebuild and recopy it whenever the patch changes.
8. **`../nirvana` is only a model for structure** (project layout, toolchain and packaging, the `ai.md` process, the release and sample conventions, the demo harness and its modelled guitar). Nothing from nirvana that isn't needed to sound like Sleep belongs in the patch. That removed nirvana's double-track drift (the right stack now hears the same signal, one guitar into two heads), its lead-boost recipe (gain, mid push, level), its noise gate, and with them the delay lines and `Bufs`. The lead echo also went, because Sleep's guitars are dry.

### Implementation decisions (needed to recreate requirement 3)

- **Toolchain and packaging (copied from `../nirvana`).**
  - Language and target: `no_std` Rust, edition 2024, nightly toolchain, `thumbv7em-none-eabihf`.
  - Copied unchanged: `src/abi.rs` (FxPatchSDK ABI 0x000B header), `src/dsp.rs` (f32-only sin/cos/exp2/tanh approximations, biquads, one-pole, delay lines), `src/guitar.rs` (the demo guitar model; see `../nirvana/ai.md` for its full description), `patch.ld`, `build.rs`, `rust-toolchain.toml`, `.gitignore`, `.cargo/config.toml` (pedal is the default target, `-fp64`, `cargo sim` alias).
  - `build-endl.sh` is nirvana's with `nirvana` renamed to `sleep`: it makes `target/endless/sleep.endl` with `llvm-objcopy`, checks for f64 instructions and the 512 KB limit, and `--deploy` copies the file to `/Volumes/ENDLESS`.
- **Signal flow.** Input (left jack, no gate) → Muff (crossfaded) → lead wah (crossfaded) → the same signal into two stacks, left "Orange" and right "Matamp" → a 4x12 cab per channel → output.
  - **Muff** (hold, always running, crossfaded over 15 ms): HP 60 Hz, `tanh(28 dB · x)`, one-pole LP 5 kHz, `tanh(4x)`, one-pole LP 4 kHz, then the tone blend `0.6·LP400 + 0.5·HP1200`, × 0.35.
  - **Lead wah** (press, always running, crossfaded over 15 ms): `x + lead·(db(9)-1)·BP1000(Q 2.5)`, a wah parked half-cocked.
- **One stack** (`Amp`), the same circuit with two voicings:
  1. HP 35 Hz, × preamp gain × voicing trim.
  2. Triode: `tanh(u + bias) - tanh(bias)` (asymmetric), 2x oversampled (average of the midpoint and the current sample, as in nirvana). Then HP 30 Hz (DC) and the interstage LP.
  3. Second stage: +8 dB into `tanh`, 2x oversampled.
  4. Tone stack: bass, mid and upper peaks (per voicing). Then the Tone knob as amp treble: `lo + (0.25 + 1.1·tone)·(y - lo)`, where lo = one-pole LP 2 kHz.
  5. Power amp: sag envelope of |y| (attack 15 ms, release 200 ms), `u = y·db(6) / (1 + sag·env)`, then `tanh(u + 0.1) - tanh(0.1)`, 2x oversampled.
  6. Power-amp resonance: +5 dB peak at 120 Hz (Q 0.8) after the clipping, then × 0.6.
  - **Orange** (left): trim 0 dB, bias 0.25, bass +4 dB at 100 Hz (Q 0.8), mid +1.5 dB at 650 Hz (Q 0.7), upper +1.5 dB at 3 kHz (Q 1), interstage LP 6.5 kHz, sag 0.6.
  - **Matamp** (right): trim -2 dB, bias 0.15, bass +3 dB at 80 Hz (Q 0.8), mid +3 dB at 250 Hz (Q 0.9), upper -1.5 dB at 1.2 kHz (Q 1), interstage LP 5.5 kHz, sag 0.8.
- **Cab and output.**
  - Each channel has its own 4x12: HP 55 Hz, +6 dB at 100 Hz (Q 0.9), -2.5 dB at 500 Hz (Q 0.9), +2 dB at 1.8 kHz (Q 1.4), -4 dB at 4 kHz (Q 2), LP 5 kHz (Q 0.75).
  - Output is Level × 0.47 trim, then `tanh` × 0.98.
- **Controls.**
  - Knobs: Gain (default 0.7; 14 + 30·knob dB), Tone (default 0.4), Level (square law, unity at 0.75; +2.5 dB in lead). Gain, level and tone are smoothed over 20 ms.
  - Press: Riff ↔ Lead (crossfaded over 15 ms). The patch starts in Riff, Muff off.
  - Hold: Muff on/off.
  - LED: Riff beige / dark red, Lead light green / dark lime (normal / Muff). Red means the working buffer is missing.
- **Size.** No delay lines or buffers. The `.endl` is ~9 KB.
- **Host audition.** `cargo sim` renders an *original* doom riff at 52 bpm in C standard (not the actual *Dopesmoker* riff) on the modelled guitar, at ×0.6 input trim, and writes `target/render/sleep_demo.wav` (~63 s). It prints per-section peak, RMS, side level and L/R correlation, and fails on NaN or full-scale output.
  - Power chords are root, fifth and octave, T60 8 s (sustained in front of the amps), downstrokes 14 ms per string. A chug is the palm-muted low C and its fifth. Timing is nudged by up to ±8 ms, repeatably.
  - Bar A: C5 1.5 beats, C chug 0.5, D#5 0.75, F5 0.5, F#5 0.25, F5 0.5. Bar B: D#5 1 (slid from 2 semitones below), C5 1, G5 1.5 (slid from 1 below), F#5 0.25, F5 0.25. End bar: C5 for 4 beats.
  - Form: end bar as an intro; A B A B; hold (Muff) for A B; hold off and press (Lead) for a pentatonic line (G3 slide+vib, A#3 bent +200 cents, G3, F3, D#3, C3 with vibrato, C4 slid from 3 below, D#4 bent +100, C4, A#3, G3 bent +200 with vibrato, F3); press back to Riff for A and the end bar; a 3 s tail.
