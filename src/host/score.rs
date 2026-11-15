//! The demo score: an original, slow doom riff at 52 bpm in C standard (power
//! chords left to ring, palm-muted low-C chugs, a bluesy flat-five walk,
//! slides) and a pentatonic lead with bends and vibrato. Not the actual
//! *Dopesmoker* riff.

use crate::pedalkit::guitar::Note;
use crate::pedalkit::host::{Ev, Score};

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

/// The events (time s, event, section label) and the total length (s).
pub fn score() -> (Score, f64) {
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
