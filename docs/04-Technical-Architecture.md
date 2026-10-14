# FretStudy — Technical Architecture (Rust Desktop App)

**Target:** a native desktop app for **macOS** (Apple Silicon and Intel) and **Windows 10/11**, written in Rust. It's offline-first, all data is local, all sound is generated, and the microphone is used for checks.

---

## 1. Decisions

| Area | Choice | Why |
|---|---|---|
| Language | Rust (stable) | Requested. Fast, safe, one codebase for Mac and Windows. Great fit for real-time audio. |
| UI | **egui** via **eframe** | Fretboards, chord boxes, tab, and animations are all **custom-drawn**. egui's painter API makes custom 2D drawing and animation simple. It runs on macOS and Windows from one codebase, ships as a single binary, and has screen-reader support through AccessKit. *Considered:* Iced (more structured, weaker custom drawing), Slint (nice declarative UI, less flexible canvas), Tauri (rejected: it's a web UI inside a desktop shell). |
| Audio in/out | **cpal** | The standard cross-platform audio I/O crate (CoreAudio on Mac, WASAPI on Windows). |
| Audio thread messaging | **rtrb** (lock-free ring buffer) | The real-time audio callback must never lock or allocate. |
| DSP | **rustfft**, **pitch-detection** (McLeod pitch method), custom code | FFT for chord and onset analysis. MPM is accurate for guitar pitch. |
| WAV files | **hound** | Saving recordings and loading test samples. |
| Local database | **rusqlite** (feature `bundled`) | SQLite compiled in, with no install needed. One file per computer. Handles multiple profiles. |
| Content files | **serde** + **RON** | Lessons, songs, and assessment items are human-editable text files, embedded in the app at build time. |
| File locations | **directories** | Correct per-OS app-data folders. |
| Errors / logging | **anyhow**, **thiserror**, **tracing** | Standard choices. |
| Fonts | **Atkinson Hyperlegible** (SIL OFL), **Bravura** (SIL OFL, music symbols) | Very readable for kids and adults. Bravura draws notes, rests, and clefs for standard notation. Both are free to bundle. |
| Packaging | **cargo-packager** | Builds a `.app`/`.dmg` for macOS and an `.msi`/NSIS installer for Windows. |

Exact crate versions are pinned in `Cargo.lock` when the workspace is created.

---

## 2. Project Layout (Cargo workspace)

```
fretstudy/
├── Cargo.toml                  # workspace
├── crates/
│   ├── theory/                 # PURE music theory, no I/O. Heavily unit-tested.
│   │   └── notes, intervals, scales, chords, keys, tunings, fretboard mapping,
│   │       chord-shape → notes, chord naming, CAGED/pentatonic shape generators
│   ├── content/                # Lesson/song/assessment schema + loader + validator
│   │   └── prerequisite graph, "what can I play now" queries, placement algorithm
│   ├── audio/                  # Real-time engine
│   │   ├── synth/              # plucked-string guitar, strums, metronome, drums, bass
│   │   ├── sched/              # sample-accurate event scheduler + transport clock
│   │   └── listen/             # mic capture, tuner, note/chord/onset/bend detectors
│   ├── store/                  # SQLite: profiles, progress, practice log, migrations, backup
│   └── app/                    # eframe app (the binary)
│       ├── screens/            # profiles, setup, placement, path map, lesson player,
│       │                       # practice tools, songbook, library, settings
│       ├── widgets/            # ChordBox, NeckMap, TabStaff, StrumGrid, NumberWheel,
│       │                       # Tuner, PitchMeter, TimingMeter, StaffNotation
│       └── theme/              # Okabe–Ito colors, Playful/Clean looks, fonts, text sizes
├── content/                    # Course data (RON), embedded into the binary
│   ├── levels/L0.ron … L8-*.ron
│   ├── songs/*.ron
│   └── assessment/*.ron
├── assets/fonts/
└── xtask/                      # `cargo xtask validate` / `cargo xtask package`
```

