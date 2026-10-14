//! # FretStudy music theory
//!
//! Pure, well-tested music theory with no I/O. Everything the app teaches about notes,
//! chords, scales, keys, and the fretboard is computed here, so lesson content can be
//! checked automatically.
//!
//! String convention: index 0 = thickest string (string 6), index 5 = thinnest (string 1).

// Guitar code indexes strings 0..6 constantly; index loops read more clearly here.
#![allow(clippy::needless_range_loop)]

pub mod chord;
pub mod fretboard;
pub mod interval;
pub mod key;
pub mod library;
pub mod note;
pub mod scale;
pub mod shapes;
pub mod voicings;

pub use chord::{Chord, ChordQuality};
pub use fretboard::{Tuning, string_index, string_number};
pub use interval::{Interval, Role};
pub use key::{Key, Mode};
pub use note::{Letter, NoteName, Pitch, PitchClass, Spelling};
pub use scale::{Scale, ScaleKind};
pub use shapes::{BarreFamily, Caged, ChordShape, NeckNote};
