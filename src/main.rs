//! sleep: a Polyend Endless patch for the most Matt Pike guitar sound ever, the
//! tone of Sleep's *Dopesmoker*: a Les Paul tuned way down into cranked valve
//! stacks, a slow, thick, mid-forward wall of fuzz-like amp distortion.
//!
//! Signal flow:
//!   input -> gate -> [Muff fuzz, crossfaded in] -> two full stacks in stereo:
//!     Left   "Orange" voicing: biased triode preamp, second gain stage,
//!            mid-forward tone stack, sagging power amp
//!     Right  "Matamp" voicing: the same circuit with a fatter low-mid voice,
//!            fed a second "take" ~23 ms late and drifting (double-tracked)
//!   -> lead voice (mid push + half-cocked wah peak + echo, crossfaded in)
//!   -> a 4x12 cab sim per channel -> output level.
//!
//! Controls
//!   Left knob   Gain   (preamp gain, ~14 to 44 dB)
//!   Mid knob    Tone   (amp treble: CCW dark and woolly, CW cutting)
//!   Right knob  Level  (output; unity at 3 o'clock)
//!   Footswitch press  Riff <-> Lead (more gain, mid push, wah peak, echo)
//!   Footswitch hold   Muff on/off (a Big Muff-style fuzz stacked in front of the amps)
//!   LED  Riff: beige | Lead: light green | with Muff: dark red | dark lime
//!
//! Host builds (`cargo sim`) render a demo riff through the same code; see `host`.

#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

#[cfg_attr(not(target_os = "none"), allow(dead_code))]
mod abi;
#[allow(dead_code)]
mod dsp;

use core::cell::UnsafeCell;
use core::f32::consts::TAU;

use abi::*;
use dsp::*;

// ---------------------------------------------------------------- voicing

/// Preamp gain (dB) at knob 0, and the extra at knob 1.
const GAIN_MIN_DB: f32 = 14.0;
const GAIN_RANGE_DB: f32 = 30.0;
/// Second preamp stage and power-amp drive (dB).
const STAGE2_DB: f32 = 8.0;
const POWER_DB: f32 = 6.0;
/// Power-amp resonance (post-clipping low boost): frequency (Hz) and gain (dB).
const RESO_HZ: f32 = 120.0;
const RESO_DB: f32 = 5.0;
/// Lead voice: extra gain, mid push (800 Hz), half-cocked wah peak (1.1 kHz)
/// and level (dB), and its echo mix.
const LEAD_GAIN_DB: f32 = 6.0;
const LEAD_MID_DB: f32 = 4.0;
const LEAD_WAH_DB: f32 = 6.0;
const LEAD_LEVEL_DB: f32 = 2.0;
const LEAD_ECHO_MIX: f32 = 0.22;

/// Echo (lead only): ~375 ms with dark, thinning repeats.
const ECHO_TIME: f32 = 18_000.0;
const ECHO_SPREAD: f32 = 900.0;
const ECHO_FEEDBACK: f32 = 0.38;

/// Double-track: the right stack hears a second "take", this late (samples,
/// ~23 ms) and drifting with two slow LFOs (Hz, samples of drift).
const DOUBLE_DELAY: f32 = 1100.0;
const DOUBLE_DRIFT: f32 = 25.0;
const DOUBLE_RATE_A: f32 = 0.23;
const DOUBLE_RATE_B: f32 = 0.41;

/// Muff: gain into its two clipping stages (dB) and its output trim, so the
/// fuzz drives the amps a few dB harder than the guitar alone.
const MUFF_GAIN_DB: f32 = 28.0;
const MUFF_OUT: f32 = 0.35;

/// Noise gate on the input envelope (linear), with hysteresis. Kept low:
/// sustain is the point, and notes are left to bloom and ring out.
const GATE_OPEN: f32 = 0.0008;
const GATE_CLOSE: f32 = 0.0004;

/// Output trim: unity Level puts the riff around -14 dB RMS.
const OUT_TRIM: f32 = 0.47;

