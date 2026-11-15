# sleep

A [Polyend Endless](https://polyend.com/endless/) patch for the most Matt Pike guitar sound ever: Sleep's *Dopesmoker*.
That means a Les Paul tuned way down into cranked valve stacks, making a slow, thick, mid-forward wall of amp distortion that never gets scooped or fizzy.
It's written in `no_std` Rust against the patch ABI of Polyend's [FxPatchSDK](https://github.com/polyend/FxPatchSDK) (ABI 0x000B), so no ARM GCC is needed.

The guitar (left jack) goes into **two full stacks at once**, one per side, like running one guitar into several heads. There's no gate: the wall hums and rings out.

- **Left: "Orange" voicing.** A biased triode preamp (asymmetric, warm clipping) into a second gain stage, a mid-forward tone stack and a power amp that **sags** on hard hits, so chords compress
  and bloom. Power-amp resonance brings the low end back after the clipping.
- **Right: "Matamp" voicing.** The same circuit with fatter low-mids and more sag.

Each stack goes through its own 4x12 cab sim, voiced low for C tuning.

| Control | Function |
|---|---|
| Left knob | **Gain**: preamp gain, about 14 to 44 dB |
| Mid knob | **Tone**: amp treble. CCW = dark and woolly, CW = cutting |
| Right knob | **Level**: output, unity at 3 o'clock |
| Footswitch press | **Riff ↔ Lead**. Lead parks a wah half-cocked in front of the amps (a +9 dB resonant peak at 1 kHz) and adds +2.5 dB level |
| Footswitch hold | **Muff**: a Big Muff-style fuzz stacked in front of the amps, for an even thicker wall |

| LED | Normal | Muff |
|---|---|---|
| Riff | beige | dark red |
| Lead | light green | dark lime |

Red means the pedal didn't provide the patch's working buffer.

## Install

Download [`release/sleep.endl`](release/sleep.endl) (on GitHub: open it and click the download button), connect the Endless to your computer over USB and copy the file onto the pedal. Nothing needs to be
built. To rebuild it from source, see below.

## Build

```sh
rustup target add thumbv7em-none-eabihf --toolchain nightly
rustup component add llvm-tools --toolchain nightly
cargo build --release        # ELF for the pedal (default target is thumbv7em-none-eabihf)
./build-endl.sh              # build + make target/endless/sleep.endl, check f64 use and size
./build-endl.sh --deploy     # ...and copy to /Volumes/ENDLESS
```

## Audition on your computer

```sh
cargo sim                    # -> target/render/sleep_demo.wav
```

This renders an original slow doom riff at 52 bpm in C standard through the same patch code. It's not the actual *Dopesmoker* riff: it uses ringing power chords, palm-muted low-C chugs, a bluesy
flat-five walk and slides. It's played on a modelled electric guitar (`src/pedalkit/guitar.rs`: exact tuning, pick and humbucker, palm mutes, strums, bends, vibrato and human timing). It goes intro chord →
riff → riff + Muff → lead → riff, then prints the level and stereo width of each section. A render is in [`samples/sleep_demo.wav`](samples/sleep_demo.wav).

## Code layout

- `src/sleep/`: the pedal. `patch.rs` wires it up, `presets.rs` holds every number, and the other files are its blocks.
- `src/pedalkit/`: shared building blocks (effects, DSP, the Endless ABI, the entry points, the demo renderer). It's a copy of the master in [superPedal](https://github.com/matteliot/superPedal), which also carries this pedal, so don't edit it here.
- `src/host/`: the `cargo sim` demo (`score.rs` is what it plays).