**Why split into crates:** `theory` and `content` can be tested without any UI or audio. That's where accuracy lives. The UI and audio crates stay thin.

---

## 3. Course Content as Data

Every lesson in the curriculum docs becomes a RON file entry, for example:

```ron
Lesson(
  id: "L2.4",
  title: "E and Am: Same Shape, New Spot!",
  thread: Chords, minutes: 12,
  prerequisites: ["L2.3"],
  skills: ["chord.open.E", "chord.open.Am"],
  explain: "Here's a secret: E major and A minor are the exact same finger shape...",
  more_detail: "Both chords are built 1-3-5 ...",
  steps: [
    Learn(diagram: ChordPair(a: "E", b: "Am", animate: SlideOneString)),
    Try(exercise: OneMinuteChanges(a: "Am", b: "E", target: 25)),
    Play(progression: ["Am", "E", "Am", "E"], bpm: 70, strum: "quarters"),
    Check(items: [MicChord("Am"), MicChord("E"), SelfRate("I can switch Am to E without stopping")]),
  ],
)
```

Chords are defined once in a shared library (`E = [0,2,2,1,0,0]`, `fingers = [_,2,3,1,_,_]`), and lessons refer to them by name.

### `cargo xtask validate` (runs in CI and before every release)
The validator automatically checks the accuracy rules from the curriculum appendix:
1. **Chord shapes match their names.** The `theory` crate computes the notes from frets plus standard tuning and confirms they spell the named chord (for example, `x32010` → C E G → "C").
2. **Scale and pentatonic shapes** contain exactly the right pitch classes, and roots are marked correctly.
3. **Assessment answers** are computed and compared (for example, "6 chord in G" → Em).
4. **The prerequisite graph** has no cycles, and every ID exists.
5. **Songs and exercises use only chords and skills** taught in that lesson's prerequisites.
6. **Reading level:** every `explain` box scores Flesch–Kincaid grade ≤ 5.5.
7. **Public domain flag:** every song has `public_domain: true` or `original: true`.
8. **Every quiz answer is explained.** Each lesson question has an `explain` of at least 6 words, shown after any answer (right or wrong).
9. **Lessons teach in depth.** Each lesson needs at least 2 Learn steps, at least 2 pictures, at least 120 teaching words (explain + facts + tips), "Watch out" tips on at least one step, and a `practice:` plan.

---

## 4. Generated Sound (synth)

- **Sound chain (Oct 2026 upgrade):** each string is a Karplus–Strong voice excited by a triangle "pluck shape" plus a short high-passed pick-attack burst. Strings are panned across a stereo field, then go through a guitar **body** (resonant band-pass filters at about 100, 205, 410, and 2500 Hz) and a small Freeverb-style **room** reverb. Output is soft-clipped. `cargo run -p audio --example sound_demo -- out.wav` renders a demo for listening.
- **Guitar voice:** an extended **Karplus–Strong** plucked-string model (a delay line plus a damping filter). It includes fractional-delay tuning so notes are exactly in tune, a pick-position filter (bright near the bridge, warm near the neck), and a small body-resonance filter (acoustic vs. electric clean presets).
- **Techniques:** palm mute (fast decay plus low-pass), hammer-on and pull-off (re-excite without a new pluck), slides (glide the delay length through the frets), bends (smooth pitch glide to the target), vibrato (LFO on pitch), harmonics (pluck at 1/2, 1/3, or 1/4 position).
- **Strums:** each string is plucked 8–25 ms apart (down = low→high, up = high→low). Muted strings (`x`) are skipped. "Chk" strums use a short noise burst.
- **Backing:** metronome (accent on beat 1), simple drum kits (rock, shuffle, folk, bossa), and a sine/pluck bass. This is enough for every play-along in the curriculum.
- **Scheduling:** the UI sends time-stamped events to the audio thread. The audio thread owns a **sample counter** (the master clock) that the UI reads to draw playheads and animations, so sound and graphics stay in sync.
- **Speed control:** 50%, 75%, and 100% change the tempo, not the pitch.

---

## 5. Microphone Checks