/// One stack's circuit values.
struct Voicing {
    /// Gain offset into the preamp (dB).
    trim_db: f32,
    /// Triode bias: asymmetric clipping, so even harmonics.
    bias: f32,
    bass: (f32, f32, f32),
    mid: (f32, f32, f32),
    upper: (f32, f32, f32),
    /// Interstage low-pass (Hz).
    inter_lp: f32,
    /// Power supply sag: how far the power amp's gain drops when hit hard.
    sag: f32,
}

/// Orange-style: big lows, forward mids around 650 Hz, a bit of bite.
const ORANGE: Voicing = Voicing {
    trim_db: 0.0,
    bias: 0.25,
    bass: (100.0, 0.8, 4.0),
    mid: (650.0, 0.7, 1.5),
    upper: (3000.0, 1.0, 1.5),
    inter_lp: 6500.0,
    sag: 0.6,
};

/// Matamp-style: fatter low-mids, a little softer up top and more sag.
const MATAMP: Voicing = Voicing {
    trim_db: -2.0,
    bias: 0.15,
    bass: (80.0, 0.8, 3.0),
    mid: (250.0, 0.9, 3.0),
    upper: (1200.0, 1.0, -1.5),
    inter_lp: 5500.0,
    sag: 0.8,
};

// ---------------------------------------------------------------- one stack

/// A cranked valve head: biased triode -> interstage filter -> second stage
/// -> tone stack (+ lead peaks) -> sagging power amp. 2x oversampled clippers.
struct Amp {
    trim: f32,
    bias: f32,
    bias_off: f32,
    sag_k: f32,
    hp_in: Biquad,
    dc: Biquad,
    inter: Biquad,
    bass: Biquad,
    mid: Biquad,
    upper: Biquad,
    tre_split: OnePole,
    lead_mid: Biquad,
    wah: Biquad,
    reso: Biquad,
    u1: f32,
    u2: f32,
    u3: f32,
    sag_env: f32,
}

impl Amp {
    const NEW: Self = Self {
        trim: 1.0,
        bias: 0.0,
        bias_off: 0.0,
        sag_k: 0.0,
        hp_in: Biquad::NEW,
        dc: Biquad::NEW,
        inter: Biquad::NEW,
        bass: Biquad::NEW,
        mid: Biquad::NEW,
        upper: Biquad::NEW,
        tre_split: OnePole::NEW,
        lead_mid: Biquad::NEW,
        wah: Biquad::NEW,
        reso: Biquad::NEW,
        u1: 0.0,
        u2: 0.0,
        u3: 0.0,
        sag_env: 0.0,
    };

    fn init(&mut self, v: &Voicing) {
        self.trim = db(v.trim_db);
        self.bias = v.bias;
        self.bias_off = tanh(v.bias);
        self.sag_k = v.sag;
        self.hp_in.highpass(35.0, 0.707); // keeps the low C, drops rumble
        self.dc.highpass(30.0, 0.707);
        self.inter.lowpass(v.inter_lp, 0.707);
        self.bass.peak(v.bass.0, v.bass.1, v.bass.2);
        self.mid.peak(v.mid.0, v.mid.1, v.mid.2);
        self.upper.peak(v.upper.0, v.upper.1, v.upper.2);
        self.tre_split.set(2000.0);
        self.lead_mid.bandpass(800.0, 0.8);
        self.wah.bandpass(1100.0, 2.5);
        // Power-amp resonance: low end restored after the clipping, where it can't turn to mud.
        self.reso.peak(RESO_HZ, 0.8, RESO_DB);
    }

    #[inline]
    fn triode(&self, u: f32) -> f32 {
        tanh(u + self.bias) - self.bias_off
    }

