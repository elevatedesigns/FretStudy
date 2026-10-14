# FretStudy Placement Assessment

**Goal:** place each new learner at the right **level and unit** in 3–5 minutes, with **no more than 12 questions**.
**Principles:** short, picture-based, never scary. True beginners skip almost everything.

---

## 1. Flow

```
Stage A: Quick Profile (2 questions)  ──►  Starting guess (Level 0–7)
        │
        ├── "Never played" + nothing checked  ──►  Start at Level 0. Done.
        │
Stage B: Adaptive Level Search (6–8 questions)
        │   Ask 2 checkpoint items from the guessed level.
        │   2/2 right → move UP (jump 2 levels if guess ≤ 3, else 1)
        │   1/2 right → ask a 3rd tie-breaker item from the same level
        │   0/2 right (or 1/3) → move DOWN 1 level
        │   Stop when level N is passed and level N+1 is failed (adjacent).
        │
Stage C: Pinpoint (2 questions inside level N+1)
        │   Unit-level items ("chords" vs "rhythm" vs "notes" vs "theory")
        │
Result: Start at the first unit of level N+1 that wasn't passed.
        Everything before it = "Placed Out."
```

**Passing a level** = answering its checkpoint items correctly. **Hands-on items** (microphone) are optional. If the learner skips the mic, the matching self-rating item is used instead, with half weight in the scoring.

### Pseudocode
```js
function place(profile) {
  let guess = startingGuess(profile);          // Stage A
  if (guess === 0 && profile.neverPlayed) return { level: 0, unit: first(0) };

  // Checkpoints exist for Levels 0–7; "failing" Level 8 is implied (it has no checkpoint).
  let highestPassed = -1, lowestFailed = 8, asked = 0;
  let level = guess;
  while (lowestFailed - highestPassed > 1 && asked < 9) {
    const r = askCheckpoint(level);            // 2 or 3 items
    asked += r.count;
    if (r.passed) { highestPassed = Math.max(highestPassed, level);
                    level = Math.min(lowestFailed - 1, level + (level <= 3 ? 2 : 1)); }
    else          { lowestFailed = Math.min(lowestFailed, level);
                    level = Math.max(highestPassed + 1, level - 1); }
    if (level <= highestPassed || level >= lowestFailed) break;
  }
  const target = Math.min(highestPassed + 1, 8);
  const unit = pinpoint(target);               // Stage C: ≤ 2 items
  markPlacedOut(allLessonsBefore(target, unit));
  return { level: target, unit };
}
```

---

## 2. Stage A — Quick Profile

**Q1. How long have you been playing guitar?**
Never · Less than 6 months · 6 months to 2 years · More than 2 years

**Q2. Which of these can you do *without stopping to think*?** (tap all that apply. Each option has a small picture.)

| Option | Points to level |
|---|---|
| Tune my guitar | 1 |
| Play G, C, D, Em, and Am | 2 |
| Strum along with a song without stopping | 3 |
| Read guitar tab | 2 |
| Play an F barre chord or Bm | 4 |
| Play the minor pentatonic scale | 5 |
| Make up a solo over a blues | 5 |
| Play scales and chords all over the neck (CAGED) | 6 |
| Fingerpick a pattern (like Travis picking) | 6 |
| Explain what a ii–V–I or a mode is | 7 |

**Starting guess** = the highest level where the learner checked **at least half** of that level's options (and all lower levels' options have at least one check). Experience caps it: "Never" → 0, "< 6 months" → max 3, "6 mo – 2 yr" → max 5, "2+ years" → no cap.

---

## 3. Stage B — Checkpoint Item Bank

Each level has 6–8 items. The app picks 2 at random (from different threads) and keeps 1 as a tie-breaker. Items marked 🎤 use the microphone. Each has a 🙋 self-rating fallback.

### Level 0 checkpoint — Get Ready
| ID | Type | Item | Answer |
|---|---|---|---|
| P0-1 | Picture MCQ | A chord box with a dot on string 5, fret 2 and an "X" over string 6. "What does the X mean?" | Don't play that string |
| P0-2 | MCQ | "What is the name of string 2 (second-thinnest)?" | B |
| P0-3 | Tab read | Tab: a `3` on the second line from the top. "Which string and fret?" | String 2, fret 3 |
| P0-4 | 🎤 / 🙋 | "Play your open strings from thickest to thinnest" / "I can tune my guitar myself" | E A D G B E in tune |
| P0-5 | MCQ | "Which finger is finger 3?" | Ring finger |

### Level 1 checkpoint — First Strums
| ID | Type | Item | Answer |
|---|---|---|---|
| P1-1 | Picture ID | Chord box `0 2 2 0 0 0`. "Name this chord." | E minor |
| P1-2 | Picture ID | Chord box `x x 0 2 1 2`. "Name this chord." | D7 |
| P1-3 | Ear | Two chords play. "Which one sounds minor (sadder)?" | (Em vs G) |
| P1-4 | 🎤 / 🙋 | "Strum G (easy or full) four times on the click" / "I can switch between two chords without stopping" | Steady at 70 BPM |
| P1-5 | Tab + ear | A short open-string riff plays with 3 tab options. "Which tab matches?" | — |

### Level 2 checkpoint — Open Chord Foundations
| ID | Type | Item | Answer |
|---|---|---|---|
| P2-1 | Picture ID | Box `x 0 2 2 1 0`. Name it. | Am |
| P2-2 | Picture ID | Box `x x 0 2 3 1`. Name it. | Dm |
| P2-3 | MCQ | "Which pair of letters has NO sharp or flat between them?" (C–D, E–F, G–A, A–B) | E–F |
| P2-4 | Strum grid | "Down on the numbers, up on the…?" | "and" (&) |
| P2-5 | 🎤 / 🙋 | "Play G → C → D, one bar each, at 80 BPM" / "I can play G, C, D, Em, Am cleanly and switch in time" | — |
| P2-6 | MCQ | "To turn A major into A minor, which note changes?" | The 3rd (string 2 drops 1 fret) |

