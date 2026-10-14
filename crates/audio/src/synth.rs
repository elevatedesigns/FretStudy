//! Generated guitar sound: an extended Karplus–Strong plucked-string model, plus a
//! metronome click. Pure DSP with no audio I/O, so it can render offline in tests.

/// Simple xorshift noise (deterministic, allocation-free, real-time safe).
#[derive(Clone)]
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// How a note is plucked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PluckStyle {
    /// 0..1 loudness.
    pub velocity: f32,
    /// 0..1: 0 = warm (soft pick / thumb), 1 = bright (hard pick near the bridge).
    pub brightness: f32,
    /// Palm-muted: short, dull note.
    pub muted: bool,
}

impl Default for PluckStyle {
    fn default() -> Self {
        PluckStyle {
            velocity: 0.8,
            brightness: 0.6,
            muted: false,
        }
    }
}

/// One vibrating string.
///
/// A Karplus–Strong loop whose length can change while the note rings: the newest sample is
/// written at `w`, and the output is read `delay` samples behind it with smooth (Hermite)
/// interpolation. Changing `delay` over time gives bends, slides, hammer-ons, and vibrato.
pub struct StringVoice {
    sample_rate: f32,
    buf: Vec<f32>,
    w: usize,
    delay: f32,
    target_delay: f32,
    /// How far `delay` moves toward `target_delay` each sample (0 = no glide).
    glide: f32,
    vib_depth: f32,
    vib_phase: f32,
    vib_inc: f32,
    vib_left: u32,
    // loop filter
    loss: f32,
    smoothing: f32,
    prev_out: f32,
    // output DC blocker
    dc_x1: f32,
    dc_y1: f32,
    active: bool,
    quiet_samples: u32,
    damping: bool,
    noise: Noise,
    // pick attack transient
    attack: f32,
    attack_decay: f32,
    attack_hp: f32,
}

/// Lowest note the delay line must hold (a little below drop-D's D2 ≈ 73 Hz).
const MIN_FREQ: f32 = 30.0;

