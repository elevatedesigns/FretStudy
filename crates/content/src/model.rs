//! The course content format. Lessons are written as RON text files in `content/` and
//! embedded into the app at build time.
//!
//! String numbers in content use the guitarist's numbering: 1 = thinnest, 6 = thickest.

use serde::{Deserialize, Serialize};

/// The five threads that run through every level (plus the level's song).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Thread {
    Technique,
    Chords,
    Rhythm,
    Notes,
    TheoryEar,
    Song,
}

impl Thread {
    pub fn name(self) -> &'static str {
        match self {
            Thread::Technique => "Technique",
            Thread::Chords => "Chords",
            Thread::Rhythm => "Rhythm",
            Thread::Notes => "Notes & Melody",
            Thread::TheoryEar => "Theory & Ear",
            Thread::Song => "Song",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Level {
    pub number: u8,
    pub title: String,
    pub goal: String,
    pub weeks: String,
    pub lessons: Vec<Lesson>,
    /// Set for Level 8 style tracks (electives): the track's short name, e.g. "Blues".
    #[serde(default)]
    pub track: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lesson {
    pub id: String,
    pub title: String,
    pub thread: Thread,
    pub minutes: u8,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    /// Listed on the path but not written yet.
    #[serde(default)]
    pub coming_soon: bool,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    /// Play this lesson in another tuning: "drop_d", "open_d", "open_g", or "dadgad".
    #[serde(default)]
    pub tuning: Option<String>,
    /// How to practice it after the lesson: minutes a day, for how many days, and what.
    #[serde(default)]
    pub practice: String,
}

/// The tuning a lesson names (standard when None or unknown).
pub fn tuning_named(name: Option<&str>) -> theory::Tuning {
    match name {
        Some("drop_d") => theory::Tuning::drop_d(),
        Some("open_d") => theory::Tuning::open_d(),
        Some("open_g") => theory::Tuning::open_g(),
        Some("dadgad") => theory::Tuning::dadgad(),
        _ => theory::Tuning::standard(),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Step {
    /// Learn It / See & Hear It: a kid-simple explanation with a graphic.
    Learn {
        title: String,
        explain: String,
        #[serde(default)]
        more: Option<String>,
        #[serde(default)]
        facts: Vec<String>,
        #[serde(default)]
        visual: Option<Visual>,
        /// "Watch out" list: common mistakes and how to fix them.
        #[serde(default)]
        tips: Vec<String>,
    },
    /// Try It / Play It: hands-on practice. Activities never block progress.
    Activity {
        title: String,
        instructions: String,
        activity: Activity,
    },
    /// Check It: questions that must reach `pass_percent` to complete the lesson.
    Check {
        questions: Vec<Question>,
        pass_percent: u8,
    },
}

impl Step {
    pub fn title(&self) -> &str {
        match self {
            Step::Learn { title, .. } | Step::Activity { title, .. } => title,
            Step::Check { .. } => "Check It",
        }
    }
}

/// A note on the neck or in tab. `string` is 1 (thinnest) … 6 (thickest).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NotePos {
    pub string: u8,
    pub fret: u8,
}

/// A tab note with a length in beats.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabNote {
    pub string: u8,
    pub fret: u8,
    #[serde(default = "one")]
    pub beats: f32,
    /// How this note is played (hammer-on, bend…). None = picked normally.
    #[serde(default)]
    pub tech: Option<Tech>,
}

/// Lead-guitar techniques written in tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tech {
    /// Reached by hammering on from the previous note (tab: h).
    Hammer,
    /// Reached by pulling off from the previous note (tab: p).
    Pull,
    /// Reached by sliding from the previous note (tab: /).
    Slide,
    /// Picked, then bent up this many half steps (1 or 2) (tab: b).
    Bend(u8),
    /// Held with vibrato (tab: ~).
    Vibrato,
}

impl Tech {
    /// The mark written in tab.
    pub fn mark(self) -> &'static str {
        match self {
            Tech::Hammer => "h",
            Tech::Pull => "p",
            Tech::Slide => "/",
            Tech::Bend(_) => "b",
            Tech::Vibrato => "~",
        }
    }
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrumPattern {
    /// Down on every beat (1 2 3 4).
    Quarters,
    /// Down on numbers, up on "and"s.
    Eighths,
    /// D D U U D U (ghost on beat 3).
    OldFaithful,
    /// Bass note on 1, strum on 2, other bass note on 3, strum on 4.
    BoomChuck,
    /// 6/8 fingerpicking: bass, then up and back down the strings (one note per eighth).
    Arpeggio68,
    /// Eight downstrokes per bar (palm-muted rock "chug").
    Chug,
    /// Down-up eighths played long-short (shuffle / swing feel).
    Swing8,
    /// The boogie shuffle riff: power chord 5th, 5th, 6th, 6th… with a swing feel.
    Boogie,
    /// Sixteen strums a bar: down-up-down-up on 1 e & a.
    Sixteenths,
    /// A funk groove: 16ths with accents and muted ghosts.
    Funk16,
    /// Fingerstyle eighths in 4/4: p-i-m-a-m-i-m-i (thumb, then up and back down).
    Pima,
    /// Travis picking: the thumb alternates bass notes on every beat, fingers on the "&"s.
    Travis,
    /// Rock gallop: an eighth and two sixteenths on every beat (palm-muted).
    Gallop,
    /// Bossa nova: thumb on beats 1 and 3, fingers on syncopated off-beats.
    Bossa,
    /// The jazz Charleston: chord stabs on 1 and the "and" of 2.
    Charleston,
    /// Classical tremolo: thumb, then the ring, middle, and index fingers on one string.
    Tremolo,
}

/// One slot of a strum grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stroke {
    Down,
    Up,
    /// The hand moves but misses the strings.
    Ghost,
    /// Pick a bass note: 0 = the chord's root, 1 = the alternate bass note.
    Bass(u8),
    /// Pick one string, counted so that 4 = the thinnest played string, 3 = the next one
    /// down, and so on (works for 4-, 5-, and 6-string chords alike).
    Pluck(u8),
    /// Two-note boogie dyad on the chord's root: 0 = root + 5th, 1 = root + 6th.
    Boogie(u8),
}

impl Stroke {
    /// Does the learner make a sound on this slot?
    pub fn sounds(self) -> bool {
        !matches!(self, Stroke::Ghost)
    }
}

impl StrumPattern {
    /// The slots of one bar.
    pub fn slots(self) -> Vec<Stroke> {
        use Stroke::*;
        match self {
            StrumPattern::Quarters => vec![Down, Ghost, Down, Ghost, Down, Ghost, Down, Ghost],
            StrumPattern::Eighths => vec![Down, Up, Down, Up, Down, Up, Down, Up],
            StrumPattern::OldFaithful => vec![Down, Ghost, Down, Up, Ghost, Up, Down, Up],
            StrumPattern::BoomChuck => {
                vec![Bass(0), Ghost, Down, Ghost, Bass(1), Ghost, Down, Ghost]
            }
            StrumPattern::Arpeggio68 => {
                vec![Bass(0), Pluck(2), Pluck(3), Pluck(4), Pluck(3), Pluck(2)]
            }
            StrumPattern::Chug => vec![Down; 8],
            StrumPattern::Swing8 => vec![Down, Up, Down, Up, Down, Up, Down, Up],
            StrumPattern::Boogie => vec![
                Boogie(0),
                Boogie(0),
                Boogie(1),
                Boogie(1),
                Boogie(0),
                Boogie(0),
                Boogie(1),
                Boogie(1),
            ],
            StrumPattern::Sixteenths => [Down, Up].repeat(8),
            StrumPattern::Funk16 => vec![
                Down, Ghost, Ghost, Up, Ghost, Up, Down, Ghost, Down, Ghost, Ghost, Up, Ghost, Up,
                Down, Up,
            ],
            StrumPattern::Pima => vec![
                Bass(0),
                Pluck(2),
                Pluck(3),
                Pluck(4),
                Pluck(3),
                Pluck(2),
                Pluck(3),
                Pluck(2),
            ],
            StrumPattern::Travis => vec![
                Bass(0),
                Pluck(3),
                Bass(1),
                Pluck(2),
                Bass(0),
                Pluck(4),
                Bass(1),
                Pluck(2),
            ],
            StrumPattern::Gallop => [Down, Ghost, Down, Down].repeat(4),
            StrumPattern::Bossa => vec![Bass(0), Ghost, Ghost, Up, Bass(1), Up, Ghost, Ghost],
            StrumPattern::Charleston => vec![Down, Ghost, Ghost, Down, Ghost, Ghost, Ghost, Ghost],
            StrumPattern::Tremolo => [Bass(0), Pluck(4), Pluck(4), Pluck(4)].repeat(4),
        }
    }