### Level 3 checkpoint — Rhythm & Reading
| ID | Type | Item | Answer |
|---|---|---|---|
| P3-1 | Picture ID | Box `0 2 0 1 0 0`. Name it. | E7 |
| P3-2 | Strum grid ear | A strum plays. "Which grid matches?" (D D U U D U vs others) | D D U U D U |
| P3-3 | MCQ | "In the key of D, the 1, 4, and 5 chords are…" | D, G, A |
| P3-4 | Neck Map | "Tap the note C on string 5 in first position." | 5:3 |
| P3-5 | MCQ | "To tune string 2 (B) to string 3 (G), you press string 3 at which fret?" | 4 |
| P3-6 | Rhythm card | A bar with a quarter rest on beat 3. "On which beat are you silent?" | 3 |

### Level 4 checkpoint — Movable Shapes
| ID | Type | Item | Answer |
|---|---|---|---|
| P4-1 | Picture ID | Barre box at 3fr, E-shape major. Name it. | G |
| P4-2 | Picture ID | Box `x 2 4 4 3 2`. Name it. | Bm |
| P4-3 | MCQ | "Capo on fret 2, you play a G shape. What chord do people hear?" | A |
| P4-4 | MCQ | "Major scale step recipe?" | W W H W W W H |
| P4-5 | MCQ | "In the key of G, what is the 6 chord?" | Em |
| P4-6 | 🎤 / 🙋 | "Play F (full barre)" / "My F barre chord rings clearly" | — |
| P4-7 | Neck Map | "Tap C on string 6." | 6:8 |

### Level 5 checkpoint — Lead Guitar Basics
| ID | Type | Item | Answer |
|---|---|---|---|
| P5-1 | Neck Map | Shows Box 1 at fret 5 with one dot missing. "Tap the missing note." | (varies) |
| P5-2 | MCQ | "The 12-bar blues uses which chords?" | I, IV, V |
| P5-3 | Tab symbols | "`7b9` means…" | Bend from fret 7 up a whole step (to the pitch of fret 9) |
| P5-4 | MCQ | "The blues scale adds what note to the minor pentatonic?" | ♭5 |
| P5-5 | Ear | "Straight or shuffle feel?" | (audio) |
| P5-6 | 🎤 / 🙋 | "Play the A minor pentatonic up and down" / "I can solo over a 12-bar blues" | — |

### Level 6 checkpoint — The Whole Neck
| ID | Type | Item | Answer |
|---|---|---|---|
| P6-1 | MCQ | "What is the relative minor of G major?" | E minor |
| P6-2 | MCQ | "Order of CAGED shapes going up the neck?" | C–A–G–E–D |
| P6-3 | Neck Map | Shows pentatonic shape 2 (A minor). "Which shape number is this?" | 2 |
| P6-4 | MCQ | "In fingerstyle, which finger is 'm'?" | Middle |
| P6-5 | Inversion ID | Notes E-G-C (low to high). "Which inversion of C?" | 1st inversion |
| P6-6 | Strum grid | "How do you count 16th notes?" | 1 e & a |

### Level 7 checkpoint — Harmony & Expression
| ID | Type | Item | Answer |
|---|---|---|---|
| P7-1 | MCQ | "Which diatonic chord is naturally a dominant 7th?" | V7 |
| P7-2 | MCQ | "Dorian is like natural minor but with a raised…?" | 6th |
| P7-3 | Spell | "Spell Dm7." | D F A C |
| P7-4 | Ear | "Major 7 or dominant 7?" | (audio) |
| P7-5 | MCQ | "In A minor, why is the E chord often major?" | Harmonic minor (raised 7th, G♯) |

Passing Level 7 → the learner starts at **Level 8** with all core levels Placed Out, and is asked which Style Track to begin.

---

## 4. Stage C — Pinpoint Items (inside the start level)

Ask up to 2 items, one per thread, in order: **Chords → Rhythm → Notes → Theory**. The first thread with a missed item becomes the starting unit. Threads that are passed are marked Placed Out for that level only, so learners keep the level's song and other threads.

Example: placed into Level 4.
- Pinpoint 1 (Chords): "Name this power chord: root at string 6, fret 5." → A5 ✅ → power-chord lessons L4.3–L4.4 Placed Out.
- Pinpoint 2 (Chords): "Bm box?" ❌ → start at **L4.5 (Mini-F / barre prep)**.

---

## 5. After Placement

- Results screen: "**You're starting at Level 4 · Barre Chords.** We marked 38 lessons as known. Tap any of them to review."
- Placed Out lessons show a blue ✓ and stay fully open. Completing one later turns it green.
- **Safety net:** if the learner fails 2 Check-Its in their first week, show: "Want a quick 10-minute refresher of [prerequisite unit]?" Offer it, never force it.
- **Retake** is available from Profile anytime (it only moves the start forward or back. It never erases Completed lessons).

## 6. Measuring Quality (no telemetry)
The app sends no data anywhere, so placement quality is tuned with **beta testers** instead of analytics:
- Testers use a debug build that writes a local log of placement answers and their first 3 Check-It results. They choose to send the log file by email.
- Target: 70–85% of placed learners pass their first 3 Check-Its. Lower means placement is too high; higher with fast completion means it's too low.
- Watch where testers hesitate or quit, to keep the quiz short and friendly.
- Rotate items, and retire items that most testers get right or wrong regardless of level (low discrimination).
- **On-device self-correction:** the app adjusts for each learner (the safety-net refresher in §5) without sending anything.
