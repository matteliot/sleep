//! sleep: a Polyend Endless patch for the most Matt Pike guitar sound ever, the
//! tone of Sleep's *Dopesmoker*: a Les Paul tuned way down into cranked valve
//! stacks, a slow, thick, mid-forward wall of amp distortion.
//!
//! Signal flow (`patch`, built from `pedalkit` blocks set by `presets`):
//!   input -> Muff (crossfaded in) -> wah (lead only) -> one guitar into two
//!   full stacks at once, one per side (no gate: the wall hums and rings out):
//!     Left   "Orange" voicing   } biased triode preamp, second gain stage,
//!     Right  "Matamp" voicing   }  tone stack, sagging power amp
//!   -> a 4x12 per side -> output level.
//!
//! Controls (`Patch::PARAMS`; entry points from `pedalkit::endless_patch!`)
//!   Left knob   Gain   (preamp gain, ~14 to 44 dB)
//!   Mid knob    Tone   (amp treble: CCW dark and woolly, CW cutting)
//!   Right knob  Level  (output; unity at 3 o'clock)
//!   Footswitch press  Riff <-> Lead (a half-cocked wah into the stacks, a little louder)
//!   Footswitch hold   Muff on/off (a Big Muff-style fuzz stacked in front of the amps)
//!   LED  Riff: beige | Lead: light green | with Muff: dark red | dark lime
//!
//! Host builds (`cargo sim`) render a demo riff through the same code; see `host`.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

/// Reusable blocks (effects, DSP, the Endless ABI); see its docs.
mod pedalkit;

/// The pedal itself.
mod sleep;

endless_patch!(kit = crate::pedalkit, pedal = crate::sleep::Patch);

#[cfg(not(target_os = "none"))]
mod host;

#[cfg(not(target_os = "none"))]
fn main() {
    host::main();
}
