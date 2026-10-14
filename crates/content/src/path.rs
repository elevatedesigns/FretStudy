//! Lesson status on the learning path.

use crate::Curriculum;
use std::collections::HashMap;

/// What the learner has done with a lesson (stored per profile).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Done {
    InProgress,
    Completed,
    PlacedOut,
}

impl Done {
    pub fn as_str(self) -> &'static str {
        match self {
            Done::InProgress => "in_progress",
            Done::Completed => "completed",
            Done::PlacedOut => "placed_out",
        }
    }

    pub fn parse(s: &str) -> Option<Done> {
        match s {
            "in_progress" => Some(Done::InProgress),
            "completed" => Some(Done::Completed),
            "placed_out" => Some(Done::PlacedOut),
            _ => None,
        }
    }

    /// Counts as "known" for unlocking later lessons.
    pub fn satisfies(self) -> bool {
        matches!(self, Done::Completed | Done::PlacedOut)
    }
}

/// How a lesson appears on the path map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Locked,
    Ready,
    InProgress,
    Completed,
    PlacedOut,
    /// On the path, but the lesson isn't written yet.
    ComingSoon,
}

impl Curriculum {
    pub fn status(&self, id: &str, progress: &HashMap<String, Done>) -> Status {
        let Some(lesson) = self.lesson(id) else {
            return Status::Locked;
        };
        match progress.get(id) {
            Some(Done::Completed) => return Status::Completed,
            Some(Done::PlacedOut) => return Status::PlacedOut,
            _ => {}
        }
        if lesson.coming_soon {
            return Status::ComingSoon;
        }
        let unlocked = lesson
            .prerequisites
            .iter()
            .all(|p| progress.get(p).is_some_and(|d| d.satisfies()));
        match (unlocked, progress.get(id)) {
            (false, _) => Status::Locked,
            (true, Some(Done::InProgress)) => Status::InProgress,
            (true, _) => Status::Ready,
        }
    }

    /// The lesson to suggest next: the first in-progress or ready lesson in path order.
    pub fn next_lesson(&self, progress: &HashMap<String, Done>) -> Option<&str> {
        self.lessons()
            .map(|l| (l, self.status(&l.id, progress)))
            .find(|(_, s)| matches!(s, Status::InProgress | Status::Ready))
            .map(|(l, _)| l.id.as_str())
    }

    /// Completed or placed out, out of all lessons in a level.
    pub fn level_progress(&self, level: u8, progress: &HashMap<String, Done>) -> (usize, usize) {
        let Some(l) = self.levels.iter().find(|l| l.number == level) else {
            return (0, 0);
        };
        let done = l
            .lessons
            .iter()
            .filter(|x| progress.get(&x.id).is_some_and(|d| d.satisfies()))
            .count();
        (done, l.lessons.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlocking_follows_prerequisites() {
        let c = Curriculum::load().unwrap();
        let mut p = HashMap::new();
        assert_eq!(c.status("L0.1", &p), Status::Ready);
        assert_eq!(c.status("L0.2", &p), Status::Locked);
        assert_eq!(c.status("L0.7", &p), Status::Ready); // no prerequisites
        assert_eq!(c.next_lesson(&p), Some("L0.1"));
        p.insert("L0.1".to_string(), Done::Completed);
        assert_eq!(c.status("L0.1", &p), Status::Completed);
        assert_eq!(c.status("L0.2", &p), Status::Ready);
        p.insert("L0.2".to_string(), Done::InProgress);
        assert_eq!(c.status("L0.2", &p), Status::InProgress);
        assert_eq!(c.next_lesson(&p), Some("L0.2"));
        assert_eq!(c.status("L7.1", &p), Status::Locked);
        assert_eq!(c.status("L2.1", &p), Status::Locked);
        // Placed-out lessons unlock their followers like completed ones.
        let mut q = HashMap::new();
        for id in [
            "L0.1", "L0.2", "L0.3", "L0.4", "L0.5", "L0.6", "L0.7", "L0.8",
        ] {
            q.insert(id.to_string(), Done::PlacedOut);
        }
        assert_eq!(c.status("L1.1", &q), Status::Ready);
        assert_eq!(c.level_progress(0, &q), (8, 8));
    }
}