impl StringVoice {
    pub fn new(sample_rate: f32, seed: u32) -> Self {
        let cap = (sample_rate / MIN_FREQ) as usize + 8;
        StringVoice {
            sample_rate,
            buf: vec![0.0; cap],
            w: 0,
            delay: 100.0,
            target_delay: 100.0,
            glide: 0.0,
            vib_depth: 0.0,
            vib_phase: 0.0,
            vib_inc: 0.0,
            vib_left: 0,
            loss: 0.999,
            smoothing: 0.5,
            prev_out: 0.0,
            dc_x1: 0.0,
            dc_y1: 0.0,
            active: false,
            quiet_samples: 0,
            damping: false,
            noise: Noise(seed.max(1)),
            attack: 0.0,
            attack_decay: 0.0,
            attack_hp: 0.0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Loop delay for a frequency. The loop filter adds about `smoothing` samples of delay.
    fn delay_for(&self, freq: f32) -> f32 {
        let freq = freq.clamp(MIN_FREQ, self.sample_rate / 4.0);
        (self.sample_rate / freq - self.smoothing).clamp(2.0, self.buf.len() as f32 - 6.0)
    }

    /// Starts a new note, cutting off whatever the string was playing (like a real guitar).
    pub fn pluck(&mut self, freq: f32, style: PluckStyle) {
        let freq = freq.clamp(MIN_FREQ, self.sample_rate / 4.0);
        // Loop filter: one-zero lowpass y = (1-s)x[n] + s x[n-1] (phase delay ≈ s samples).
        self.smoothing = if style.muted {
            0.5
        } else {
            0.5 - 0.2 * style.brightness.clamp(0.0, 1.0) * (freq / 1000.0).min(1.0)
        };
        self.delay = self.delay_for(freq);
        self.target_delay = self.delay;
        self.glide = 0.0;
        self.vib_left = 0;

        // Decay: ~4 s for low notes, shorter for high notes; palm mutes die quickly.
        let t60 = if style.muted {
            0.18
        } else {
            4.5 * (82.0 / freq).powf(0.5)
        };
        // Per-period gain needed for that decay, minus what the loop lowpass already removes.
        let target = 0.001f32.powf(1.0 / (freq * t60));
        let wv = std::f32::consts::TAU * freq / self.sample_rate;
        let s = self.smoothing;
        let lowpass_gain = ((1.0 - s).powi(2) + s * s + 2.0 * s * (1.0 - s) * wv.cos()).sqrt();
        self.loss = (target / lowpass_gain).min(0.99995);

        // Excitation: the shape a pick leaves the string in when it lets go — a triangle
        // with its peak at the pick point (nearer the bridge = brighter) — plus a little
        // filtered noise for the texture of the pick, then softened for pick hardness.
        let m = (self.delay.round() as usize).max(4);
        let bright = style.brightness.clamp(0.0, 1.0) * if style.muted { 0.3 } else { 1.0 };
        let pick_point = 0.10 + 0.14 * (1.0 - bright);
        let apex = ((pick_point * m as f32) as usize).clamp(1, m - 2);
        let mut exc: Vec<f32> = (0..m)
            .map(|i| {
                if i <= apex {
                    i as f32 / apex as f32
                } else {
                    (m - i) as f32 / (m - apex) as f32
                }
            })
            .collect();
        let lp = 0.12 + 0.6 * bright;
        let mut state = 0.0;
        for x in exc.iter_mut() {
            state += lp * (self.noise.next() - state);
            *x += 0.22 * state;
        }
        // Soften (a hard pick keeps more of the sharp corner, a thumb rounds it off).
        let soft = 0.25 + 0.65 * bright;
        for _ in 0..2 {
            let mut prev = exc[m - 1];
            for x in exc.iter_mut() {
                prev += soft * (*x - prev);
                *x = prev;
            }
        }
        let mean = exc.iter().sum::<f32>() / m as f32;
        let peak = exc
            .iter()
            .fold(0.0f32, |a, x| a.max((x - mean).abs()))
            .max(1e-6);
        let gain = style.velocity.clamp(0.0, 1.0) / peak;
        let cap = self.buf.len();
        for x in self.buf.iter_mut() {
            *x = 0.0;
        }
        for (i, x) in exc.iter().enumerate() {
            self.buf[(self.w + cap - m + i) % cap] = (x - mean) * gain;
        }
        self.prev_out = 0.0;
        self.active = true;
        self.damping = false;
        self.quiet_samples = 0;
        // The pick's own little "tick" on top of the string sound.
        self.attack = 0.06 * style.velocity.clamp(0.0, 1.0) * (0.4 + bright);
        self.attack_decay = (-1.0 / (0.004 * self.sample_rate)).exp();
        self.attack_hp = 0.0;
    }

    /// Glides the ringing string to a new pitch over `seconds` (0 = instantly).
    pub fn glide_to(&mut self, freq: f32, seconds: f32) {
        self.target_delay = self.delay_for(freq);
        let samples = (seconds * self.sample_rate).max(1.0);
        self.glide = (self.target_delay - self.delay).abs() / samples;
        if seconds <= 0.0 {
            self.delay = self.target_delay;
            self.glide = 0.0;
        }
    }

    /// A hammer-on, pull-off, or slide: change pitch without a new pick, adding a little
    /// energy (`accent`, 0..1) the way a finger landing on the string does.
    pub fn legato(&mut self, freq: f32, seconds: f32, accent: f32) {
        if !self.active {
            self.pluck(
                freq,
                PluckStyle {
                    velocity: 0.5,
                    ..Default::default()
                },
            );
            return;
        }
        self.glide_to(freq, seconds);
        if accent > 0.0 {
            let cap = self.buf.len();
            let m = (self.target_delay as usize).max(2);
            for i in 0..m / 3 {
                let k = (self.w + cap - m + i) % cap;
                self.buf[k] += self.noise.next() * 0.15 * accent;
            }
        }
        self.damping = false;
        self.quiet_samples = 0;
    }

    /// Vibrato: wobble the pitch `cents` up and down, `rate` times a second, for `seconds`.
    pub fn vibrato(&mut self, cents: f32, rate: f32, seconds: f32) {
        self.vib_depth = 2f32.powf(cents / 1200.0) - 1.0;
        self.vib_inc = std::f32::consts::TAU * rate / self.sample_rate;
        self.vib_phase = 0.0;
        self.vib_left = (seconds * self.sample_rate) as u32;
    }

    /// Stops the note quickly (fretting hand lifting or palm resting on the string).
    pub fn damp(&mut self) {
        if self.active {
            self.damping = true;
        }
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.glide > 0.0 {
            let diff = self.target_delay - self.delay;
            if diff.abs() <= self.glide {
                self.delay = self.target_delay;
                self.glide = 0.0;
            } else {
                self.delay += self.glide * diff.signum();
            }
        }
        let mut d = self.delay;
        if self.vib_left > 0 {
            // Positive phase raises pitch (shorter loop); stays at or above the base pitch
            // like a real bend-style vibrato.
            d /= 1.0 + self.vib_depth * 0.5 * (1.0 - self.vib_phase.cos());
            self.vib_phase += self.vib_inc;
            self.vib_left -= 1;
        }
        let cap = self.buf.len();
        let mut pos = self.w as f32 - d;
        if pos < 0.0 {
            pos += cap as f32;
        }
        let i1 = pos.floor() as usize % cap;
        let t = pos - pos.floor();
        let i0 = (i1 + cap - 1) % cap;
        let i2 = (i1 + 1) % cap;
        let i3 = (i1 + 2) % cap;
        let (x0, x1, x2, x3) = (self.buf[i0], self.buf[i1], self.buf[i2], self.buf[i3]);
        // 4-point Hermite interpolation
        let c1 = 0.5 * (x2 - x0);
        let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3;
        let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);
        let out = ((c3 * t + c2) * t + c1) * t + x1;

        // Damped: fade 60 dB in about 80 ms (each loop pass is one period long).
        let loss = if self.damping {
            (-6.9 * self.delay / (0.08 * self.sample_rate)).exp()
        } else {
            self.loss
        };
        let filtered = loss * ((1.0 - self.smoothing) * out + self.smoothing * self.prev_out);
        self.prev_out = out;
        self.buf[self.w] = filtered;
        self.w = (self.w + 1) % cap;

        // DC blocker
        let mut y = out - self.dc_x1 + 0.995 * self.dc_y1;
        self.dc_x1 = out;
        self.dc_y1 = y;
        if self.attack > 1e-5 {
            // High-passed noise burst: the click of the pick leaving the string.
            let n = self.noise.next();
            let hp = n - self.attack_hp;
            self.attack_hp = n;
            y += hp * self.attack;
            self.attack *= self.attack_decay;
        }

        if y.abs() < 1e-4 {
            self.quiet_samples += 1;
            if self.quiet_samples > self.delay as u32 * 4 + 512 {
                self.active = false;
            }
        } else {
            self.quiet_samples = 0;
        }
        y
    }
}

/// A short metronome "tick". Accented clicks are higher and louder.
pub struct Click {
    sample_rate: f32,
    phase: f32,
    freq: f32,
    amp: f32,
    decay: f32,
}

impl Click {
    pub fn new(sample_rate: f32) -> Self {
        Click {
            sample_rate,
            phase: 0.0,
            freq: 1000.0,
            amp: 0.0,
            decay: 0.0,
        }
    }