    /// `g` preamp gain, `tone` 0..1, `lead` 0..1 (smoothed).
    #[inline]
    fn process(&mut self, x: f32, g: f32, tone: f32, lead: f32, c_att: f32, c_rel: f32) -> f32 {
        let u = self.hp_in.process(x) * g * self.trim;
        let y = 0.5 * (self.triode(0.5 * (u + self.u1)) + self.triode(u));
        self.u1 = u;
        let y = self.inter.process(self.dc.process(y));
        let u = y * db(STAGE2_DB);
        let y = 0.5 * (tanh(0.5 * (u + self.u2)) + tanh(u));
        self.u2 = u;
        // Tone stack, then the amp's treble (Tone knob) on the content above 2 kHz.
        let y = self.upper.process(self.mid.process(self.bass.process(y)));
        let lo = self.tre_split.process(y);
        let y = lo + (0.25 + 1.1 * tone) * (y - lo);
        // Lead: parallel band-passes, so the fade in is click-free.
        let y = y
            + lead * ((db(LEAD_MID_DB) - 1.0) * self.lead_mid.process(y) + (db(LEAD_WAH_DB) - 1.0) * self.wah.process(y));
        // Power amp: the supply sags on hard hits (compression and bloom).
        let a = y.abs();
        self.sag_env += (a - self.sag_env) * if a > self.sag_env { c_att } else { c_rel };
        let u = y * db(POWER_DB) / (1.0 + self.sag_k * self.sag_env);
        let p = |u: f32| tanh(u + 0.1) - tanh(0.1);
        let y = 0.5 * (p(0.5 * (u + self.u3)) + p(u));
        self.u3 = u;
        self.reso.process(y) * 0.6
    }
}

// ---------------------------------------------------------------- state

/// The delay lines. They are all zero at rest, so they live in .bss and add
/// nothing to the patch file; `patch_init` zeroes them before use.
struct Bufs {
    dbl_line: Delay<2048>,
    echo_line: Delay<32768>,
}

impl Bufs {
    const NEW: Self = Self { dbl_line: Delay::NEW, echo_line: Delay::NEW };
}

struct Sl {
    ready: bool,
    error: bool,
    // knobs, 0..1
    gain: f32,
    tone: f32,
    level: f32,
    lead: bool,
    muff: bool,
    // smoothed values and targets
    g: f32,
    g_t: f32,
    out_g: f32,
    out_t: f32,
    tone_s: f32,
    lead_s: f32,
    muff_s: f32,
    // the stacks
    amp_l: Amp,
    amp_r: Amp,
    // muff
    muff_hp: Biquad,
    muff_lp1: OnePole,
    muff_lp2: OnePole,
    muff_lo: OnePole,
    muff_hi: OnePole,
    // echo loop
    echo_lp: OnePole,
    echo_hp: Biquad,
    // double-track second take
    dbl_lp: Biquad,
    cab: [[Biquad; 6]; 2],
    // dynamics
    env: f32,
    gate: f32,
    gate_open: bool,
    // modulation
    dbl_ph_a: f32,
    dbl_ph_b: f32,
    // coefficients
    c_param: f32,
    c_mix: f32,
    c_att: f32,
    c_rel: f32,
    c_gate_open: f32,
    c_gate_close: f32,
    c_sag_att: f32,
    c_sag_rel: f32,
}

impl Sl {
    const NEW: Self = Self {
        ready: false,
        error: false,
        gain: 0.7,
        tone: 0.4,
        level: 0.75,
        lead: false,
        muff: false,
        g: 1.0,
        g_t: 1.0,
        out_g: 0.0,
        out_t: 0.0,
        tone_s: 0.4,
        lead_s: 0.0,
        muff_s: 0.0,
        amp_l: Amp::NEW,
        amp_r: Amp::NEW,
        muff_hp: Biquad::NEW,
        muff_lp1: OnePole::NEW,
        muff_lp2: OnePole::NEW,
        muff_lo: OnePole::NEW,
        muff_hi: OnePole::NEW,
        echo_lp: OnePole::NEW,
        echo_hp: Biquad::NEW,
        dbl_lp: Biquad::NEW,
        cab: [[Biquad::NEW; 6]; 2],
        env: 0.0,
        gate: 0.0,
        gate_open: false,
        dbl_ph_a: 0.0,
        dbl_ph_b: 0.25,
        c_param: 1.0,
        c_mix: 1.0,
        c_att: 1.0,
        c_rel: 1.0,
        c_gate_open: 1.0,
        c_gate_close: 1.0,
        c_sag_att: 1.0,
        c_sag_rel: 1.0,
    };

