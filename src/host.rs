//! Host-only demo renderer: `cargo sim [OUT.wav]`.
//!
//! Plays an original, slow doom riff at 52 bpm in C standard (power chords
//! left to ring, palm-muted low-C chugs, a bluesy flat-five walk, slides) and
//! a pentatonic lead with bends and vibrato, on a modelled electric guitar (see
//! `guitar`: exact tuning, pick and pickup, strums, human timing). It drives
//! the patch's entry points the way the firmware does (128-sample blocks,
//! footswitch events at set times) and writes a stereo float WAV. Prints
//! per-section level stats and fails on NaN or clipping.

use crate::guitar::{Note, Player};
use crate::*;

const N: usize = 128;
const SRU: usize = 48_000;
/// The modelled guitar into this patch, matched to the level the patch was voiced for.
const INPUT_TRIM: f32 = 0.6;

// C standard: every string two whole steps down.
const C2: f32 = 65.41;
const DS2: f32 = 77.78;
const F2: f32 = 87.31;
const FS2: f32 = 92.50;
const G2: f32 = 98.0;
const C3: f32 = 130.81;
const DS3: f32 = 155.56;
const F3: f32 = 174.61;
const G3: f32 = 196.0;
const AS3: f32 = 233.08;
const C4: f32 = 261.63;
const DS4: f32 = 311.13;

enum Ev {
    /// Play these notes, strummed this many seconds apart; `true` lets what's ringing carry on.
    Play(Vec<Note>, f32, bool),
    Press,
    Hold,
}

/// A power chord left to ring in front of a wall of amps.
fn power(root: f32, vel: f32) -> Vec<Note> {
    [root, root * 1.4983, root * 2.0].iter().map(|f| Note::new(*f).vel(vel).t60(8.0)).collect()
}

/// A palm-muted chug on the low C (root and fifth).
fn chug() -> Vec<Note> {
    [C2, C2 * 1.4983].iter().map(|f| Note::new(*f).vel(0.85).pm()).collect()
}

#[derive(Clone, Copy)]
enum K {
    Hit,
    Chug,
    /// Slide into the chord from this many semitones below.
    Slide(f32),
}

type Score = Vec<(f64, Ev, &'static str)>;

fn score() -> (Score, f64) {
    let mut ev: Score = vec![];
    let q = 60.0 / 52.0; // quarter note
    let mut t = 0.1;
    // Small, repeatable timing slop: nobody plays exactly on the grid (and doom drags).
    let mut seed: u32 = 0x5eed_1e57;
    let mut slop = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed as f64 / u32::MAX as f64 * 2.0 - 1.0) * 0.008
    };
    // (root, beats, how)
    let bar_a = [(C2, 1.5, K::Hit), (C2, 0.5, K::Chug), (DS2, 0.75, K::Hit), (F2, 0.5, K::Hit), (FS2, 0.25, K::Hit), (F2, 0.5, K::Hit)];
    let bar_b = [(DS2, 1.0, K::Slide(2.0)), (C2, 1.0, K::Hit), (G2, 1.5, K::Slide(1.0)), (FS2, 0.25, K::Hit), (F2, 0.25, K::Hit)];
    let bar_end = [(C2, 4.0, K::Hit)];
    let play_bar = |ev: &mut Score, t: &mut f64, slop: &mut dyn FnMut() -> f64, bar: &[(f32, f64, K)]| {
        for &(root, beats, k) in bar {
            let notes = match k {
                K::Hit => power(root, if beats >= 1.0 { 0.95 } else { 0.85 }),
                K::Chug => chug(),
                K::Slide(st) => power(root, 0.9).into_iter().map(|n| n.slide(st, 0.08)).collect(),
            };
            // Slow, heavy downstrokes.
            ev.push((*t + slop(), Ev::Play(notes, 0.014, false), ""));
            *t += beats * q;
        }
    };
    // Intro: one big chord left to ring for a bar.
    play_bar(&mut ev, &mut t, &mut slop, &bar_end);
    ev.push((t, Ev::Play(vec![], 0.0, true), "riff (amps alone)"));
    for bar in [&bar_a[..], &bar_b[..], &bar_a[..], &bar_b[..]] {
        play_bar(&mut ev, &mut t, &mut slop, bar);
    }
    ev.push((t, Ev::Hold, "riff + Muff"));
    for bar in [&bar_a[..], &bar_b[..]] {
        play_bar(&mut ev, &mut t, &mut slop, bar);
    }
    ev.push((t, Ev::Hold, ""));
    ev.push((t, Ev::Press, "lead (half-cocked wah)"));
    // (note, beats, bend cents, slide semitones, vibrato)
    let lead: [(f32, f64, f32, f32, bool); 12] = [
        (G3, 1.0, 0.0, 2.0, true),
        (AS3, 1.5, 200.0, 0.0, true),
        (G3, 0.5, 0.0, 0.0, false),
        (F3, 0.5, 0.0, 0.0, false),
        (DS3, 0.5, 0.0, 0.0, false),
        (C3, 4.0, 0.0, 0.0, true),
        (C4, 2.0, 0.0, 3.0, true),
        (DS4, 1.0, 100.0, 0.0, false),
        (C4, 0.5, 0.0, 0.0, false),
        (AS3, 0.5, 0.0, 0.0, false),
        (G3, 3.0, 200.0, 0.0, true),
        (F3, 1.0, 0.0, 0.0, false),
    ];
    for (f, beats, bend, slide, vib) in lead {
        let mut n = Note::new(f).vel(0.9);
        if bend > 0.0 {
            n = n.bend(0.12, 0.25, bend);
        }
        if slide > 0.0 {
            n = n.slide(slide, 0.12);
        }
        if vib {
            n = n.vib();
        }
        ev.push((t + slop(), Ev::Play(vec![n], 0.0, false), ""));
        t += beats * q;
    }
    ev.push((t, Ev::Press, "riff again, ending on C"));
    for bar in [&bar_a[..], &bar_end[..]] {
        play_bar(&mut ev, &mut t, &mut slop, bar);
    }
    (ev, t + 3.0)
}

