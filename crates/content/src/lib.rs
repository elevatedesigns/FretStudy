//! # FretStudy course content
//!
//! The curriculum (levels, lessons, steps) and the placement question bank, loaded from
//! RON files embedded in the app. Also: lesson status on the path, the placement
//! algorithm, and a validator that checks the content for mistakes.

pub mod model;
pub mod path;
pub mod placement;
pub mod readability;

#[cfg(test)]
mod validate;

pub use model::*;
pub use path::{Done, Status};

const L0: &str = include_str!("../../../content/levels/L0.ron");
const L1: &str = include_str!("../../../content/levels/L1.ron");
const L2: &str = include_str!("../../../content/levels/L2.ron");
const L3: &str = include_str!("../../../content/levels/L3.ron");
const L4: &str = include_str!("../../../content/levels/L4.ron");
const L5: &str = include_str!("../../../content/levels/L5.ron");
const L6: &str = include_str!("../../../content/levels/L6.ron");
const L7: &str = include_str!("../../../content/levels/L7.ron");
/// Level 8 style tracks (electives).
const TRACKS: &[(&str, &str)] = &[
    (
        "blues.ron",
        include_str!("../../../content/levels/tracks/blues.ron"),
    ),
    (
        "rock.ron",
        include_str!("../../../content/levels/tracks/rock.ron"),
    ),
    (
        "fingerstyle.ron",
        include_str!("../../../content/levels/tracks/fingerstyle.ron"),
    ),
    (
        "jazz.ron",
        include_str!("../../../content/levels/tracks/jazz.ron"),
    ),
    (
        "classical.ron",
        include_str!("../../../content/levels/tracks/classical.ron"),
    ),
    (
        "tunings.ron",
        include_str!("../../../content/levels/tracks/tunings.ron"),
    ),
    (
        "tone.ron",
        include_str!("../../../content/levels/tracks/tone.ron"),
    ),
];
const OUTLINE: &str = include_str!("../../../content/levels/outline.ron");
const PLACEMENT: &str = include_str!("../../../content/placement.ron");

pub struct Curriculum {
    pub levels: Vec<Level>,
    pub placement: PlacementBank,
}

#[derive(Debug)]
pub struct ContentError(pub String);

impl std::fmt::Display for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ContentError {}

fn parse<T: serde::de::DeserializeOwned>(name: &str, text: &str) -> Result<T, ContentError> {
    ron::from_str(text).map_err(|e| ContentError(format!("{name}: {e}")))
}

impl Curriculum {
    /// Parses the embedded content. Tests guarantee this succeeds for shipped content.
    pub fn load() -> Result<Curriculum, ContentError> {
        let mut levels = vec![
            parse::<Level>("L0.ron", L0)?,
            parse::<Level>("L1.ron", L1)?,
            parse::<Level>("L2.ron", L2)?,
            parse::<Level>("L3.ron", L3)?,
            parse::<Level>("L4.ron", L4)?,
            parse::<Level>("L5.ron", L5)?,
            parse::<Level>("L6.ron", L6)?,
            parse::<Level>("L7.ron", L7)?,
        ];
        for (name, text) in TRACKS {
            levels.push(parse::<Level>(name, text)?);
        }
        levels.extend(parse::<Vec<Level>>("outline.ron", OUTLINE)?);
        levels.sort_by_key(|l| l.number);
        let placement = parse::<PlacementBank>("placement.ron", PLACEMENT)?;
        Ok(Curriculum { levels, placement })
    }

    /// All lessons in path order.
    pub fn lessons(&self) -> impl Iterator<Item = &Lesson> {
        self.levels.iter().flat_map(|l| l.lessons.iter())
    }

    pub fn lesson(&self, id: &str) -> Option<&Lesson> {
        self.lessons().find(|l| l.id == id)
    }

    pub fn level_of(&self, id: &str) -> Option<u8> {
        self.levels
            .iter()
            .find(|l| l.lessons.iter().any(|x| x.id == id))
            .map(|l| l.number)
    }

    /// Position in path order (used to compare "earlier" and "later").
    pub fn order(&self, id: &str) -> Option<usize> {
        self.lessons().position(|l| l.id == id)
    }

    pub fn max_level(&self) -> u8 {
        self.levels.iter().map(|l| l.number).max().unwrap_or(0)
    }
}
