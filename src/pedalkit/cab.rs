//! A speaker cabinet as EQ: a high-pass (the cab's low limit), four bands
//! (resonance, dips, presence) and a low-pass (the speakers' top-end roll-off).

use super::dsp::Biquad;
use super::helpers::Peak;

pub struct CabSettings {
    pub hp_hz: f32,
    pub bands: [Peak; 4],
    /// Low-pass corner (Hz) and Q.
    pub lp_hz: f32,
    pub lp_q: f32,
}

pub struct Cab {
    eq: [Biquad; 6],
}

impl Cab {
    pub const NEW: Self = Self { eq: [Biquad::NEW; 6] };

    pub fn init(&mut self, s: &CabSettings) {
        self.eq[0].highpass(s.hp_hz, 0.707);
        for (b, p) in self.eq[1..5].iter_mut().zip(s.bands.iter()) {
            p.apply(b);
        }
        self.eq[5].lowpass(s.lp_hz, s.lp_q);
    }

    #[inline]
    pub fn process(&mut self, mut x: f32) -> f32 {
        for b in self.eq.iter_mut() {
            x = b.process(x);
        }
        x
    }
}