    /// Slots per counted beat: 2 for 4/4 eighth-note grids, 1 for 6/8 (count every eighth).
    pub fn slots_per_beat(self) -> usize {
        match self {
            StrumPattern::Arpeggio68 => 1,
            StrumPattern::Sixteenths
            | StrumPattern::Funk16
            | StrumPattern::Gallop
            | StrumPattern::Tremolo => 4,
            _ => 2,
        }
    }

    /// Swing feel: the second eighth of each beat lands two-thirds of the way through.
    pub fn swing(self) -> bool {
        matches!(self, StrumPattern::Swing8 | StrumPattern::Boogie)
    }

    /// Where slot `k` (0-based, within one beat) starts, as a fraction of the beat.
    pub fn slot_offset(self, k: usize) -> f32 {
        let spb = self.slots_per_beat();
        if self.swing() && spb == 2 && k == 1 {
            2.0 / 3.0
        } else {
            k as f32 / spb as f32
        }
    }

    /// Beats in one bar (4 for 4/4, 6 for 6/8).
    pub fn beats_per_bar(self) -> usize {
        self.slots().len() / self.slots_per_beat()
    }

    /// The count written under each slot.
    pub fn counts(self) -> Vec<&'static str> {
        match self {
            StrumPattern::Arpeggio68 => vec!["1", "2", "3", "4", "5", "6"],
            StrumPattern::Sixteenths
            | StrumPattern::Funk16
            | StrumPattern::Gallop
            | StrumPattern::Tremolo => [
                "1", "e", "&", "a", "2", "e", "&", "a", "3", "e", "&", "a", "4", "e", "&", "a",
            ]
            .to_vec(),
            _ => vec!["1", "&", "2", "&", "3", "&", "4", "&"],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            StrumPattern::Quarters => "Down on every beat",
            StrumPattern::Eighths => "Down-up eighth notes",
            StrumPattern::OldFaithful => "Old Faithful (D D U U D U)",
            StrumPattern::BoomChuck => "Boom-chuck (bass, strum, bass, strum)",
            StrumPattern::Arpeggio68 => "6/8 fingerpicking (bass, then up and down)",
            StrumPattern::Chug => "Downstroke eighths (the rock chug)",
            StrumPattern::Swing8 => "Swing eighths (long-short)",
            StrumPattern::Boogie => "Boogie shuffle (5th, 5th, 6th, 6th)",
            StrumPattern::Sixteenths => "Sixteenth notes (1 e & a)",
            StrumPattern::Funk16 => "Funk sixteenths (accents and ghosts)",
            StrumPattern::Pima => "Fingerpicking p-i-m-a-m-i-m-i",
            StrumPattern::Travis => "Travis picking (thumb on every beat)",
            StrumPattern::Gallop => "The gallop (one and two-and)",
            StrumPattern::Bossa => "Bossa nova (thumb on 1 and 3)",
            StrumPattern::Charleston => "The Charleston (1 and the \"and\" of 2)",
            StrumPattern::Tremolo => "Tremolo (p-a-m-i)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Visual {
    /// Interactive labeled guitar.
    GuitarParts,
    /// A fretting hand with finger numbers and colors.
    Hand,
    /// The six strings with numbers and names.
    Strings,
    /// A chord from the chord library, by id.
    Chord(String),
    /// Two library chords side by side.
    ChordPair(String, String),
    /// Any shape (e.g. a barre chord in placement questions): frets + fingers, thickest first.
    Shape {
        frets: String,
        fingers: String,
        #[serde(default)]
        name: String,
    },
    /// Notes on a neck map.
    Neck {
        notes: Vec<NotePos>,
        #[serde(default)]
        first_fret: u8,
        #[serde(default = "twelve")]
        last_fret: u8,
        #[serde(default)]
        show_names: bool,
    },
    Tab(Vec<TabNote>),
    StrumGrid(StrumPattern),
    BeatPulse {
        bpm: f32,
    },
    /// One octave of piano keys (half and whole steps).
    Piano,
    /// The musical alphabet A–G repeating, with octaves.
    Alphabet,
    /// The W-W-H-W-W-W-H staircase of a major scale from a root (e.g. "G").
    ScaleSteps {
        root: String,
    },
    /// The seven chords of a key, with jersey numbers (7th chords and minor keys optional).
    NumberWheel {
        key: String,
        #[serde(default)]
        sevenths: bool,
        #[serde(default)]
        minor: bool,
    },
    /// Note and rest lengths on a 4-beat timeline.
    Durations,
    /// A capo on a fret, with a library chord shape played above it.
    Capo {
        fret: u8,
        chord: String,
    },
    /// A scale on the neck in note-job colors. `scale`: "minor_pentatonic",
    /// "major_pentatonic", "blues", "major", "natural_minor", "dorian", "mixolydian"...
    /// `shape` 1–5 picks one pentatonic box (the rest of the neck shows faint).
    ScaleBox {
        root: String,
        scale: String,
        #[serde(default)]
        shape: Option<u8>,
        #[serde(default)]
        first_fret: u8,
        #[serde(default = "fifteen")]
        last_fret: u8,
    },
    /// One major chord in all five CAGED shapes along the neck.
    Caged {
        root: String,
    },
    /// The 12-bar blues chart in a key (numbers and chord names).
    TwelveBar {
        key: String,
    },
    /// Two scales on one neck: shared notes, notes only in `a`, and notes only in `b`.
    ScaleCompare {
        root: String,
        a: String,
        b: String,
        #[serde(default)]
        first_fret: u8,
        #[serde(default = "twelve")]
        last_fret: u8,
    },
    /// Type a chord symbol and see its recipe, notes, a shape, and hear it.
    Decoder {
        examples: Vec<String>,
    },
    /// Standard notation with tab underneath, in a major key (None = C, no signature).
    Staff {
        notes: Vec<TabNote>,
        #[serde(default)]
        key: Option<String>,
        /// Beats in a bar (4 = 4/4, 3 = 3/4).
        #[serde(default = "four")]
        bar: u8,
    },
}

fn fifteen() -> u8 {
    15
}

/// Parses a scale name used in content.
pub fn scale_kind(name: &str) -> Option<theory::ScaleKind> {
    use theory::ScaleKind::*;
    Some(match name {
        "minor_pentatonic" => MinorPentatonic,
        "major_pentatonic" => MajorPentatonic,
        "blues" => Blues,
        "major" => Major,
        "natural_minor" => NaturalMinor,
        "dorian" => Dorian,
        "mixolydian" => Mixolydian,
        "harmonic_minor" => HarmonicMinor,
        "lydian" => Lydian,
        "phrygian" => Phrygian,
        "locrian" => Locrian,
        "melodic_minor" => MelodicMinor,
        "chromatic" => Chromatic,
        _ => return None,
    })
}

fn twelve() -> u8 {
    12
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarKind {
    /// Which of two notes is higher?
    HighLow,
    /// Is this chord major or minor?
    MajorMinor,
    /// Which chord sounds finished (home) vs. wanting to move (G vs D7)?
    HomeOrAway,
    /// After the 1 chord (G), did it move to the 4 chord (C) or the 5 chord (D)?
    FourOrFive,
    /// Major 3rd or minor 3rd?
    Thirds,
    /// Perfect 4th or perfect 5th?
    FourthsFifths,
    /// Which progression: 1–4–5–1, 1–5–6–4, or 6–4–1–5?
    Progressions,
    /// Straight or swung eighths?
    StraightOrSwing,
    /// Major 7th chord or dominant 7th chord?
    Sevenths,
    /// Dorian or Aeolian (natural minor) over an A drone?
    DorianAeolian,
    /// Mixolydian or Ionian (major) over an A drone?
    MixolydianIonian,
    /// In a minor key, does the 5 chord sound minor (v) or major (V)?
    MinorFive,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Activity {
    /// Tap along with a metronome.
    TapBeat { bpm: f32, taps: u32 },
    /// Tune all six strings with the built-in tuner.
    Tune,
    /// Find named guitar parts on the picture.
    PartsGame { rounds: u8 },
    /// Find (or pluck) the named string.
    StringGame { rounds: u8 },
    /// Pluck a chord one string at a time; the mic checks each string.
    StringCheck { chord: String },
    /// Strum chords along with a metronome; the mic checks timing.
    StrumAlong {
        chords: Vec<String>,
        beats_per_chord: u8,
        pattern: StrumPattern,
        tempos: Vec<f32>,
        #[serde(default = "two")]
        repeats: u8,
    },
    /// Switch between two chords as many times as possible in a minute.
    ChangeTrainer { a: String, b: String, target: u32 },
    /// Play notes from tab; the mic listens for each note.
    PlayNotes {
        notes: Vec<TabNote>,
        tempo: f32,
        /// Beats in a bar (4 = 4/4, 3 = 3/4).
        #[serde(default = "four")]
        bar: u8,
    },
    /// Listening game.
    EarQuiz { kind: EarKind, rounds: u8 },
    /// Tick off each item.
    Checklist { items: Vec<String> },
    /// Find named notes on the neck (click, or pluck with the mic on).
    NoteHunt {
        /// String numbers (1–6) the notes may be on.
        strings: Vec<u8>,
        max_fret: u8,
        #[serde(default = "yes")]
        naturals_only: bool,
        rounds: u8,
    },
    /// Strum each chord; the mic checks every string.
    ChordCheck { chords: Vec<String> },
    /// Pick a fretted note and bend it up `steps` half steps; the mic checks the pitch.
    BendCheck { string: u8, fret: u8, steps: u8 },
    /// Hold a note with vibrato; the mic measures how steady it is.
    VibratoCheck { string: u8, fret: u8 },
    /// Solo over a backing groove. The mic lights up the notes you play on the scale map.
    Jam {
        key: String,
        scale: String,
        #[serde(default)]
        shape: Option<u8>,
        /// Chords of the backing, one per bar (library ids).
        chords: Vec<String>,
        pattern: StrumPattern,
        bpm: f32,
        /// Show the current chord's tones in color and the rest of the scale in gray.
        #[serde(default)]
        chord_tones: bool,
    },
    /// Record yourself (optionally over a backing groove); takes are saved to listen back.
    Record {
        #[serde(default)]
        chords: Vec<String>,
        #[serde(default = "eighths")]
        pattern: StrumPattern,
        #[serde(default = "eighty")]
        bpm: f32,
    },
    /// Play the same passage soft, loud, and short; the mic compares the takes.
    Expression { notes: Vec<TabNote> },
    /// Hear a 4-chord progression in a key and tap its numbers in order.
    Detective { key: String, rounds: u8 },
    /// Build a song in the Song Builder; done when the verse and chorus have enough bars.
    SongBuilder { verse_bars: u8, chorus_bars: u8 },
}

fn four() -> u8 {
    4
}

fn eighths() -> StrumPattern {
    StrumPattern::Eighths
}

fn eighty() -> f32 {
    80.0
}

fn yes() -> bool {
    true
}

fn two() -> u8 {
    2
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Audio {
    /// Strum these library chords one after another.
    Chords(Vec<String>),
    /// Play these notes in order.
    Notes(Vec<TabNote>),
}

/// A multiple-choice question (Check It, and placement).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub prompt: String,
    #[serde(default)]
    pub visual: Option<Visual>,
    #[serde(default)]
    pub audio: Option<Audio>,
    pub choices: Vec<String>,
    pub answer: usize,
    /// Shown after answering ("Not yet — here's why").
    #[serde(default)]
    pub explain: String,
}

/// One placement checkpoint item.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacementItem {
    pub id: String,
    pub level: u8,
    pub thread: Thread,
    pub question: Question,
}

/// Stage A options: what the learner says they can do, and the level each points to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillClaim {
    pub id: String,
    pub text: String,
    pub level: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacementBank {
    pub claims: Vec<SkillClaim>,
    pub items: Vec<PlacementItem>,
}