    pub fn trigger(&mut self, accent: bool, volume: f32) {
        self.freq = if accent { 1760.0 } else { 1320.0 };
        self.amp = volume * if accent { 0.6 } else { 0.4 };
        self.phase = 0.0;
        // ~25 ms decay
        self.decay = (-1.0 / (0.025 * self.sample_rate)).exp();
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        if self.amp < 1e-4 {
            return 0.0;
        }
        let y = (self.phase * std::f32::consts::TAU).sin() * self.amp;
        self.phase = (self.phase + self.freq / self.sample_rate).fract();
        self.amp *= self.decay;
        y
    }
}

/// Drum sounds for song playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drum {
    Kick,
    Snare,
    HiHat,
}

/// A tiny drum kit: a falling-pitch sine kick, a noisy snare, and a short hi-hat.
pub struct Drums {
    sample_rate: f32,
    kick_phase: f32,
    kick_freq: f32,
    kick_amp: f32,
    snare_amp: f32,
    hat_amp: f32,
    noise: u32,
    last_noise: f32,
}

impl Drums {
    pub fn new(sample_rate: f32) -> Self {
        Drums {
            sample_rate,
            kick_phase: 0.0,
            kick_freq: 0.0,
            kick_amp: 0.0,
            snare_amp: 0.0,
            hat_amp: 0.0,
            noise: 0x1234_5678,
            last_noise: 0.0,
        }
    }

