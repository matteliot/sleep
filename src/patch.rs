//! The whole patch: Muff -> wah -> one guitar into two stacks at once, one per
//! side (no gate: the wall hums and rings out) -> a 4x12 per side -> level.

use crate::controls::{self, GAIN, LEVEL, TONE};
use crate::pedalkit::abi::Color;
use crate::pedalkit::amp::Amp;
use crate::pedalkit::cab::Cab;
use crate::pedalkit::dsp::{db, tanh};
use crate::pedalkit::helpers::Smooth;
use crate::pedalkit::muff::Muff;
use crate::pedalkit::wah::Wah;
use crate::presets::{CAB_4X12, MATAMP, MUFF, ORANGE, WAH};

/// Preamp gain (dB) at knob 0, and the extra at knob 1.
const GAIN_MIN_DB: f32 = 14.0;
const GAIN_RANGE_DB: f32 = 30.0;
/// Lead: a little more level (dB) so the solo sits on top.
const LEAD_LEVEL_DB: f32 = 2.5;
/// Output trim: unity Level puts the riff around -14 dB RMS.
const OUT_TRIM: f32 = 0.47;
/// Knob smoothing (s).
const KNOB_GLIDE: f32 = 0.02;

pub struct Patch {
    ready: bool,
    error: bool,
    knobs: [f32; 3],
    lead: bool,
    muff_on: bool,
    gain: Smooth,
    out: Smooth,
    tone: Smooth,
    muff: Muff,
    wah: Wah,
    amp_l: Amp,
    amp_r: Amp,
    cab_l: Cab,
    cab_r: Cab,
}

impl Patch {
    pub const NEW: Self = Self {
        ready: false,
        error: false,
        knobs: controls::defaults(),
        lead: false,
        muff_on: false,
        gain: Smooth::new(1.0),
        out: Smooth::new(0.0),
        tone: Smooth::new(controls::PARAMS[TONE].default),
        muff: Muff::NEW,
        wah: Wah::NEW,
        amp_l: Amp::NEW,
        amp_r: Amp::NEW,
        cab_l: Cab::NEW,
        cab_r: Cab::NEW,
    };

    pub fn init(&mut self) {
        self.amp_l.init(&ORANGE);
        self.amp_r.init(&MATAMP);
        self.muff.init(&MUFF);
        self.wah.init(&WAH);
        self.cab_l.init(&CAB_4X12);
        self.cab_r.init(&CAB_4X12);
        for s in [&mut self.gain, &mut self.out, &mut self.tone] {
            s.time(KNOB_GLIDE);
        }
        self.retarget();
        self.gain.snap();
        self.out.snap();
        self.tone.snap();
        self.muff.snap();
        self.wah.snap();
        self.ready = true;
    }

    pub fn ready(&self) -> bool {
        self.ready
    }

    /// The firmware didn't provide the working buffer (shown on the LED).
    pub fn set_error(&mut self) {
        self.error = true;
    }

    /// Sets knob `idx` to `v` (0..1); false if there's no such knob.
    pub fn set_knob(&mut self, idx: usize, v: f32) -> bool {
        let Some(k) = self.knobs.get_mut(idx) else { return false };
        *k = v.clamp(0.0, 1.0);
        self.retarget();
        true
    }

    /// Footswitch press: Riff <-> Lead.
    pub fn toggle_lead(&mut self) {
        self.lead = !self.lead;
        self.retarget();
    }

    /// Footswitch hold: Muff on/off.
    pub fn toggle_muff(&mut self) {
        self.muff_on = !self.muff_on;
        self.retarget();
    }

    /// Points the smoothed values at the knobs and switches. Safe before
    /// `init`, which re-applies them and snaps.
    fn retarget(&mut self) {
        let lead = if self.lead { 1.0 } else { 0.0 };
        self.gain.set(db(GAIN_MIN_DB + GAIN_RANGE_DB * self.knobs[GAIN]));
        // Square law: 0 at minimum, unity at 3 o'clock, +5 dB at maximum.
        let v = self.knobs[LEVEL] / 0.75;
        self.out.set(v * v * OUT_TRIM * db(LEAD_LEVEL_DB * lead));
        self.tone.set(self.knobs[TONE]);
        self.wah.on(self.lead);
        self.muff.on(self.muff_on);
    }

    #[inline]
    pub fn tick(&mut self, x: f32) -> (f32, f32) {
        let g = self.gain.tick();
        let out = self.out.tick();
        let t = self.tone.tick();
        let x = self.muff.process(x);
        let x = self.wah.process(x);
        // One guitar into both stacks at once.
        let l = self.cab_l.process(self.amp_l.process(x, g, t));
        let r = self.cab_r.process(self.amp_r.process(x, g, t));
        (limit(l * out), limit(r * out))
    }

    pub fn led(&self) -> Color {
        if self.error {
            return Color::Red;
        }
        match (self.lead, self.muff_on) {
            (false, false) => Color::Beige,
            (true, false) => Color::LightGreen,
            (false, true) => Color::DarkRed,
            (true, true) => Color::DarkLime,
        }
    }
}

/// Output soft limiter, never reaching ±1.
#[inline]
fn limit(x: f32) -> f32 {
    tanh(x) * 0.98
}
