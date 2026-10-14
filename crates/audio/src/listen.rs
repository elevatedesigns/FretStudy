//! Note listener: reports each new note the learner plays (when it started and its pitch).
//! Used to follow melodies and to check chords string by string.

use crate::onset::OnsetDetector;
use crate::pitch::PitchDetector;
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Heard {
    /// A new note with a clear pitch.
    Note { onset: u64, freq: f32 },
    /// Something was played, but no clear pitch (muted, buzzing, or several strings).
    Unclear { onset: u64 },
}

pub struct NoteListener {
    sample_rate: u32,
    onsets: OnsetDetector,
    detector: PitchDetector,
    buf: VecDeque<f32>,
    position: u64,
    since_hop: usize,
    hop: usize,
    awaiting: Option<u64>,
    previous: Option<f32>,
    /// Also report notes that start without a new pick (hammer-ons, pull-offs, slides).
    pub legato: bool,
    /// The last note reported, and a candidate new pitch seen without an onset.
    last_note: Option<f32>,
    legato_candidate: Option<f32>,
}

impl NoteListener {
    pub fn new(sample_rate: u32) -> NoteListener {
        let size = if sample_rate > 60_000 { 4096 } else { 2048 };
        let mut detector = PitchDetector::new(size, sample_rate as f32);
        detector.min_freq = 70.0;
        detector.max_freq = 1400.0;
        detector.min_level_db = -60.0;
        NoteListener {
            sample_rate,
            onsets: OnsetDetector::new(sample_rate),
            detector,
            buf: VecDeque::with_capacity(size * 2),
            position: 0,
            since_hop: 0,
            hop: size / 4,
            awaiting: None,
            previous: None,
            legato: false,
            last_note: None,
            legato_candidate: None,
        }
    }

    /// Feeds samples starting at absolute frame `start`. Returns notes heard.
    pub fn push(&mut self, start: u64, samples: &[f32]) -> Vec<Heard> {
        let mut out = Vec::new();
        let onsets = self.onsets.push(start, samples);
        let size = self.detector.window_size();
        let wait_after_onset = self.sample_rate as u64 * 50 / 1000; // let the attack settle
        let give_up = self.sample_rate as u64 * 450 / 1000;
        self.position = start;
        let mut next_onset = onsets.iter().peekable();
        for &s in samples {
            if next_onset.peek().is_some_and(|o| **o <= self.position) {
                let o = *next_onset.next().unwrap();
                if let Some(prev) = self.awaiting.take() {
                    out.push(Heard::Unclear { onset: prev });
                }
                self.awaiting = Some(o);
                self.previous = None;
            }
            if self.buf.len() == size {
                self.buf.pop_front();
            }
            self.buf.push_back(s);
            self.position += 1;
            self.since_hop += 1;
            if self.since_hop < self.hop {
                continue;
            }
            self.since_hop = 0;
            let Some(onset) = self.awaiting else {
                if self.legato && self.buf.len() == size {
                    self.check_legato(&mut out);
                }
                continue;
            };
            let age = self.position.saturating_sub(onset);
            if age < wait_after_onset || self.buf.len() < size {
                continue;
            }
            if age > give_up {
                out.push(Heard::Unclear { onset });
                self.awaiting = None;
                continue;
            }
            let window: Vec<f32> = self.buf.iter().copied().collect();
            if let Some(e) = self.detector.detect(&window).filter(|e| e.clarity >= 0.8) {
                // Two estimates in a row that agree → a confident note.
                if let Some(p) = self.previous
                    && (1200.0 * (e.frequency / p).log2()).abs() < 30.0
                {
                    let freq = (e.frequency + p) / 2.0;
                    out.push(Heard::Note { onset, freq });
                    self.last_note = Some(freq);
                    self.awaiting = None;
                    self.previous = None;
                    continue;
                }
                self.previous = Some(e.frequency);
            }
        }
        out
    }