    pub fn trigger(&mut self, drum: Drum, volume: f32) {
        match drum {
            Drum::Kick => {
                self.kick_freq = 130.0;
                self.kick_amp = 0.9 * volume;
                self.kick_phase = 0.0;
            }
            Drum::Snare => self.snare_amp = 0.5 * volume,
            Drum::HiHat => self.hat_amp = 0.25 * volume,
        }
    }

    #[inline]
    pub fn tick(&mut self) -> f32 {
        let sr = self.sample_rate;
        let mut out = 0.0;
        if self.kick_amp > 1e-4 {
            out += (self.kick_phase * std::f32::consts::TAU).sin() * self.kick_amp;
            self.kick_phase = (self.kick_phase + self.kick_freq / sr).fract();
            // The pitch drops toward 45 Hz; the thump fades over ~0.3 s.
            self.kick_freq = 45.0 + (self.kick_freq - 45.0) * (-1.0 / (0.03 * sr)).exp();
            self.kick_amp *= (-1.0 / (0.12 * sr)).exp();
        }
        if self.snare_amp > 1e-4 || self.hat_amp > 1e-4 {
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            let n = (self.noise as f32 / u32::MAX as f32) * 2.0 - 1.0;
            out += n * self.snare_amp;
            // Hi-hat: only the fizzy top (difference of noise samples).
            out += (n - self.last_noise) * 0.5 * self.hat_amp;
            self.last_noise = n;
            self.snare_amp *= (-1.0 / (0.06 * sr)).exp();
            self.hat_amp *= (-1.0 / (0.02 * sr)).exp();
        }
        out
    }
}

/// A two-pole filter section (RBJ cookbook).
#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// Band-pass with 0 dB peak gain at `freq`.
    fn bandpass(sample_rate: f32, freq: f32, q: f32) -> Biquad {
        let w = std::f32::consts::TAU * freq / sample_rate;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        Biquad {
            b0: alpha / a0,
            b1: 0.0,
            b2: -alpha / a0,
            a1: -2.0 * w.cos() / a0,
            a2: (1.0 - alpha) / a0,
            ..Default::default()
        }
    }

    /// Second-order low-pass.
    fn lowpass(sample_rate: f32, freq: f32, q: f32) -> Biquad {
        let w = std::f32::consts::TAU * freq / sample_rate;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        let c = w.cos();
        Biquad {
            b0: (1.0 - c) / 2.0 / a0,
            b1: (1.0 - c) / a0,
            b2: (1.0 - c) / 2.0 / a0,
            a1: -2.0 * c / a0,
            a2: (1.0 - alpha) / a0,
            ..Default::default()
        }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// The wooden body of an acoustic guitar: the air inside rings near 100 Hz, the top near
/// 200 and 400 Hz, and there's a little presence around 2.5 kHz. Very high fizz is rolled off.
#[derive(Clone)]
struct Body {
    modes: [(Biquad, f32); 4],
    top: Biquad,
}

impl Body {
    fn new(sample_rate: f32) -> Body {
        Body {
            modes: [
                (Biquad::bandpass(sample_rate, 100.0, 5.0), 0.9),
                (Biquad::bandpass(sample_rate, 205.0, 6.0), 0.8),
                (Biquad::bandpass(sample_rate, 410.0, 3.5), 0.45),
                (Biquad::bandpass(sample_rate, 2500.0, 1.2), 0.3),
            ],
            top: Biquad::lowpass(sample_rate, 7000.0, 0.6),
        }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let mut y = x * 0.75;
        for (f, g) in self.modes.iter_mut() {
            y += f.run(x) * *g;
        }
        self.top.run(y)
    }
}

/// A small room reverb (Freeverb: parallel damped combs into series all-passes).
struct Room {
    combs: Vec<(Vec<f32>, usize, f32)>,
    allpasses: Vec<(Vec<f32>, usize)>,
}

impl Room {
    fn new(sample_rate: f32, spread: usize) -> Room {
        let scale = sample_rate / 44_100.0;
        let len = |n: usize| (((n + spread) as f32) * scale) as usize;
        Room {
            combs: [1116, 1188, 1277, 1356, 1422, 1491]
                .iter()
                .map(|n| (vec![0.0; len(*n)], 0, 0.0))
                .collect(),
            allpasses: [556, 441, 341]
                .iter()
                .map(|n| (vec![0.0; len(*n)], 0))
                .collect(),
        }
    }

    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        const FEEDBACK: f32 = 0.78;
        const DAMP: f32 = 0.35;
        let mut out = 0.0;
        for (buf, i, store) in self.combs.iter_mut() {
            let y = buf[*i];
            *store = y * (1.0 - DAMP) + *store * DAMP;
            buf[*i] = x + *store * FEEDBACK;
            *i = (*i + 1) % buf.len();
            out += y;
        }
        for (buf, i) in self.allpasses.iter_mut() {
            let b = buf[*i];
            buf[*i] = out + b * 0.5;
            out = b - out;
            *i = (*i + 1) % buf.len();
        }
        out
    }
}

