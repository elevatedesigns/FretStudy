# FretStudy — App Outline & Teaching Plan

*A free desktop app for Mac and Windows, written in Rust, that teaches guitar from "never held one" to "can improvise and play in any style," one building block at a time. It works fully offline, and all data stays on the learner's computer.*

> **Build decisions (confirmed):** generated sound plus animated diagrams (no recorded video) · microphone checks included · native Rust desktop app for macOS and Windows, with no web app · all data stored locally · audience is kids and adults · public-domain and original songs only. Technical details are in [04-Technical-Architecture.md](04-Technical-Architecture.md).

---

## 1. The Big Idea

Most guitar books teach the same things, but in different orders, with different words, and usually with black-and-white pictures. FretStudy takes the best order from ~50 guitar books and turns it into **one clear path** where:

1. **Every lesson uses only things you already learned**, plus *one* new thing.
2. **Every idea is shown, not just told.** Colorful fretboards, numbered fingers, animated strums, and sound on every diagram.
3. **Every lesson ends with real music.** You play something that sounds good, not just an exercise.
4. **Everyone starts in the right place.** A short placement quiz skips what you already know. You can still go back.

### Who it's for
- Total beginners (most users)
- "Campfire players" who know a few chords but are stuck
- Self-taught players with gaps (they can play but don't know *why*)
- Returning players who want a refresh

### Kids and adults, one app
- **One plain-language voice for everyone.** The "Explain it like this" boxes stay at a 5th-grade reading level, which helps kids and is quick for adults. Each one has an optional **"More detail"** fold-out with the fuller explanation (exact theory terms, the "why").
- **Profiles per person.** A family can share one computer: each person gets their own profile, avatar, progress, and settings. There are no passwords. An optional 4-digit PIN keeps a sibling from playing your lessons.
- **Look and feel setting:** *Playful* (bigger art, celebration animations, stickers for streaks) or *Clean* (calmer visuals). The default is set by the age band picked when the profile is created (Kid / Teen / Adult), and it can be changed anytime.
- **Hand size:** a "small hands" tip appears on stretch-heavy lessons (barre chords, B7) and suggests a capo higher up the neck or a smaller guitar.
- **Privacy by design:** no accounts, no internet connection required, no analytics, and no data leaves the computer. This keeps the app simple and safe for children. If online features are ever added (backups, sharing, crash reports), revisit children's privacy rules (such as COPPA in the U.S.) before shipping them.

### What "done" looks like
A learner who finishes the **Core Path (Levels 0–6)** can:
- Tune, hold, and care for a guitar
- Play all common open chords, barre chords, and power chords, and change between them in time
- Strum dozens of rhythm patterns, including swing/shuffle and 16th-note grooves
- Read tab, chord charts, rhythm notation, and basic standard notation
- Find any note on the neck
- Understand keys, scales, chord numbers (I–IV–V), and why songs work
- Play pentatonic and blues scales across the whole neck and improvise a solo
- Fingerpick basic patterns
- Then pick a **Style Track** (Blues, Rock, Fingerstyle/Folk, Jazz, Classical, Alternate Tunings & Songwriting)

---

## 2. Teaching Philosophy (what the research and the source books agree on)

| Principle | What it means in the app | Source support |
|---|---|---|
| **Play music on day one** | Lesson 1.1 has learners strumming a real two-chord song using 1-finger "easy" chords. | *Learning to Play Guitar* (Great Courses), Lesson 1, "Play a Song in 60 Seconds"; *Alfred's* 3-string chords |
| **Spiral, don't stack** | Each level has the same 5 "threads" (Technique, Chords, Rhythm, Notes/Melody, Theory & Ear). Each thread advances a little every level instead of finishing one topic before starting the next. | Great Courses uses 5 modules per lesson; Learn & Master interleaves reading, chords, and theory |
| **Small, frequent practice** | Lessons are 10–20 minutes. A daily "Practice Session" builder targets 15–30 minutes. | Learn & Master ("at least 15 minutes a day"); *Guitar Tips You'll Hate to Miss* (20–30 min daily, short sessions, journal); *Speed Building Handbook* (10-min metronome plans) |
| **Slow and correct beats fast and sloppy** | Every exercise starts with a slow metronome tempo and an "earn +5 BPM" button. | *Speed Building Handbook*; *Guitar Exercises For Dummies* |
| **Relaxation and minimal motion** | Technique lessons repeat "don't squeeze," "fingertips behind the fret," "thumb behind the neck." | *Guitar Tips* (Principle of No Tension, Streamline Motions); Great Courses Lesson 1 |
| **Theory is a tool, not a test** | Theory appears only when a learner needs it for the next song (e.g., the major scale shows up right before keys and barre chords). | *Music Theory for the Music Industry*; *Guitar Scales in Context* ("a phrase book, not a dictionary") |
| **Ear first, then eyes** | Sing it, hear it, then play it. Every lesson has a short listening task. | *Train Your Ear*; *Guitar Tips* (Aural Skills) |
| **Shapes anchor to chords** | Scales are taught "around" chord shapes (CAGED), so learners always know where they are. | *Guitar Scales in Context*; *Progressions* series |
| **Public-domain songs** | All built-in songs are public domain or original, so the app can stay free without licensing. | The method books use folk and public-domain tunes for the same reason |

---

## 3. App Structure (Information Architecture)

```
FretStudy
├── Choose or create a profile (local) ──► First-time setup (mic check + latency test) ──► Placement Assessment ──► "Your Path" map
│
├── Your Path (home)
│   ├── Level 0  Get Ready
│   ├── Level 1  First Strums
│   ├── Level 2  Open Chord Foundations
│   ├── Level 3  Rhythm & Reading
│   ├── Level 4  Movable Shapes
│   ├── Level 5  Lead Guitar Basics
│   ├── Level 6  The Whole Neck
│   ├── Level 7  Harmony & Expression
│   └── Level 8  Style Tracks (choose any)
│        Blues · Rock · Fingerstyle & Folk · Jazz · Classical · Alt Tunings & Songwriting · Tone & Gear
│
├── Practice
│   ├── Today's Session (auto-built: warm-up → review → new skill → song)
│   ├── Tuner (microphone)
│   ├── Metronome / drum loops
│   ├── Chord-Change Trainer (1-minute changes counter)
│   ├── Fretboard Explorer (tap any note; show notes/intervals/scales)
│   └── Ear Gym (intervals, major vs minor, chord progressions)
│
├── Songbook (public-domain songs, tagged by the skills they use)
├── Chord & Scale Library (unlocks as you learn)
└── Profile (progress, streaks, practice journal, switch profile, export/import backup,
             settings: left-handed, labels, colors, Playful/Clean look, audio input device, latency)
```

### Unit of learning: Level → Unit → Lesson → Steps

- **Level**: one big phase (3–8 weeks at 20 min/day). Ends with a **Level Song** and a **Level Check**.
- **Unit**: one thread inside a level (for example, "Level 2 · Chords").
- **Lesson**: 10–20 minutes, one new idea.
- **Steps inside every lesson** (always the same, so learners know what to expect):
  1. **Warm-up** (1–2 min). Uses something from a past lesson, for spaced review.
  2. **Learn It**. A kid-simple explanation with one big graphic.
  3. **See & Hear It**. An interactive diagram. Tap a dot to hear it, and watch the animation.
  4. **Try It**. Guided practice with a metronome or backing track at a slow, a medium, and a goal tempo.
  5. **Play It**. A short song, riff, or progression using the new skill.
  6. **Check It**. A 2–4 question mastery check plus a self-rating ("I can do this without stopping").

### Lesson status (on the Path map)

| Status | Look | Meaning |
|---|---|---|
| Locked | Gray, with a lock | Prerequisites not finished |
| Ready | Bright outline | Can start now |
| In progress | Half-filled ring | Started |
| Completed | Solid green check | Passed the Check It |
| **Placed Out** | Blue check with a "skip" arrow | Marked done by the placement quiz. Fully re-openable. Shows "Review anytime." |
| Needs Review | Yellow dot | Failed a later check that depends on this skill, or not practiced in 30+ days |

Each lesson stores its **prerequisites** (lesson IDs). The Path unlocks a lesson when all of its prerequisites are Completed or Placed Out. Most prerequisites are the previous lesson in the same thread plus one or two from other threads.

---

## 4. The Placement Assessment (brief but accurate)

**Goal:** about 3–5 minutes, **at most 12 questions**, landing the learner at the right **unit** (not just the right level).

### How it works (3 stages)

**Stage A — Quick Profile (2 taps)**
1. "How long have you played?" (Never / Under 6 months / 6 months–2 years / 2+ years)
2. "Which of these can you do *without stopping to think*?" (multi-select, with small pictures)
   - Tune my guitar
   - Play G, C, D, Em, Am
   - Strum along to a song
   - Read guitar tab
   - Play an F or B♭ barre chord
   - Play the minor pentatonic scale
   - Improvise a solo
   - Fingerpick a pattern

The app turns these answers into a **starting guess** (for example, "probably Level 3").

**Stage B — Adaptive Checkpoint Questions (6–8 questions)**
- Each level has a **Checkpoint Bank** of short questions tied to that level's must-know skills. Many questions show a fretboard, chord box, or tab and ask the learner to identify it, or to play it with optional microphone checking.
- The app starts at the guessed level and asks **2 questions**:
  - **Both right**: jump up (skip a level when possible).
  - **One right**: ask a 3rd tie-breaker.
  - **Both wrong**: move down one level.
- It stops when it finds the **highest level passed** and the **lowest level failed** next to each other. That takes 3–4 rounds at most across 9 levels.
- A "Never played" learner skips Stage B entirely and starts at Level 0. No quiz anxiety for true beginners.

**Stage C — Pinpoint (2 questions)**
- Inside the first failed level, two quick questions pick the exact **unit** to start in (for example, they already know the Level 4 power chords but not barre chords).

### Placement result
- Everything **before** the start point is marked **Placed Out**.
- The results screen shows: "You're starting at **Level 4 · Barre Chords**. We marked 31 lessons as done. You can review any of them anytime."
- **Safety net:** if a learner fails two Check It quizzes in a row soon after placement, the app offers: "Want a quick refresher of Level 3 Chords?" It never forces them back.
- The quiz can be retaken anytime from Profile.

> The full question bank is in [03-Placement-Assessment.md](03-Placement-Assessment.md).

### Why this design
Self-report alone over- or under-estimates skill. Knowledge questions alone can't test hands. Combining a short self-report (to choose where to start) with adaptive picture-based checkpoint questions (to confirm) keeps the quiz short while still pinpointing the right unit. This matches how adaptive placement systems work: they ask only as many questions as needed and stop once the boundary is found. Optional mic-based "play this" items add a real hands-on check without being required.

---

## 5. Visual Language (graphics-heavy by design)

Graphics are the core of every lesson, not decoration. One consistent visual system is used everywhere, so a learner never has to relearn how to read a picture.

### 5.1 Diagram types

| Diagram | Use | Orientation & rules |
|---|---|---|
| **Chord Box** (vertical) | Single chord shapes | Nut on top (thick line). Low E string on the **left**. Dots show fingers, with a **finger number inside each dot**. `O` above a string means play it open, `X` means don't play it. Fret number label (e.g., "5fr") when not at the nut. Barre = rounded bar across strings. |
| **Neck Map** (horizontal) | Scales, note maps, chord tones, CAGED | Headstock on the left. **High E on top** so it lines up with tab. Fret numbers under the neck. Inlay dots at frets 3, 5, 7, 9, 12 (double), 15, 17. Strings labeled on the left (e B G D A E). |
| **Tab Staff** | Riffs, melodies | 6 lines, high E on top. Fret numbers **colored to match** the dot colors on the Neck Map above them. Rhythm stems shown below. |
| **Strum Grid** | Rhythm patterns | Beat boxes labeled `1 & 2 & 3 & 4 &`. Solid ↓/↑ arrows = hit the strings. Faded "ghost" arrows = hand still moves but misses the strings (teaches "keep your hand moving"). An accented beat gets a bigger arrow. |
| **Hand Pictures** | Posture, pick grip, fingerstyle | Simple illustrated hands with **finger colors** (see below). |
| **Number Wheel / Chord Family** | Keys and I–IV–V | A circle or ladder showing the 7 chords in a key, with jersey numbers 1–7, colored by major (warm) or minor (cool). |
| **Animated Fretboard** | Chord changes, slides, bends | Fingers glide from chord A to chord B. "Anchor fingers" that stay put glow and pulse. Bends show an arrow and the target note. |

### 5.2 Color system (color-blind safe)

Colors come from the **Okabe–Ito palette**, a set designed to stay distinguishable for the roughly 8% of men with color-vision differences. **Color never works alone.** Every colored dot also has a number or label inside it, and roots also have a different **shape**.

There are two color "modes," and every diagram shows a small legend saying which mode is on.

**Mode 1 — FINGER colors** (Levels 0–4, chord shapes, technique)

| Finger | Number | Color | Hex |
|---|---|---|---|
| Index | 1 | Blue | `#0072B2` |
| Middle | 2 | Green | `#009E73` |
| Ring | 3 | Orange | `#E69F00` |
| Pinky | 4 | Pink-purple | `#CC79A7` |
| Thumb (fretting, rare) | T | Gray | `#6B6B6B` |
| Open string | O | Hollow ring | — |
| Muted string | X | Dark X | — |

Fingerstyle picking hand uses letters **p i m a** (thumb, index, middle, ring) in the same order of colors, but with a white outline so it can't be confused with the fretting hand.

**Mode 2 — NOTE ROLE colors** (theory, scales, soloing; mainly Level 4 and up)

| Role | Label | Color | Shape |
|---|---|---|---|
| Root (home note) | R | Vermillion `#D55E00` | **Rounded square** |
| 3rd (major or minor) | 3 / ♭3 | Sky blue `#56B4E9` | Circle |
| 5th | 5 | Green `#009E73` | Circle |
| 7th (any kind) | 7 / ♭7 | Purple `#CC79A7` | Circle |
| Other scale notes (2, 4, 6) | 2 4 6 | Light gray `#BDBDBD` | Circle |
| Blue note (♭5) | ♭5 | Blue `#0072B2` | Diamond |

### 5.3 Labels toggle
Every Neck Map has a 3-way toggle: **Fingers** | **Note names** | **Intervals**. This lets the same picture teach "where to put my fingers," "what notes these are," and "why it sounds this way."

### 5.4 Sound on everything (generated, not recorded)
- All sound is **synthesized in the app**: a plucked-string guitar model, strums built from staggered string plucks (down-strums low→high, up-strums high→low), palm mutes, slides, bends, a metronome, and simple drum and bass backing loops. There are no audio files to record or license.
- Tap a dot to hear the note. Tap a chord box to hear it strummed.
- Every tab and strum grid has a play button with a moving playhead, plus speed control (50%, 75%, 100%).
- Animations (finger movement, strum arrows, playheads) are driven by the same clock as the audio, so the picture and the sound always line up.

### 5.4b Microphone checks
- The learner plays into the computer's built-in or USB microphone (or an audio interface for electric guitar).
- Checks: tuner, single notes and melodies, a chord's notes, string-by-string chord checks, strum timing, bend accuracy, vibrato, and swing feel.
- The mic is **always optional**. Every mic check has a self-rating fallback, so the app works without a microphone.
- What the mic can and can't judge is spelled out honestly in [04-Technical-Architecture.md §5](04-Technical-Architecture.md). For example, it can hear *whether* the right notes and timing happened, but it can't reliably tell a down-strum from an up-strum.

### 5.5 Left-handed mode
A Profile setting mirrors every diagram, photo, and animation.

### 5.6 Accessibility
- Every diagram has a text description (for example, "C major: 3rd finger on the 5th string, 3rd fret…").
- Contrast targets WCAG AA. Never color-only meaning.
- Screen-reader support through the UI toolkit's accessibility layer, plus full keyboard navigation.
- Adjustable text size.

---

## 6. Practice System

- **Today's Session**: auto-built from (a) a 2-min warm-up (chromatic 1-2-3-4 finger exercise, from Learn & Master, Great Courses, and *Guitar Exercises For Dummies*), (b) 1–2 review items from older lessons (spaced repetition: review at 1 day, 3 days, 7 days, 21 days), (c) the current lesson, and (d) a song.
- **One-Minute Changes**: count how many clean switches between two chords you can make in 60 seconds. Track your best score over time.
- **Tempo Ladder**: earn +5 BPM after 3 clean runs.
- **Practice Journal**: a one-line note after each session. The *Guitar Tips* book stresses tracking practice and setting one focus per day.
- **Streaks and gentle goals**: a weekly goal (for example, 5 days × 15 minutes). Set weekly, not monthly, as advised in *Guitar Tips*.

---

## 7. Songbook (licensing-safe)

Built-in songs are **public domain or written for FretStudy**, each tagged with the chords and skills it needs, so the app can say "You can play this now!" Examples by level:

| Level | Songs (public domain) |
|---|---|
| 1 | "Row, Row, Row Your Boat" (1 chord), "Skip to My Lou" (2 chords), "Down in the Valley" (2 chords) |
| 2 | "Oh, Susanna," "Amazing Grace," "Ode to Joy" (melody), "Jingle Bells" (melody + chords) |
| 3 | "When the Saints Go Marching In," "Red River Valley," "Home on the Range," "Aura Lee," "Simple Gifts" |
| 4 | "House of the Rising Sun" (Am, C, D, F, E), "Scarborough Fair," "Greensleeves" |
| 5 | Original 12-bar blues jams; "Frankie and Johnny" (trad. blues) |
| 6 | "Canon in D" (simplified), "Wabash Cannonball," "Shenandoah" |
| 7+ | Original jam tracks in every style; Bach "Minuet in G" (classical track) |

For modern songs, learners can use the **Chord Chart Reader**: they type a chord progression from any song chart, and the app shows the shapes and the number system (I–V–vi–IV). The app doesn't host copyrighted lyrics or tabs.

---

## 8. Data Model (for the build)

There are two kinds of data:

**1. Course content** (the same for everyone). It ships inside the app as human-editable data files, versioned with the code:
```
Level(id, number, title, summary, est_weeks)
Unit(id, level_id, thread[technique|chords|rhythm|notes|theory_ear|song], order)
Lesson(id "L2.4", unit_id, title, minutes, prerequisites[lesson_id], skills[skill_id], steps[...])
Skill(id, name, description)           // e.g., "chord.open.G", "rhythm.eighths.down-up"
Diagram(type, data)                    // chord box / neck map / tab / strum grid
AssessmentItem(id, level, thread, skill_ids[], kind[mcq|identify|play_mic|self_rate], answer, media)
Song(id, title, public_domain:true, chords[], skills[], key, bpm, chart, tab)
```

**2. Learner data** (per profile). It's stored in a local SQLite database on the computer:
```
Profile(id, name, avatar, age_band[kid|teen|adult], pin_hash?, created_at,
        settings{handedness, labels, look[playful|clean], input_device, latency_ms, text_size})
Progress(profile_id, lesson_id, status[completed|placed_out|in_progress|needs_review], best_score, updated_at)
// "locked" and "ready" are computed from prerequisites, not stored
CheckResult(profile_id, lesson_id, item_id, score, used_mic, taken_at)
PracticeLog(profile_id, date, minutes, items[], note)
ChangeScore(profile_id, chord_a, chord_b, count, date)
TempoBest(profile_id, exercise_id, bpm, date)
ReviewSchedule(profile_id, skill_id, next_due, interval_days)
Recording(profile_id, lesson_id, file_path, created_at)   // optional saved takes (WAV)
```

Diagrams are stored as **data**, not images (for example, a chord as `strings: [x, 3, 2, 0, 1, 0], fingers: [_, 3, 2, _, 1, _]`). One renderer then draws every chord, scale, and tab in the consistent style, supports left-handed mirroring, label toggles, and audio, and lets the curriculum grow without new artwork. The same data drives the synthesizer (what to play) and the mic checker (what to listen for).

---

## 9. Curriculum at a Glance

**Core Path: Levels 0–6. Expression: Level 7. Electives: Level 8.**
Times assume about 20 minutes a day.

| Level | Name | Weeks | You'll be able to… | Level Song |
|---|---|---|---|---|
| 0 | **Get Ready** | 1 | Name guitar parts, hold it, tune it, read a chord box and tab, use finger numbers | Tune all 6 strings |
| 1 | **First Strums** | 2–3 | Play 1- and 2-finger easy chords (Em, Em7, easy G, easy C, Asus2) with steady down-strums; pick open-string riffs | "Skip to My Lou" |
| 2 | **Open Chord Foundations** | 4–6 | Play E, A, D, Am, Dm, G, C full chords; strum eighth notes down-up; read notes on strings 1–2 | "Oh, Susanna" |
| 3 | **Rhythm & Reading** | 4–6 | Strum 4 core patterns, play 7th and sus chords, read all first-position notes, understand half and whole steps and the I–IV–V | "When the Saints Go Marching In" |
| 4 | **Movable Shapes** | 6–8 | Play power chords, palm mute, play E-shape and A-shape barre chords, find any root on strings 6 and 5, use a capo, understand the major scale and keys | "House of the Rising Sun" (full version with F barre) |
| 5 | **Lead Guitar Basics** | 6–8 | Play minor pentatonic Box 1, hammer-ons, pull-offs, slides, bends, vibrato; play the 12-bar blues and shuffle; improvise | Your first 12-bar blues solo |
| 6 | **The Whole Neck** | 8–10 | Play all 5 pentatonic shapes, major pentatonic, the CAGED system, triads, major scale positions, 16th-note strumming, Travis picking | "Canon in D" chord-melody (simplified) |
| 7 | **Harmony & Expression** | 8+ | Play 7th-chord families and diatonic harmony, solo with chord tones, use modes, recognize progressions by ear, write a song | Original song + solo |
| 8 | **Style Tracks** | ongoing | Go deep in Blues, Rock, Fingerstyle & Folk, Jazz, Classical, Alternate Tunings & Songwriting, or Tone & Gear | Track capstones |

The five threads in every level (Core Path):

| Thread | L0 | L1 | L2 | L3 | L4 | L5 | L6 |
|---|---|---|---|---|---|---|---|
| **Technique** | Posture, pick grip | Fingertip fretting, 1-2-3-4 warm-up | Clean chords, anchor fingers | Alternate picking, muting | Barre strength, palm muting | Bends, vibrato, legato | Position shifts, fingerstyle |
| **Chords** | Reading chord boxes | Easy chords | Full open majors and minors | 7ths and sus | Power chords, barres, capo | Dominant 7th barres for blues | CAGED, triads, inversions |
| **Rhythm** | Steady beat (tap) | Quarter-note down strums | Eighth-note down-up | Patterns, ties, rests | Syncopation, palm-muted 8ths | Shuffle and swing | 16th notes, accents |
| **Notes & Melody** | String names | Open-string riffs (tab) | Notes on strings 1–2 | All first-position notes | Notes on strings 6 and 5 | Pentatonic Box 1 licks | Whole-neck pentatonic and major |
| **Theory & Ear** | Musical alphabet | High/low, major vs minor (ear) | Half and whole steps | Chord families, I–IV–V | Major scale, keys, number system | Blues form, intervals | Relative minor, triads, CAGED logic |

---

## 10. How the Source Library Was Used

All 50 files in the project folder were reviewed. The table shows which books shaped which parts of the curriculum.

| Source | Used for |
|---|---|
| *Learn & Master Guitar* Lesson Book + Bonus Workshop Book (Steve Krenz) | Backbone sequence (20 sessions): open chords → sus/7ths → 6th- then 5th-string barres → strumming → fingerstyle → pentatonic → blues → techniques → power chords → 16ths → 3-notes-per-string → jazz → soloing → chord formulas. Time estimates per topic. Worksheets (keys, intervals, triads). |
| *Learning to Play Guitar: Chords, Scales and Solos* (Great Courses, 24 lessons) | 5-module lesson format; "play a song in 60 seconds" with easy G and C; practice and shifting; Travis picking; capo; DADGAD and modes order. |
| *The Only Basic Guitar Instruction Book You'll Ever Need* | Order for reading → right-hand styles (alternate bass, folk, country) → movable chords → position playing → harmony and the cycle of 4ths/5ths. |
| *Alfred's Teach Yourself to Play Guitar* | 3-string beginner chords, string-by-string note reading, bass-chord accompaniment. |
| *Learn How To Play Guitar* (classical method) | Rhythm-reading exercises, accidentals, key signatures for the classical track. |
| *Christopher Parkening Guitar Method* (scan, low resolution) | Classical track structure (rest vs. free stroke, first-position reading). |
| *Guitar Exercises For Dummies* | Warm-ups, 5 major-scale patterns, scale sequences, arpeggios, finger-independence drills. |
| *Guitar Chords For Dummies*; *All the Chords* (Kunde) | Chord library voicings and naming in all 12 keys. |
| *Music Theory for the Music Industry* (Kunde) | Kid-simple major-scale formula (W-W-H-W-W-W-H), chord "numbers" (1 major, 2 minor…), inversions, slash chords, sus chords. |
| *How I Play Guitar* (Kunde) | Inversions across the neck in common keys; creative capo use. |
| *Train Your Ear* (Kunde) | Interval ear-training sequence (compare 2 intervals at a time; sing it). |
| *Speed Building Handbook* (Kunde) | Metronome tempo ladders, 10-minute daily plans, alternate picking and finger isolation. |
| *The Tone Method* (Kunde) | Tone & Gear elective (amp + guitar, drive, compression, delay/reverb, EQ, troubleshooting). |
| *Progressions: Classics* Vols 1–9 + *Alternate Tunings* (Open D, Open G, DADGAD) | Common progressions (I–vi–IV–V, I–V–vi–IV, I–IV–V…) in all 12 keys across CAGED positions; alternate-tunings elective. |
| *Guitar Scales in Context*; *Teach Yourself to Play Guitar Scales* | Scale-around-chord-shape teaching; five positions; three-octave scales; modes with sound and feel. |
| *Spotlight Blues Guitar* (Krenz); *Mastering Electric Blues Guitar* (Riker); *Beginning Fingerstyle Blues* (Berle/Galbo) | Blues track: 12-bar, shuffles, turnarounds, the three bend spots, major/minor pentatonic mix, Mixolydian, dominant arpeggios, 8-bar and minor blues, fingerstyle blues with alternating bass. |
| *Total Rock Guitar* (Stetina) | Rock track: 4th-dyad → 5th power chords, palm muting, rests, riffs by style. |
| *Electric Guitar in 5 Months*; *lead guitar lesson guide* | Lead sequence (pentatonic → techniques → bars → power chords → tapping/sweeping); intro lead using "House of the Rising Sun." |
| *Fingerstyle Guitar For Beginners*; *Latin-American Jazz for Fingerstyle* | Fingerstyle track: p-i-m-a, thumb independence, Latin/bossa rhythms (advanced). |
| *Beginning Jazz Guitar* (Fisher); *Strategies in Jazz Guitar Improvisation* (Solstad thesis) | Jazz track: triads/inversions on string sets, 6th/7th/9th chords, harmonized scale, Roman numerals, modes. "Chunking" research supports practicing short licks as building blocks. |
| *Guitar Tips You'll Hate to Miss* | Motivation, expectations, practice planning, posture, no-tension principle, song-learning method (loop trouble spots, practice slowly). |
| Acoustic-guitar beginner e-books; *Learn Guitar in 30 Days* | Guitar anatomy, buying, stringing, common beginner mistakes, a 30-day chord-change routine. |
| *The Ultimate Guitar Sourcebook* | History and "types of guitars" sidebars (Tone & Gear elective). |

**Not usable:** *Guitar Mastery Simplified* is an empty file (0 bytes). Several Alfred/Mel Bay books are image-only scans and were read by viewing their pages. The *Progressions* books are chord-chart images, so the progression names were used and individual voicings were not transcribed.

### Outside research applied
- **Color-blind-safe palette**: the Okabe–Ito / Wong 2011 colors, with color never used alone ([R documentation](https://search.r-project.org/CRAN/refmans/see/html/palette_okabeito.html), [EU data-viz guide](https://data.europa.eu/apps/data-visualisation-guide/colour-for-categories)).
- **Fretboard diagram conventions**: root highlighted in a distinct color, interval-based coloring, nut drawn thick, low E assumed at the bottom when strings are unlabeled. FretStudy labels strings always and uses tab orientation ([fretboardgtr docs](https://fretboardgtr.readthedocs.io/en/latest/api_documentation/note_colors.html), [guitarscale.org](https://guitarscale.org/faq.html), [GuitarMasterclass](https://www.guitarmasterclass.net/wiki/index.php/Reading_Scale_Diagrams_%28lesson%29), [JustinGuitar community](https://community.justinguitar.com/t/fretboard-diagrams-for-scales/2152)).
- **Adaptive placement**: prerequisite-graph placement and minimum-question stopping rules ([PLACEments / Heffernan](https://www.neilheffernan.net/projects/pilot-projects/placements), [DFKI competency placement](https://dfki.de/web/forschung/projekte-publikationen/publikation/15606), [HAL adaptive testing paper](https://hal.univ-grenoble-alpes.fr/hal-01493787/document)).