pub fn main() {
    let out = std::env::args().nth(1).unwrap_or("target/render/sleep_demo.wav".into());
    let env = PatchEnv { abi_version: PATCH_ABI_VERSION as u32, user_ctx: core::ptr::null_mut() };
    let mut work = vec![0.0f32; 16];
    unsafe {
        patch_init(&env);
        set_buffer(&env, work.as_mut_ptr());
        for i in 0..PARAM_COUNT {
            set_param(&env, i, get_param_default(&env, i));
        }
    }

    let (mut events, total_s) = score();
    events.sort_by(|a, b| b.0.total_cmp(&a.0));
    let total = (total_s * SRU as f64) as usize;
    let mut guitar = Player::new();

    let mut lo = vec![];
    let mut ro = vec![];
    let mut sections: Vec<(usize, &str)> = vec![(0, "intro (one chord, ringing)")];
    let mut t = 0;
    while t < total {
        while events.last().is_some_and(|(et, _, _)| (*et * SRU as f64) as usize <= t) {
            let (_, e, label) = events.pop().unwrap();
            match e {
                Ev::Press => unsafe { special_action(&env, ACTION_FOOTSWITCH_PRESS) },
                Ev::Hold => unsafe { special_action(&env, ACTION_FOOTSWITCH_HOLD) },
                Ev::Play(notes, strum, ring) => guitar.play(&notes, strum, ring),
            }
            if !label.is_empty() {
                sections.push((t, label));
            }
        }
        let mut l = [0.0f32; N];
        let mut r = [0.0f32; N];
        for x in l.iter_mut() {
            *x = guitar.tick() * INPUT_TRIM;
        }
        unsafe { update_buffers(&env, l.as_mut_ptr(), r.as_mut_ptr(), N as i32) };
        lo.extend_from_slice(&l);
        ro.extend_from_slice(&r);
        t += N;
    }

    let mut bad = false;
    sections.push((lo.len(), ""));
    println!("{:<30} {:>7} {:>8} {:>8} {:>8}", "section", "peak", "rms dB", "L-R dB", "L/R corr");
    for w in sections.windows(2) {
        let (a, name) = w[0];
        let b = w[1].0;
        let (l, r) = (&lo[a..b], &ro[a..b]);
        let peak = l.iter().chain(r).fold(0.0f32, |m, v| m.max(v.abs()));
        let rms = |x: &[f32]| (x.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / x.len() as f64).sqrt();
        let side: Vec<f32> = l.iter().zip(r).map(|(a, b)| a - b).collect();
        let dot: f64 = l.iter().zip(r).map(|(a, b)| *a as f64 * *b as f64).sum();
        let corr = dot / (rms(l) * rms(r) * l.len() as f64).max(1e-12);
        let to_db = |v: f64| 20.0 * v.max(1e-9).log10();
        println!("{name:<30} {peak:>7.3} {:>8.1} {:>8.1} {corr:>8.2}", to_db(rms(l)), to_db(rms(&side)));
        if l.iter().chain(r).any(|v| !v.is_finite()) || peak >= 1.0 {
            bad = true;
        }
    }
    println!("final LED colour index: {}", unsafe { get_state_idx(&env) });

    if let Some(dir) = std::path::Path::new(&out).parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    write_wav(&out, &lo, &ro);
    println!("wrote {out} ({:.1} s)", lo.len() as f64 / SRU as f64);
    if bad {
        eprintln!("error: NaN or full-scale output");
        std::process::exit(1);
    }
    let _ = (get_buffer_size, get_param_min, get_param_max, is_param_enabled, get_param_name, get_param_unit);
}

fn write_wav(path: &str, l: &[f32], r: &[f32]) {
    let bytes = (l.len() * 8) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + bytes).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&48_000u32.to_le_bytes());
    b.extend_from_slice(&(48_000u32 * 8).to_le_bytes());
    b.extend_from_slice(&8u16.to_le_bytes());
    b.extend_from_slice(&32u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&bytes.to_le_bytes());
    for (a, c) in l.iter().zip(r) {
        b.extend_from_slice(&a.to_le_bytes());
        b.extend_from_slice(&c.to_le_bytes());
    }
    std::fs::write(path, b).unwrap();
}