/// What the synth can be told to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SynthEvent {
    /// Pluck a guitar string (0 = thickest) at a frequency.
    Pluck {
        string: usize,
        freq: f32,
        style: PluckStyle,
    },
    /// Stop a string ringing.
    Damp { string: usize },
    /// Bend, slide, hammer-on, or pull-off on a ringing string: glide to `freq` over
    /// `seconds` (0 = instant). `accent` adds a little attack (hammer-ons).
    Legato {
        string: usize,
        freq: f32,
        seconds: f32,
        accent: f32,
    },
    /// Vibrato on a ringing string.
    Vibrato {
        string: usize,
        cents: f32,
        rate: f32,
        seconds: f32,
    },
    /// Metronome tick.
    Click { accent: bool },
    /// A drum hit (song playback).
    Drum { drum: Drum },
    /// Stop everything.
    StopAll,
    /// Forget every event scheduled for the future (handled by the engine, not the synth).
    ClearScheduled,
}

/// Six strings plus a metronome, mixed to mono.
pub struct Synth {
    strings: Vec<StringVoice>,
    click: Click,
    drums: Drums,
    /// A recording being played back, and where it's up to (in source samples).
    sample: Option<(std::sync::Arc<[f32]>, f64, f64)>,
    pub master: f32,
    pub click_volume: f32,
    /// How much room reverb (0 = dry).
    pub room: f32,
    body: [Body; 2],
    rooms: [Room; 2],
    /// Left/right gains per string: low strings a little left, high strings a little right.
    pans: [(f32, f32); 6],
}

impl Synth {
    pub const STRINGS: usize = 6;

    pub fn new(sample_rate: f32) -> Self {
        Synth {
            strings: (0..Self::STRINGS)
                .map(|i| StringVoice::new(sample_rate, 0x9E37_79B9 ^ (i as u32 * 7919 + 1)))
                .collect(),
            click: Click::new(sample_rate),
            drums: Drums::new(sample_rate),
            sample: None,
            master: 0.5,
            click_volume: 0.8,
            room: 0.16,
            body: [Body::new(sample_rate), Body::new(sample_rate)],
            rooms: [Room::new(sample_rate, 0), Room::new(sample_rate, 23)],
            pans: std::array::from_fn(|s| {
                let pan = -0.35 + 0.14 * s as f32; // -1 = left, 1 = right
                let a = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                (
                    a.cos() * std::f32::consts::SQRT_2,
                    a.sin() * std::f32::consts::SQRT_2,
                )
            }),
        }
    }