    fn init(&mut self) {
        self.amp_l.init(&ORANGE);
        self.amp_r.init(&MATAMP);
        // Muff: two soft clippers with low-passes between, then its tone
        // blend (400 Hz low-pass against a 1.2 kHz high-pass).
        self.muff_hp.highpass(60.0, 0.707);
        self.muff_lp1.set(5000.0);
        self.muff_lp2.set(4000.0);
        self.muff_lo.set(400.0);
        self.muff_hi.set(1200.0);
        self.echo_lp.set(2800.0);
        self.echo_hp.highpass(150.0, 0.707);
        self.dbl_lp.lowpass(7500.0, 0.707);
        // 4x12, voiced low for the tuning: a big low resonance, a mid dip,
        // speaker presence and a steep top-end roll-off.
        for cab in self.cab.iter_mut() {
            cab[0].highpass(55.0, 0.707);
            cab[1].peak(100.0, 0.9, 6.0);
            cab[2].peak(500.0, 0.9, -2.5);
            cab[3].peak(1800.0, 1.4, 2.0);
            cab[4].peak(4000.0, 2.0, -4.0);
            cab[5].lowpass(5000.0, 0.75);
        }
        self.c_param = coef(0.02);
        self.c_mix = coef(0.015);
        self.c_att = coef(0.001);
        self.c_rel = coef(0.12);
        self.c_gate_open = coef(0.002);
        self.c_gate_close = coef(0.08);
        self.c_sag_att = coef(0.015);
        self.c_sag_rel = coef(0.2);
        self.retarget();
        self.g = self.g_t;
        self.out_g = self.out_t;
        self.tone_s = self.tone;
        self.lead_s = if self.lead { 1.0 } else { 0.0 };
        self.muff_s = if self.muff { 1.0 } else { 0.0 };
        self.ready = true;
    }

    fn retarget(&mut self) {
        let lead = if self.lead { 1.0 } else { 0.0 };
        self.g_t = db(GAIN_MIN_DB + GAIN_RANGE_DB * self.gain + LEAD_GAIN_DB * lead);
        // Square law: 0 at minimum, unity at 3 o'clock, +5 dB at maximum.
        let v = self.level / 0.75;
        self.out_t = v * v * OUT_TRIM * db(LEAD_LEVEL_DB * lead);
    }

    /// Big Muff-style fuzz (mono), in front of the amps.
    #[inline]
    fn muff(&mut self, x: f32) -> f32 {
        let y = tanh(self.muff_hp.process(x) * db(MUFF_GAIN_DB));
        let y = self.muff_lp1.process(y);
        let y = tanh(y * 4.0);
        let y = self.muff_lp2.process(y);
        let lo = self.muff_lo.process(y);
        let hi = y - self.muff_hi.process(y);
        (0.6 * lo + 0.5 * hi) * MUFF_OUT
    }

    /// The second take's input: late and drifting.
    #[inline]
    fn double(&mut self, bf: &mut Bufs, x: f32) -> f32 {
        bf.dbl_line.push(x);
        self.dbl_ph_a += DOUBLE_RATE_A / SR;
        self.dbl_ph_b += DOUBLE_RATE_B / SR;
        if self.dbl_ph_a >= 1.0 {
            self.dbl_ph_a -= 1.0;
        }
        if self.dbl_ph_b >= 1.0 {
            self.dbl_ph_b -= 1.0;
        }
        let drift = 0.6 * sin(TAU * self.dbl_ph_a) + 0.4 * sin(TAU * self.dbl_ph_b);
        bf.dbl_line.read(DOUBLE_DELAY + DOUBLE_DRIFT * drift)
    }

