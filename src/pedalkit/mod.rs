//! pedalkit: reusable building blocks for Polyend Endless patches.
//!
//! Nothing in here knows about a particular pedal: every effect takes a
//! settings struct, and each patch keeps its own presets. Modules only refer
//! to each other through relative paths (`super::`), so this directory can be
//! lifted out into its own crate unchanged.
//!
//!   abi       the Endless patch ABI (FxPatchSDK 0x000B)
//!   global    a static cell for state shared with the firmware entry points
//!   dsp       f32-only math, biquads, one-pole, delay lines
//!   helpers   Smooth (glide), Peak (EQ band), Clip2x (oversampled clipper)
//!   amp       a valve head: Preamp -> ToneStack -> PowerAmp, set by a Voicing
//!   muff      Big Muff-style fuzz, crossfaded in
//!   wah       a wah: parked or swept, faded in
//!   cab       a speaker cabinet EQ
//!   guitar    (host only) a modelled electric guitar for demo renders

// A kit: each patch uses only some of it.
#![allow(dead_code)]

pub mod abi;
pub mod amp;
pub mod cab;
pub mod dsp;
pub mod global;
pub mod helpers;
pub mod muff;
pub mod wah;

#[cfg(not(target_os = "none"))]
pub mod guitar;