| Check | Method | What it can judge reliably | What it can't |
|---|---|---|---|
| **Tuner** | McLeod Pitch Method on a 4096-sample window (48 kHz), smoothed. Readout in cents. | Pitch of a single open string within ±2 cents in a quiet room | Several strings ringing at once |
| **Single notes / melodies** | Onset detection plus MPM pitch per note, compared to the expected sequence with timing windows | Right note (including octave), timing | Which string/fret was used (the same pitch on two strings sounds the same) |
| **String-by-string chord check** ("Buzz Detective") | The learner plucks each string in turn, and each pitch is checked against the shape | Exactly which string is muted, buzzing, or wrong | — |
| **Strummed chord check** | FFT → 12-bin chroma (pitch-class energy) ~150–400 ms after the strum onset, compared to the **expected** chord's pitch classes | Whether the chord's notes are present, and whether a wrong note is strongly present | Exactly which string is dead (use the string-by-string check for that) |
| **Strum and rhythm timing** | Spectral-flux onset detection vs. the beat grid, after latency calibration | On-time / early / late, and whether hits match the pattern's hit positions | Down vs. up strum direction |
| **Bend accuracy** | Pitch tracking over time | Reached the target within ±20 cents | — |
| **Vibrato** | Pitch-wobble rate and depth | Steady vs. uneven | — |
| **Swing feel** | Ratio between off-beat onset times | Straight vs. swung | — |
| **Muted vs. open (palm mute)** | Decay time and high-frequency energy | Clearly muted vs. clearly open | Subtle partial muting |

Because the app always knows **what the learner is supposed to play**, it *verifies* rather than *recognizes*. That's much more reliable than general chord recognition.

**First-time setup (≈1 minute):**
1. Pick the input device and confirm the permission prompt.
2. Level check: "play your low E string" (sets the input gain threshold, warns if too quiet or noisy).
3. **Latency calibration:** the app plays clicks and listens for them through the speakers. With headphones, the learner strums along to 8 clicks instead. The measured offset is stored per device.

**Honest feedback rules:**
- When the signal is unclear, the app says "I couldn't hear that clearly, try again or use the self-check." It never marks the learner wrong on a bad signal.
- Every mic check has a self-rating fallback, and mic results only count when the signal is confident.

**Permissions:**
- **macOS:** `NSMicrophoneUsageDescription` in Info.plist, plus the hardened-runtime entitlement `com.apple.security.device.audio-input`. The OS asks the user once.
- **Windows:** the mic must be allowed under *Settings → Privacy → Microphone → Let desktop apps access your microphone*. The app detects a blocked mic and shows those steps.

---

## 6. Local Storage, Profiles, and Backups

- **Database:** one SQLite file in the OS app-data folder (on macOS under `~/Library/Application Support/`, on Windows under `%APPDATA%`). The schema matches the learner-data model in [01-App-Outline.md §8](01-App-Outline.md), with numbered migrations.
- **Profiles:** many per computer, no passwords, an optional PIN (stored hashed). Profile settings include handedness, labels, Playful/Clean look, text size, input device, and latency.
- **Recordings:** optional WAV takes saved next to the database, referenced by path.
- **Backup:** *Profile → Export* writes a single `.fretstudy` file (a zip with that profile's rows plus recordings). *Import* restores it on any Mac or PC. This is how learners move to a new computer, because nothing is in the cloud.
- **No network access** in the app at all. (Optional "check for updates" can be decided later. It would be the only network call, and it would be opt-in.)

---

## 7. UI Architecture

- **Screens:** Profile picker → First-time setup → Placement → Path map → Lesson player (Warm-up → Learn → See & Hear → Try → Play → Check) → Practice tools → Songbook → Library → Settings.
- **Widgets** draw from data with egui's painter (vector shapes, so they're crisp at any size):
  - `ChordBox`, `NeckMap` (any fret range, label mode: fingers / notes / intervals, color mode: finger / note-role, left-handed mirroring), `TabStaff` (color-matched numbers, rhythm stems), `StrumGrid` (solid/ghost/accent arrows, 8/16/triplet grids), `NumberWheel`, `StaffNotation` (Bravura), `TunerNeedle`, `PitchMeter`, `TimingMeter`.
  - Every widget can play its own content (click a dot, chord, or play button) and follows the shared transport clock for playheads and animations.