    #[inline]
    fn tick(&mut self, bf: &mut Bufs, x: f32) -> (f32, f32) {
        self.g += (self.g_t - self.g) * self.c_param;
        self.out_g += (self.out_t - self.out_g) * self.c_param;
        self.tone_s += (self.tone - self.tone_s) * self.c_param;
        let lead_t = if self.lead { 1.0 } else { 0.0 };
        let muff_t = if self.muff { 1.0 } else { 0.0 };
        self.lead_s += (lead_t - self.lead_s) * self.c_mix;
        self.muff_s += (muff_t - self.muff_s) * self.c_mix;

        let a = x.abs();
        self.env += (a - self.env) * if a > self.env { self.c_att } else { self.c_rel };
        if self.env > GATE_OPEN {
            self.gate_open = true;
        } else if self.env < GATE_CLOSE {
            self.gate_open = false;
        }
        let (gt, gc) = if self.gate_open { (1.0, self.c_gate_open) } else { (0.0, self.c_gate_close) };
        self.gate += (gt - self.gate) * gc;
        let x = x * self.gate;

        // The Muff always runs, so switching it is a click-free crossfade.
        let f = self.muff(x);
        let x = x + (f - x) * self.muff_s;

        let xr = self.double(bf, x);
        let (g, t, ld) = (self.g, self.tone_s, self.lead_s);
        let mut l = self.amp_l.process(x, g, t, ld, self.c_sag_att, self.c_sag_rel);
        let mut r = self.amp_r.process(xr, g, t, ld, self.c_sag_att, self.c_sag_rel);
        r = self.dbl_lp.process(r);

        // Lead echo: one line, two taps for width, dark repeats.
        let el = bf.echo_line.read(ECHO_TIME);
        let er = bf.echo_line.read(ECHO_TIME + ECHO_SPREAD);
        let fb = self.echo_hp.process(self.echo_lp.process(el));
        bf.echo_line.push(0.5 * (l + r) * self.lead_s + ECHO_FEEDBACK * fb);
        let em = LEAD_ECHO_MIX * self.lead_s;
        l += em * el;
        r += em * er;

        for b in self.cab[0].iter_mut() {
            l = b.process(l);
        }
        for b in self.cab[1].iter_mut() {
            r = b.process(r);
        }
        (limit(l * self.out_g), limit(r * self.out_g))
    }

    fn led(&self) -> Color {
        if self.error {
            return Color::Red;
        }
        match (self.lead, self.muff) {
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

/// All patch state. The firmware calls every entry point from the audio
/// thread (SDK: "no need to synchronize").
struct Global<T>(UnsafeCell<T>);
unsafe impl<T> Sync for Global<T> {}
static STATE: Global<Sl> = Global(UnsafeCell::new(Sl::NEW));
static BUFS: Global<Bufs> = Global(UnsafeCell::new(Bufs::NEW));

#[allow(clippy::mut_from_ref)]
fn state() -> &'static mut Sl {
    // SAFETY: single-threaded access from the firmware's audio thread.
    unsafe { &mut *STATE.0.get() }
}

#[allow(clippy::mut_from_ref)]
fn bufs() -> &'static mut Bufs {
    // SAFETY: as `state`.
    unsafe { &mut *BUFS.0.get() }
}

/// Zeroes the delay lines in place. The loader should already have cleared
/// .bss, but nothing promises it, and a NaN left in a feedback loop never
/// decays. (In place: a zeroed copy would be a stack temporary.)
fn clear_bufs() {
    // SAFETY: `Bufs` is only f32 and usize fields, so all-zero bits are a
    // valid value; single-threaded as `state`.
    unsafe { core::ptr::write_bytes(BUFS.0.get(), 0, 1) }
}

// ---------------------------------------------------------------- entry points

const PARAM_GAIN: i32 = 0;
const PARAM_TONE: i32 = 1;
const PARAM_LEVEL: i32 = 2;

unsafe extern "C" fn patch_init(env: *const PatchEnv) {
    if env.is_null() || unsafe { (*env).abi_version } != PATCH_ABI_VERSION as u32 {
        return;
    }
    clear_bufs();
    state().init();
}

unsafe extern "C" fn update_buffers(_: *const PatchEnv, left: *mut f32, right: *mut f32, samples: i32) -> f32 {
    let Ok(n) = usize::try_from(samples) else { return 0.0 };
    if left.is_null() || right.is_null() || n == 0 {
        return 0.0;
    }
    // SAFETY: the firmware passes two buffers of `samples` floats.
    let (l, r) = unsafe { (core::slice::from_raw_parts_mut(left, n), core::slice::from_raw_parts_mut(right, n)) };
    let (s, bf) = (state(), bufs());
    if !s.ready {
        return 0.0;
    }
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        let (ol, or) = s.tick(bf, *a);
        *a = ol;
        *b = or;
    }
    0.0
}

unsafe extern "C" fn set_buffer(_: *const PatchEnv, buffer: *mut f32) {
    // All state is static; the working buffer isn't needed.
    if buffer.is_null() {
        state().error = true;
    }
}

