//! # FretStudy audio
//!
//! - [`synth`]: generated guitar sound (Karplus–Strong) and metronome clicks
//! - [`engine`]: real-time output with sample-accurate scheduling and a shared clock
//! - [`input`]: microphone capture
//! - [`pitch`]: single-note pitch detection (McLeod Pitch Method)
//! - [`tuner`]: the tuner built on top of pitch detection
//! - [`onset`]: note-start detection (latency calibration, rhythm checks)
//! - [`play`]: helpers that turn chord shapes and notes into synth events

pub mod chord;
pub mod dynamics;
pub mod engine;
pub mod input;
pub mod listen;
pub mod onset;
pub mod pitch;
pub mod play;
pub mod synth;
pub mod technique;
pub mod tuner;
pub mod wav;

pub use engine::{AudioEngine, Clock};
pub use input::{InputDeviceInfo, MicInput, list_input_devices};
pub use synth::{Drum, PluckStyle, SynthEvent};
pub use tuner::{Tuner, TunerReading};