- **Theme:** Okabe–Ito palette constants. Root = rounded square, blue note = diamond. Light and dark variants. Playful vs. Clean looks change spacing, illustration size, and celebration effects.
- **Accessibility:** AccessKit labels on all controls, a text description for every diagram (generated from the diagram data), keyboard navigation, and adjustable text size.
- **Performance targets:** 60 fps UI, under 20 ms audio output latency, and less than 15% of one CPU core during mic analysis.

---

## 8. Testing

- **theory / content:** unit tests plus `cargo xtask validate` (see §3).
- **Synth ↔ detectors loop:** generate known notes and chords with the synth, then run them through the detectors. Pitch, chroma, and onset detection must pass this before any real-audio testing.
- **Real guitar test set:** a small library of real recordings (acoustic, electric, nylon; laptop mic and USB mic; quiet and noisy room), with correct and deliberately wrong takes. **This needs real recordings.** You or beta testers record about 30 minutes of guided takes (the app will include a "test recorder" mode for this).
- **UI:** snapshot images of each widget for visual regression, plus manual test passes on a Mac and a Windows PC.

---

## 9. Packaging and Distribution

- **macOS:** a universal binary (Apple Silicon and Intel) in a `.dmg`. To open without security warnings, it must be **code-signed and notarized** with an Apple Developer ID. That requires the Apple Developer Program (paid yearly membership).
- **Windows:** an `.msi` or setup `.exe`. Code signing with a certificate is strongly recommended. Without it, Windows SmartScreen shows "unknown publisher" warnings.
- Building Windows installers is easiest on Windows (or in CI on a Windows runner). Mac builds need macOS.
- Download size target: under 30 MB (no audio or video files, which is a big benefit of generated sound).

**How to build a release (all scripts live in `fretstudy/scripts/`):**
- **Mac:** `scripts/package-macos.sh` builds a universal app (Apple Silicon + Intel) and wraps it in `dist/FretStudy-<version>.dmg` (about 18 MB). With no setup it's signed ad hoc, so other Macs show a warning (right-click → Open gets past it). For a warning-free public release, set `FRETSTUDY_SIGN_IDENTITY` to your "Developer ID Application" certificate and `FRETSTUDY_NOTARY_PROFILE` to a saved `notarytool` login; the script then signs with the hardened runtime (with microphone permission), notarizes, and staples.
- **Windows:** on a Windows PC with Rust and Inno Setup 6, run `scripts\package-windows.ps1`. It builds `FretStudy.exe` (icon and version info embedded, no console window) and `dist\FretStudy-<version>-setup.exe`, a per-user installer that needs no admin password. Set `FRETSTUDY_SIGN_CERT` and `FRETSTUDY_SIGN_PASSWORD` to sign both with `signtool`. **The Windows build hasn't been run yet. It needs a Windows PC.**
- **Icons:** `scripts/make-icons.sh` redraws the icon (a C chord in the finger colors) and rebuilds `assets/FretStudy.icns`, `fretstudy.ico`, and the window icon.

---

## 10. Build Milestones

| # | Milestone | Result you can try |
|---|---|---|
| M0 | Toolchain + workspace + `theory` crate with tests | `cargo test` proves the note/chord/scale math |
| M1 | Synth + `ChordBox`/`NeckMap` widgets in a demo window | Click a chord and hear it strummed. Click a neck dot and hear the note |
| M2 | Mic input + **Tuner** + setup/latency calibration | A working tuner (Lesson L0.4) |
| M3 | **Vertical slice:** profiles, SQLite, lesson player, content format, validator, Level 0 + Level 1 content | A first-time user can create a profile and complete Levels 0–1 |
| M4 | Placement assessment (Stages A–C) + path map with Placed Out status | Full onboarding flow |
| M5 | Chord check, rhythm/onset check, `TabStaff`, `StrumGrid`. Levels 2–4 content | Real strumming and chord feedback |
| M6 | Practice tools (metronome, One-Minute Changes, Tempo Ladder, Fretboard Explorer, Ear Gym, spaced review). Levels 5–6 content (bends, vibrato, swing checks) | The Core Path is complete |
| M7 | Level 7 + Style Tracks, Song Builder, staff notation, export/import, accessibility pass, packaging and signing | Release candidate |