unsafe extern "C" fn get_buffer_size(_: *const PatchEnv) -> i32 {
    WORKING_BUFFER_SIZE
}

unsafe extern "C" fn get_param_min(_: *const PatchEnv, _: i32) -> f32 {
    0.0
}

unsafe extern "C" fn get_param_max(_: *const PatchEnv, _: i32) -> f32 {
    1.0
}

unsafe extern "C" fn get_param_default(_: *const PatchEnv, idx: i32) -> f32 {
    match idx {
        PARAM_GAIN => 0.7,
        PARAM_TONE => 0.4,
        _ => 0.75,
    }
}

unsafe extern "C" fn is_param_enabled(_: *const PatchEnv, idx: i32, source_id: i32) -> u8 {
    // Knobs only (source 0), like the SDK wrapper.
    (source_id == 0 && (0..PARAM_COUNT).contains(&idx)) as u8
}

unsafe fn write_c_str(s: &str, out: *mut u8, len: usize) {
    if out.is_null() || len == 0 {
        return;
    }
    let n = s.len().min(len - 1);
    // SAFETY: the firmware provides `len` writable bytes at `out`.
    unsafe {
        core::ptr::copy_nonoverlapping(s.as_ptr(), out, n);
        *out.add(n) = 0;
    }
}

unsafe extern "C" fn get_param_name(_: *const PatchEnv, idx: i32, out: *mut u8, len: usize) {
    let name = match idx {
        PARAM_GAIN => "Gain",
        PARAM_TONE => "Tone",
        PARAM_LEVEL => "Level",
        _ => "",
    };
    unsafe { write_c_str(name, out, len) }
}

unsafe extern "C" fn get_param_unit(_: *const PatchEnv, _: i32, out: *mut u8, len: usize) {
    unsafe { write_c_str("", out, len) }
}

unsafe extern "C" fn set_param(_: *const PatchEnv, idx: i32, value: f32) {
    let s = state();
    let v = value.clamp(0.0, 1.0);
    match idx {
        PARAM_GAIN => s.gain = v,
        PARAM_TONE => s.tone = v,
        PARAM_LEVEL => s.level = v,
        _ => return,
    }
    if s.ready {
        s.retarget();
    }
}

unsafe extern "C" fn special_action(_: *const PatchEnv, idx: i32) {
    let s = state();
    match idx {
        ACTION_FOOTSWITCH_PRESS => s.lead = !s.lead,
        ACTION_FOOTSWITCH_HOLD => s.muff = !s.muff,
        _ => return,
    }
    if s.ready {
        s.retarget();
    }
}

unsafe extern "C" fn get_state_idx(_: *const PatchEnv) -> i32 {
    state().led() as i32
}

// ---------------------------------------------------------------- header

#[cfg(target_os = "none")]
unsafe extern "C" {
    static __patch_bss_start: u8;
    static patch_image_size: u8;
    static patch_bss_size: u8;
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".patch_header")]
#[used]
pub static patch_header: PatchHeader = PatchHeader {
    magic: PATCH_MAGIC,
    abi_version: PATCH_ABI_VERSION,
    flags: PATCH_FLAG_NONE,
    init: patch_init,
    agent_update_buffers: update_buffers,
    agent_set_buffer: set_buffer,
    agent_get_buffer_size: get_buffer_size,
    agent_get_param_min: get_param_min,
    agent_get_param_max: get_param_max,
    agent_get_param_default: get_param_default,
    agent_is_param_enabled: is_param_enabled,
    agent_get_param_name: get_param_name,
    agent_get_param_unit: get_param_unit,
    agent_set_param: set_param,
    agent_special_action: special_action,
    agent_get_state_idx: get_state_idx,
    image_size: &raw const patch_image_size,
    bss_begin: &raw const __patch_bss_start,
    bss_size: &raw const patch_bss_size,
    reserved: [0; 16],
};

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
#[allow(dead_code)] // each demo uses only some of the playing techniques
mod guitar;
#[cfg(not(target_os = "none"))]
mod host;

#[cfg(not(target_os = "none"))]
fn main() {
    host::main();
}