    pub fn handle(&mut self, event: SynthEvent) {
        match event {
            SynthEvent::Pluck {
                string,
                freq,
                style,
            } => {
                if let Some(v) = self.strings.get_mut(string) {
                    v.pluck(freq, style);
                }
            }
            SynthEvent::Legato {
                string,
                freq,
                seconds,
                accent,
            } => {
                if let Some(v) = self.strings.get_mut(string) {
                    v.legato(freq, seconds, accent);
                }
            }
            SynthEvent::Vibrato {
                string,
                cents,
                rate,
                seconds,
            } => {
                if let Some(v) = self.strings.get_mut(string) {
                    v.vibrato(cents, rate, seconds);
                }
            }
            SynthEvent::Damp { string } => {
                if let Some(v) = self.strings.get_mut(string) {
                    v.damp();
                }
            }
            SynthEvent::Click { accent } => self.click.trigger(accent, self.click_volume),
            SynthEvent::Drum { drum } => self.drums.trigger(drum, self.click_volume),
            SynthEvent::StopAll => {
                self.strings.iter_mut().for_each(StringVoice::damp);
                self.sample = None;
            }
            SynthEvent::ClearScheduled => {}
        }
    }

    /// One mono sample (the average of left and right).
    #[inline]
    pub fn tick(&mut self) -> f32 {
        let (l, r) = self.tick_stereo();
        (l + r) * 0.5
    }

    /// One stereo sample: strings through the guitar body, a little room, then clicks,
    /// drums, and any recording in the middle.
    #[inline]
    pub fn tick_stereo(&mut self) -> (f32, f32) {
        let (mut l, mut r) = (0.0, 0.0);
        for (v, (gl, gr)) in self.strings.iter_mut().zip(self.pans) {
            let x = v.tick();
            l += x * gl;
            r += x * gr;
        }
        let l = self.body[0].run(l) * 0.22;
        let r = self.body[1].run(r) * 0.22;
        let send = (l + r) * 0.5 * 0.015;
        let (wl, wr) = (self.rooms[0].run(send) * 3.0, self.rooms[1].run(send) * 3.0);
        let mut mid = self.click.tick() + self.drums.tick() * 0.6;
        if let Some((data, pos, step)) = self.sample.as_mut() {
            // Linear interpolation, so recordings at any sample rate play at the right speed.
            let i = *pos as usize;
            if i + 1 < data.len() {
                let f = (*pos - i as f64) as f32;
                mid += data[i] * (1.0 - f) + data[i + 1] * f;
                *pos += *step;
            } else {
                self.sample = None;
            }
        }
        let gain = self.master * 2.0;
        (
            soft_clip((l + wl * self.room + mid) * gain),
            soft_clip((r + wr * self.room + mid) * gain),
        )
    }

    /// Plays recorded audio (mono) recorded at `source_rate`, replacing any earlier one.
    pub fn play_sample(&mut self, data: std::sync::Arc<[f32]>, source_rate: f32) {
        self.sample = Some((data, 0.0, (source_rate / self.sample_rate()) as f64));
    }

    pub fn sample_rate(&self) -> f32 {
        self.drums.sample_rate
    }

    /// Is a recording playing?
    pub fn sample_playing(&self) -> bool {
        self.sample.is_some()
    }

    /// Renders mono samples (used offline in tests and by the audio callback).
    pub fn render(&mut self, out: &mut [f32]) {
        for s in out.iter_mut() {
            *s = self.tick();
        }
    }

    pub fn any_active(&self) -> bool {
        self.strings.iter().any(StringVoice::is_active)
    }
}