**Progress (as of Oct 13, 2026):** M0, M1, and M2 are built in `fretstudy/` (52 automated tests passing).
- M0: the `theory` crate checks every curriculum chord, CAGED shape, barre family, and pentatonic box automatically.
- M1: Chord Explorer (all curriculum chords, Finger/Note-job colors, finger/note/interval labels, left-handed mode, strum down/up, string-by-string) and Neck Explorer (any key/scale, pentatonic shapes 1–5, click to hear, playheads).
- M2: Tuner (auto or per-string, reference tones, level meter, device picker) and Sound Check (output test, mic test, latency calibration with a strum-along mode for headphones).
- Fonts: the app currently uses fonts already installed on the computer (Arial Unicode/Arial Rounded on Mac, Segoe UI on Windows) for ♯/♭ symbols, with an ASCII fallback. Bundling Atkinson Hyperlegible + a music font is planned before release.
- Settings (like the measured latency) aren't saved between runs yet. That arrives with the SQLite store in M3.

**M3 and M4 (built Oct 13, 2026):**
- Content format (RON) and a validator. Tests check prerequisites, chord ids, "name this chord" answers (against the real notes), chords played only after they're taught, 5th-grade reading level, and the placement bank.
- Local SQLite storage: profiles (optional PIN), settings, progress, Check It results, practice minutes, chord-change scores.
- Profile picker, welcome, lesson player (Learn → activities → Check It), and the path map with Ready / In progress / Done / Placed out / Locked / Coming soon.
- Levels 0–1 fully playable (17 lessons). Levels 2–7 are on the path as "coming soon."
- Activities: guitar-parts game, find-the-string game, posture checklist, tune all six, chord string-by-string check (mic), strum-along with mic timing, one-minute changes (mic counts strums), play-the-tab with mic note following, ear games, tap the beat.
- Placement quiz (Stages A–C), tested against simulated learners at every level. It never needs more than 10 questions after the 2 profile questions.
- 86 automated tests, including two end-to-end flows (completing a first lesson, and placing a strong player).