    /// Between picks: a steady new pitch (two readings agree, more than 60 cents from the
    /// last note) means a hammer-on, pull-off, or slide landed.
    fn check_legato(&mut self, out: &mut Vec<Heard>) {
        let window: Vec<f32> = self.buf.iter().copied().collect();
        let Some(e) = self.detector.detect(&window).filter(|e| e.clarity >= 0.85) else {
            self.legato_candidate = None;
            return;
        };
        let far = |a: f32, b: f32| (1200.0 * (a / b).log2()).abs() > 60.0;
        if self.last_note.is_some_and(|l| !far(e.frequency, l)) {
            self.legato_candidate = None;
            return;
        }
        match self.legato_candidate {
            Some(c) if !far(e.frequency, c) => {
                let freq = (e.frequency + c) / 2.0;
                out.push(Heard::Note {
                    onset: self.position,
                    freq,
                });
                self.last_note = Some(freq);
                self.legato_candidate = None;
            }
            _ => self.legato_candidate = Some(e.frequency),
        }
    }
}

/// Does a heard frequency match the expected MIDI note? Allows the octave above or below,
/// because microphones sometimes favor an overtone.
pub fn matches_note(freq: f32, expected_midi: i32, tolerance_cents: f32) -> bool {
    let midi = 69.0 + 12.0 * (freq / 440.0).log2();
    let diff = midi - expected_midi as f32;
    [0.0f32, 12.0, -12.0]
        .iter()
        .any(|o| ((diff - o) * 100.0).abs() <= tolerance_cents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{PluckStyle, Synth, SynthEvent};

    fn render(plucks: &[(f32, usize, f32)], seconds: f32) -> Vec<f32> {
        // (time s, string, freq)
        let sr = 48_000.0;
        let mut synth = Synth::new(sr);
        let n = (sr * seconds) as usize;
        let mut out = Vec::with_capacity(n);
        let mut next = 0;
        for i in 0..n {
            while next < plucks.len() && (plucks[next].0 * sr) as usize <= i {
                let (_, string, freq) = plucks[next];
                synth.handle(SynthEvent::Pluck {
                    string,
                    freq,
                    style: PluckStyle::default(),
                });
                next += 1;
            }
            out.push(synth.tick());
        }
        out
    }

    #[test]
    fn hears_a_melody_including_repeated_notes() {
        // E E F G on string 1 (Ode to Joy opening), half a second apart.
        let e = 329.63;
        let f = 349.23;
        let g = 392.0;
        let x = render(&[(0.2, 5, e), (0.7, 5, e), (1.2, 5, f), (1.7, 5, g)], 2.4);
        let mut l = NoteListener::new(48_000);
        let mut heard = Vec::new();
        for (i, chunk) in x.chunks(480).enumerate() {
            heard.extend(l.push((i * 480) as u64, chunk));
        }
        let notes: Vec<f32> = heard
            .iter()
            .filter_map(|h| match h {
                Heard::Note { freq, .. } => Some(*freq),
                _ => None,
            })
            .collect();
        assert_eq!(notes.len(), 4, "heard {heard:?}");
        for (got, want) in notes.iter().zip([e, e, f, g]) {
            assert!(
                (1200.0 * (got / want).log2()).abs() < 15.0,
                "{got} vs {want}"
            );
        }
    }

    #[test]
    fn hears_hammer_ons_in_legato_mode() {
        use crate::synth::StringVoice;
        let sr = 48_000.0;
        let mut v = StringVoice::new(sr, 2);
        let mut l = NoteListener::new(48_000);
        l.legato = true;
        let mut heard = Vec::new();
        let mut buf = Vec::new();
        for i in 0..(sr as usize * 2) {
            if i == 4800 {
                v.pluck(293.66, crate::synth::PluckStyle::default()); // D
            }
            if i == 33_600 {
                v.legato(329.63, 0.0, 0.4); // hammer-on to E, no new pick
            }
            buf.push(v.tick());
            if buf.len() == 480 {
                heard.extend(l.push((i + 1 - 480) as u64, &buf));
                buf.clear();
            }
        }
        let notes: Vec<f32> = heard
            .iter()
            .filter_map(|h| {
                if let Heard::Note { freq, .. } = h {
                    Some(*freq)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(notes.len(), 2, "{heard:?}");
        assert!(
            matches_note(notes[0], 62, 30.0) && matches_note(notes[1], 64, 30.0),
            "{notes:?}"
        );
    }

    #[test]
    fn note_matching() {
        assert!(matches_note(329.63, 64, 30.0));
        assert!(matches_note(659.26, 64, 30.0)); // octave up still counts
        assert!(!matches_note(349.23, 64, 30.0)); // F is not E
    }
}