#[inline]
fn soft_clip(x: f32) -> f32 {
    // Smooth limiter: untouched below ±0.5, then eases toward (never past) ±1.
    let a = x.abs();
    if a <= 0.5 {
        x
    } else {
        x.signum() * (0.5 + 0.5 * ((a - 0.5) / 0.5).tanh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plucked_string_is_in_tune() {
        // After the bright attack fades, the string should ring at its pitch within ~2 cents.
        let sr = 48_000.0;
        for freq in [82.41f32, 110.0, 146.83, 196.0, 246.94, 329.63, 659.26] {
            let mut v = StringVoice::new(sr, 1);
            v.pluck(
                freq,
                PluckStyle {
                    velocity: 0.8,
                    brightness: 0.0,
                    muted: false,
                },
            );
            let samples: Vec<f32> = (0..(sr as usize)).map(|_| v.tick()).collect();
            // Measure the steady ring after the bright attack has faded.
            let mut detector = crate::pitch::PitchDetector::new(8192, sr);
            let measured = detector
                .detect(&samples[24_000..24_000 + 8192])
                .unwrap_or_else(|| panic!("no pitch for {freq}"))
                .frequency;
            let cents = 1200.0 * (measured / freq).log2();
            assert!(
                cents.abs() < 3.0,
                "{freq} Hz measured {measured} ({cents:.2} cents)"
            );
        }
    }

    #[test]
    fn bends_and_slides_reach_their_target() {
        let sr = 48_000.0;
        let mut v = StringVoice::new(sr, 4);
        v.pluck(196.0, PluckStyle::default());
        for _ in 0..4800 {
            v.tick();
        }
        // Whole-step bend: G3 → A3 over 0.2 s.
        v.glide_to(220.0, 0.2);
        let samples: Vec<f32> = (0..(sr as usize / 2)).map(|_| v.tick()).collect();
        let mut d = crate::pitch::PitchDetector::new(4096, sr);
        let f = d
            .detect(&samples[samples.len() - 4096..])
            .unwrap()
            .frequency;
        assert!((1200.0 * (f / 220.0).log2()).abs() < 5.0, "bent to {f}");
        // Hammer-on: instant change, string keeps ringing.
        v.legato(246.94, 0.0, 0.5);
        let samples: Vec<f32> = (0..6000).map(|_| v.tick()).collect();
        let f = d
            .detect(&samples[samples.len() - 4096..])
            .unwrap()
            .frequency;
        assert!(
            (1200.0 * (f / 246.94).log2()).abs() < 5.0,
            "hammered to {f}"
        );
    }

    #[test]
    fn vibrato_wobbles_the_pitch() {
        let sr = 48_000.0;
        let mut v = StringVoice::new(sr, 8);
        v.pluck(220.0, PluckStyle::default());
        v.vibrato(40.0, 5.0, 2.0);
        let samples: Vec<f32> = (0..(sr as usize)).map(|_| v.tick()).collect();
        let mut d = crate::pitch::PitchDetector::new(2048, sr);
        let pitches: Vec<f32> = samples
            .chunks(1024)
            .skip(4)
            .filter(|c| c.len() == 1024)
            .collect::<Vec<_>>()
            .windows(2)
            .filter_map(|w| d.detect(&[w[0], w[1]].concat()).map(|e| e.frequency))
            .collect();
        let lo = pitches.iter().cloned().fold(f32::MAX, f32::min);
        let hi = pitches.iter().cloned().fold(0.0, f32::max);
        let spread = 1200.0 * (hi / lo).log2();
        assert!(
            spread > 15.0 && spread < 60.0,
            "vibrato spread {spread} cents"
        );
    }

    #[test]
    fn notes_decay_and_stop() {
        let sr = 48_000.0;
        let mut v = StringVoice::new(sr, 3);
        v.pluck(
            330.0,
            PluckStyle {
                muted: true,
                ..Default::default()
            },
        );
        for _ in 0..(sr as usize * 2) {
            v.tick();
        }
        assert!(!v.is_active(), "palm-muted note should be silent after 2 s");
    }

    #[test]
    fn output_never_clips() {
        let mut s = Synth::new(48_000.0);
        for string in 0..6 {
            s.handle(SynthEvent::Pluck {
                string,
                freq: 110.0 * (string + 1) as f32,
                style: PluckStyle {
                    velocity: 1.0,
                    brightness: 1.0,
                    muted: false,
                },
            });
        }
        s.handle(SynthEvent::Click { accent: true });
        let mut buf = vec![0.0; 48_000];
        s.render(&mut buf);
        assert!(buf.iter().all(|x| x.abs() <= 1.0));
        assert!(buf.iter().any(|x| x.abs() > 0.05));
    }
}