**M5 (built Oct 13, 2026):**
- Back buttons: the placement quiz can undo the last answer (the adaptive quiz replays the earlier answers). Check It can step back to review earlier questions; answers stay locked because they're revealed right away.
- Strummed-chord checking: the mic verifies the exact shape being held. It looks for each string's note (or its overtones for low strings on small mics), and flags strong sounds the chord can't explain. Tested on synthesized strums: 11 correct chords pass (including barre chords); Am-for-C, Em-for-E, and Dm-for-D are caught; a dead string is named by number. **Not yet tested with a real guitar and microphone.**
- New rhythm patterns: boom-chuck (bass notes + strums), 6/8 fingerpicking, and the palm-muted chug. Grids show bass and pick slots.
- New graphics: piano strip, musical alphabet, major-scale staircase, chord-number wheel (click to hear), note-length timeline, and a capo view on the neck.
- New activities: Note Hunt (click or play the named note) and Chord Check (strum each chord; the mic checks every string).
- Levels 2–4 written: 37 lessons, all validated (chord spelling, chords used only after they're taught, reading level, prerequisites). Levels 0–4 now hold 54 playable lessons.
- Content notes (changes from the curriculum doc, to keep every note accurate and teachable):
  - L2.10 plays the Jingle Bells chorus (first two lines, strings 1–2).
  - L2.11 (Oh, Susanna) is chords only, because its melody needs notes not yet taught.
  - L3.2, L3.7, and L3.8 use practice progressions or melodies instead of "Red River Valley," "Aura Lee," and "Simple Gifts."
  - L3.13 includes the full "Saints" melody.
  - L4.9 (capo) has no play-along, because the guide audio would play in the wrong key while a capo is on.

**M6 (built Oct 13, 2026):**
- Lead-guitar sound: tab plays hammer-ons, pull-offs, and slides without a new pick, bends glide to their target, and vibrato wobbles the held note. Tab shows slur arcs and h / p / / / b / ~ marks.
- Mic checks for lead technique: a bend meter (fretted note → goal, ±25 cents counts as in tune), a vibrato meter (wobbles per second and width, with a live pitch line), and legato note following, so hammer-ons and pull-offs count without a new pick. Swing is measured where the off-beats land. All of these are tested on synthesized guitar only. **Not yet tested with a real guitar and microphone.**
- Rhythm: swing and boogie patterns (off-beats land two-thirds of the way through the beat), 16th-note and funk grooves, and fingerstyle p-i-m-a and Travis patterns. Grids show p / i / m / a letters and boogie 5th/6th pairs.
- New graphics: scale boxes in note-job colors (pentatonic shapes 1–5, blues ♭5 diamonds), the CAGED chooser (click a shape to light it and hear it), and the 12-bar chart with a shuffle playback that lights each bar.
- New activities: Bend Check, Vibrato Check, Jam (a backing groove with the scale map; the mic lights each note you play and scores the share that's in the scale), and four new ear games (major/minor 3rds, 4ths/5ths, progressions, straight/swing).
- Practice screen: Today's Session (tune, review, changes, next lesson, ear gym, recent practice bars), Review, Metronome (40–200 BPM, 2/3/4/6 beats per bar), One-Minute Changes with any two chords you've learned, and Ear Gym (games unlock as lessons that use them are finished).
- Spaced review: finishing a lesson schedules a review quiz (three of its Check It questions) after 1 day, then 3, 7, and 21 days, then every 60. Missing one brings it back the next day. Stored in a new `reviews` table (database version 2).
- Levels 5–6 written: 26 lessons, all validated. Levels 0–6 (the whole Core Path) now hold 80 playable lessons. Level 7 is still "coming soon."
- New library chords: blues 7th barres (E-shape and A-shape, L5.12) and top-string triads (L6.6).
- Content notes (changes from the curriculum doc):
  - L5.2 uses a minor backing (Am, Dm, Em) because the 12-bar blues comes in L5.3.
  - The "fill in the 12-bar grid" check is a multiple-choice quiz for now. Recording and self-review of a solo are a checklist.
  - L6.6 strums C–F–G with top-string triads near frets 5–8 (C root position, F and G with the 5th on the bottom).
  - L6.13 plays the Canon strummed and fingerpicked (arrangement levels 1–2). The triad and melody levels come with the Song Builder in M7.
- 102 automated tests.

**M7 (built Oct 13, 2026):**
- **Level 7 written** (12 lessons): 7th chords and their movable shapes, chord-tone soloing, mixing major and minor pentatonic, modes, minor keys and harmonic minor, color chords, guest chords and walking bass, songs by ear, expression, songwriting, and the "My Song" capstone. All 92 core lessons (Levels 0–7) are now playable.
- **Style tracks written** (56 lessons): Blues (9), Rock (9), Fingerstyle & Folk (8), Jazz (9), Classical (8), Alternate Tunings & Songwriting (7), and Tone & Gear (6), each ending in a capstone. They appear under "Level 8 · Style Tracks" on the path. Tone & Gear opens after Level 2.
- **Alternate tunings:** a lesson can name a tuning (Drop D, Open D, Open G, DADGAD). While it's open, everything follows it: chord boxes, the tuner, note checks, and playback.
- **Song Builder** (new 🎼 Songs tab): pick a key (major or minor, triads or 7ths), click chord numbers into verse, chorus, and bridge, add the guest chords ♭7 and 4m, set the form, and play it back with drums in any feel. Songs are saved per profile. Charts export as printable HTML (print to PDF) or plain text. Every chord in every key has a playable shape (tested).
- **Recording:** record yourself in lessons or over your song. Takes are saved as WAV files next to the progress database, then played back in the app or shown in Finder/Explorer.
- **New activities:** chord-tone jam (the current chord's notes light up, and beat-1 landings are scored), Expression (soft/loud/staccato takes compared by loudness and ring time), Song Detective (tap a progression's numbers by ear, with a slow-down option), Song Builder tasks, and four new ear games (maj7 vs 7, Dorian vs Aeolian, Mixolydian vs Ionian, minor v vs major V).
- **New graphics:** the chord-symbol decoder (type F♯m11 and see its recipe, notes, a shape, and hear it), 7th-chord and minor-key number wheels, and a two-scale comparison neck.
- **Standard notation:** a treble-clef staff with key signatures, note lengths, ledger lines, and bar-aware accidentals, lined up note for note above the tab. Turn it on with "Standard notation" in the diagram settings; classical lessons always show it. 3/4 time is supported (Minuet in G).
- **Backups:** Settings → *Back up this profile…* writes a `.fretstudy` file (a zip of the profile's settings, progress, reviews, songs, and recordings). The player screen's *Import a backup…* restores it on any computer. If a player with the same name already exists, the import is renamed and its PIN is cleared (the PIN is tied to the name).
- **Accessibility:** every diagram gives screen readers a text description built from its data, for example "String 6: fret 3, finger 2 (G)". The left and right arrow keys move between lesson steps, and the guitar-parts game can be played from a list instead of the picture.
- **Synth fix:** damped strings now stop in about 80 ms (they used to fade for over a second), so muted and staccato notes sound right.
- **Packaging:** app icon, universal Mac app + DMG, Windows installer script, and signing and notarization hooks (see §9).
- 116 automated tests.
- **Still to do:**
  - Real-guitar testing of the mic checks.
  - A Windows test build.
  - Apple Developer ID and Windows code-signing certificates for warning-free installs. These are paid accounts only you can set up.
  - Bundled fonts (the app still uses fonts already on the computer).
  - Rhythm-aware beaming in notation.
  - Real double stops in tab.
  - The "trade fours" mode from the Level 5 doc.

**Feedback round (Oct 2026):**
- Better guitar sound (body resonance, stereo, room reverb, pick attack; see §4).
- Quiz feedback: a wrong answer now shows the right answer and why. All 388 missing explanations were written.
- Guitar-parts and string games wait for "Next" after each right answer, so the next target isn't auto-highlighted.
- Progress ring caption fits inside the ring.
- Lessons expanded: every lesson now has at least 2 Learn steps with pictures, "Watch out" tips, and a weekly practice plan (enforced by the validator).
- Chords: a "Find any chord" picker (22 chord types in 12 keys) and a "Ways to play it" panel with common shapes, every triad and 7th-chord inversion on each string set, and movable shapes up the neck. Voicings are generated and theory-checked by `theory::voicings`.
- Neck Explorer uses a light maple fretboard.

**Recommended first step:** M0 → M1 → M2, which means getting a tuner and a chord you can click and hear working on this Mac before building the rest.

---

## 11. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Laptop mics in noisy rooms give unreliable results | Confidence thresholds, "couldn't hear clearly" messages, self-rating fallback, a recommendation for USB mics or audio interfaces in setup |
| Acoustic vs. electric vs. nylon sound very different | Detectors use pitch and pitch-class energy (instrument-independent). The real-guitar test set covers all three |
| Synthesized guitar may sound "plasticky" | Karplus–Strong with body filters sounds convincingly plucked. A sampled-guitar option could be added later if needed |
| egui looks "developer-ish" by default | A custom theme (fonts, rounded shapes, spacing, illustrations) from M1 onward |
| Signing and notarization costs and setup | Plan for an Apple Developer membership and a Windows code-signing certificate before public release |
| Content volume (141 lessons) | Diagrams are generated from data. Lessons are text files. The validator catches mistakes early |
