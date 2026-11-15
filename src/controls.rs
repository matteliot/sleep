//! The knobs: one table the patch and the firmware entry points both read.

use crate::pedalkit::abi::PARAM_COUNT;

pub struct Param {
    pub name: &'static str,
    pub default: f32,
}

/// Knob indices (the firmware's parameter ids).
pub const GAIN: usize = 0;
pub const TONE: usize = 1;
pub const LEVEL: usize = 2;

pub const PARAMS: [Param; 3] = [
    // Preamp gain, ~14 to 44 dB.
    Param { name: "Gain", default: 0.7 },
    // Amp treble: CCW dark and woolly, CW cutting.
    Param { name: "Tone", default: 0.4 },
    // Output; unity at 3 o'clock.
    Param { name: "Level", default: 0.75 },
];

const _: () = assert!(PARAMS.len() == PARAM_COUNT as usize);

/// Every knob at its default.
pub const fn defaults() -> [f32; 3] {
    [PARAMS[0].default, PARAMS[1].default, PARAMS[2].default]
}
