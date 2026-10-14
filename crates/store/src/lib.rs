//! # FretStudy local storage
//!
//! One SQLite file on the learner's computer holds every profile, its settings, and its
//! progress. Nothing is sent anywhere.

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeBand {
    Kid,
    Teen,
    Adult,
}

impl AgeBand {
    pub fn as_str(self) -> &'static str {
        match self {
            AgeBand::Kid => "kid",
            AgeBand::Teen => "teen",
            AgeBand::Adult => "adult",
        }
    }
    fn parse(s: &str) -> AgeBand {
        match s {
            "kid" => AgeBand::Kid,
            "teen" => AgeBand::Teen,
            _ => AgeBand::Adult,
        }
    }
}

/// Per-profile preferences. New fields must have defaults so old saves still load.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub left_handed: bool,
    /// "finger" or "role"
    pub color_mode: String,
    /// "finger", "note", or "interval"
    pub label_mode: String,
    /// "playful" or "clean"
    pub look: String,
    pub input_device: Option<String>,
    /// Measured audio delay (Sound Check), in milliseconds.
    pub latency_ms: Option<f32>,
    pub text_scale: f32,
    /// Show standard notation above tab.
    pub show_staff: bool,
    /// Chord library ids starred on the Chords page.
    pub favorite_chords: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            left_handed: false,
            color_mode: "finger".into(),
            label_mode: "finger".into(),
            look: "playful".into(),
            input_device: None,
            latency_ms: None,
            text_scale: 1.0,
            show_staff: false,
            favorite_chords: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub avatar: String,
    pub age_band: AgeBand,
    pub has_pin: bool,
    pub placement_done: bool,
    pub settings: Settings,
    pub created_at: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LessonProgress {
    pub status: String,
    pub best_score: Option<f32>,
    pub updated_at: i64,
}

pub struct Store {
    conn: Connection,
    /// Folder for recordings (next to the database file).
    files: PathBuf,
}

/// A saved recording of the learner playing.
#[derive(Clone, Debug, PartialEq)]
pub struct Recording {
    pub id: i64,
    pub lesson: Option<String>,
    pub path: PathBuf,
    pub seconds: f32,
    pub created_at: i64,
}

/// A song made in the Song Builder (the song itself is JSON owned by the app).
#[derive(Clone, Debug, PartialEq)]
pub struct SavedSong {
    pub id: i64,
    pub name: String,
    pub data: String,
    pub updated_at: i64,
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// A small, non-cryptographic hash. The PIN only keeps siblings out of each other's
/// profiles on a shared computer; it is not a security boundary.
fn hash_pin(profile_name: &str, pin: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in profile_name.bytes().chain([0u8]).chain(pin.bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

const MIGRATIONS: &[&str] = &[
    // v1
    "CREATE TABLE profiles (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        avatar TEXT NOT NULL,
        age_band TEXT NOT NULL,
        pin_hash TEXT,
        placement_done INTEGER NOT NULL DEFAULT 0,
        settings TEXT NOT NULL DEFAULT '{}',
        created_at INTEGER NOT NULL
    );
    CREATE TABLE progress (
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        lesson_id TEXT NOT NULL,
        status TEXT NOT NULL,
        best_score REAL,
        updated_at INTEGER NOT NULL,
        PRIMARY KEY (profile_id, lesson_id)
    );
    CREATE TABLE check_results (
        id INTEGER PRIMARY KEY,
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        lesson_id TEXT NOT NULL,
        score REAL NOT NULL,
        passed INTEGER NOT NULL,
        taken_at INTEGER NOT NULL
    );
    CREATE TABLE practice_log (
        id INTEGER PRIMARY KEY,
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        day TEXT NOT NULL,
        seconds INTEGER NOT NULL,
        lesson_id TEXT,
        note TEXT
    );
    CREATE TABLE change_scores (
        id INTEGER PRIMARY KEY,
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        chord_a TEXT NOT NULL,
        chord_b TEXT NOT NULL,
        count INTEGER NOT NULL,
        at INTEGER NOT NULL
    );",
    // v2: spaced review of finished lessons
    "CREATE TABLE reviews (
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        lesson_id TEXT NOT NULL,
        stage INTEGER NOT NULL,
        due_day INTEGER NOT NULL,
        PRIMARY KEY (profile_id, lesson_id)
    );",
    // v3: recordings and Song Builder songs
    "CREATE TABLE recordings (
        id INTEGER PRIMARY KEY,
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        lesson_id TEXT,
        file TEXT NOT NULL,
        seconds REAL NOT NULL,
        created_at INTEGER NOT NULL
    );
    CREATE TABLE songs (
        id INTEGER PRIMARY KEY,
        profile_id INTEGER NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        data TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    );",
];

/// Tables copied into a profile backup (every table with a `profile_id`).
const BACKUP_TABLES: [&str; 7] = [
    "progress",
    "check_results",
    "practice_log",
    "change_scores",
    "reviews",
    "recordings",
    "songs",
];

fn json_to_sql(v: &serde_json::Value) -> rusqlite::types::Value {
    use rusqlite::types::Value;
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Integer(*b as i64),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(Value::Integer)
            .unwrap_or_else(|| Value::Real(n.as_f64().unwrap_or(0.0))),
        serde_json::Value::String(s) => Value::Text(s.clone()),
        other => Value::Text(other.to_string()),
    }
}

/// Days between reviews: a lesson comes back after 1 day, then 3, 7, and 21.
/// After the last stage it comes back every 60 days.
pub const REVIEW_GAPS: [i64; 4] = [1, 3, 7, 21];
const LATE_GAP: i64 = 60;

/// Days since 1970-01-01 (UTC), for review due dates.
pub fn day_number() -> i64 {
    now() / 86_400
}

impl Store {
    /// The default database location: the OS app-data folder.
    pub fn default_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "FretStudy", "FretStudy")
            .context("Couldn't find a folder to save your progress in.")?;
        std::fs::create_dir_all(dirs.data_dir())?;
        Ok(dirs.data_dir().join("fretstudy.db"))
    }

    pub fn open(path: &Path) -> Result<Store> {
        let conn =
            Connection::open(path).with_context(|| format!("Couldn't open {}", path.display()))?;
        let files = path
            .parent()
            .map(|p| p.join("recordings"))
            .unwrap_or_else(|| PathBuf::from("recordings"));
        Store::init(conn, files)
    }

    pub fn open_in_memory() -> Result<Store> {
        let files =
            std::env::temp_dir().join(format!("fretstudy-{}-{}", std::process::id(), now_nanos()));
        Store::init(Connection::open_in_memory()?, files)
    }

    fn init(conn: Connection, files: PathBuf) -> Result<Store> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", (i + 1) as i64)?;
        }
        Ok(Store { conn, files })
    }

    // ---------------------------------------------------------------- profiles

    pub fn create_profile(
        &self,
        name: &str,
        avatar: &str,
        age_band: AgeBand,
        pin: Option<&str>,
    ) -> Result<i64> {
        let mut settings = Settings::default();
        if age_band == AgeBand::Adult {
            settings.look = "clean".into();
        }
        self.conn.execute(
            "INSERT INTO profiles (name, avatar, age_band, pin_hash, settings, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, avatar, age_band.as_str(), pin.map(|p| hash_pin(name, p)), serde_json::to_string(&settings)?, now()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let mut stmt = self.conn.prepare("SELECT id, name, avatar, age_band, pin_hash IS NOT NULL, placement_done, settings, created_at FROM profiles ORDER BY created_at, id")?;
        let rows = stmt.query_map([], |r| {
            let settings: String = r.get(6)?;
            Ok(Profile {
                id: r.get(0)?,
                name: r.get(1)?,
                avatar: r.get(2)?,
                age_band: AgeBand::parse(&r.get::<_, String>(3)?),
                has_pin: r.get(4)?,
                placement_done: r.get::<_, i64>(5)? != 0,
                settings: serde_json::from_str(&settings).unwrap_or_default(),
                created_at: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn profile(&self, id: i64) -> Result<Option<Profile>> {
        Ok(self.profiles()?.into_iter().find(|p| p.id == id))
    }

    pub fn check_pin(&self, id: i64, pin: &str) -> Result<bool> {
        let row: Option<(String, Option<String>)> = self
            .conn
            .query_row(
                "SELECT name, pin_hash FROM profiles WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        Ok(match row {
            Some((_, None)) => true,
            Some((name, Some(hash))) => hash == hash_pin(&name, pin),
            None => false,
        })
    }

    pub fn save_settings(&self, id: i64, settings: &Settings) -> Result<()> {
        self.conn.execute(
            "UPDATE profiles SET settings = ?1 WHERE id = ?2",
            params![serde_json::to_string(settings)?, id],
        )?;
        Ok(())
    }

    pub fn set_placement_done(&self, id: i64, done: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE profiles SET placement_done = ?1 WHERE id = ?2",
            params![done as i64, id],
        )?;
        Ok(())
    }

    pub fn delete_profile(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM profiles WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------------------------------------------------------------- progress

    pub fn progress(&self, profile: i64) -> Result<HashMap<String, LessonProgress>> {
        let mut stmt = self.conn.prepare(
            "SELECT lesson_id, status, best_score, updated_at FROM progress WHERE profile_id = ?1",
        )?;
        let rows = stmt.query_map([profile], |r| {
            Ok((
                r.get::<_, String>(0)?,
                LessonProgress {
                    status: r.get(1)?,
                    best_score: r.get(2)?,
                    updated_at: r.get(3)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<HashMap<_, _>, _>>()?)
    }

    /// Sets a lesson's status. A completed lesson is never downgraded to in-progress or
    /// placed-out, and the best score is kept.
    pub fn set_status(
        &self,
        profile: i64,
        lesson: &str,
        status: &str,
        score: Option<f32>,
    ) -> Result<()> {
        let existing: Option<(String, Option<f32>)> = self
            .conn
            .query_row(
                "SELECT status, best_score FROM progress WHERE profile_id = ?1 AND lesson_id = ?2",
                params![profile, lesson],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let status = match existing.as_ref() {
            Some((old, _)) if old == "completed" => "completed",
            _ => status,
        };
        let best = match (existing.and_then(|e| e.1), score) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        self.conn.execute(
            "INSERT INTO progress (profile_id, lesson_id, status, best_score, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(profile_id, lesson_id) DO UPDATE SET status = excluded.status, best_score = excluded.best_score, updated_at = excluded.updated_at",
            params![profile, lesson, status, best, now()],
        )?;
        Ok(())
    }

    /// Marks many lessons placed out (after the placement quiz). Completed ones stay completed.
    pub fn place_out(&self, profile: i64, lessons: &[String]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for l in lessons {
            self.set_status(profile, l, "placed_out", None)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Removes placed-out marks (before re-taking placement). Completed lessons are kept.
    pub fn clear_placed_out(&self, profile: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM progress WHERE profile_id = ?1 AND status = 'placed_out'",
            [profile],
        )?;
        Ok(())
    }

    pub fn record_check(&self, profile: i64, lesson: &str, score: f32, passed: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO check_results (profile_id, lesson_id, score, passed, taken_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![profile, lesson, score, passed as i64, now()],
        )?;
        Ok(())
    }

    /// Recent Check It results, newest first: (lesson, passed).
    pub fn recent_checks(&self, profile: i64, limit: usize) -> Result<Vec<(String, bool)>> {
        let mut stmt = self.conn.prepare("SELECT lesson_id, passed FROM check_results WHERE profile_id = ?1 ORDER BY taken_at DESC, id DESC LIMIT ?2")?;
        let rows = stmt.query_map(params![profile, limit as i64], |r| {
            Ok((r.get(0)?, r.get::<_, i64>(1)? != 0))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ---------------------------------------------------------------- practice

    pub fn log_practice(
        &self,
        profile: i64,
        day: &str,
        seconds: u32,
        lesson: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO practice_log (profile_id, day, seconds, lesson_id) VALUES (?1, ?2, ?3, ?4)",
            params![profile, day, seconds as i64, lesson],
        )?;
        Ok(())
    }

    /// Total practice seconds per day (oldest first) for the last `days` days that have any.
    pub fn practice_days(&self, profile: i64, days: usize) -> Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare("SELECT day, SUM(seconds) FROM practice_log WHERE profile_id = ?1 GROUP BY day ORDER BY day DESC LIMIT ?2")?;
        let mut rows = stmt
            .query_map(params![profile, days as i64], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows.reverse();
        Ok(rows)
    }

    // ---------------------------------------------------------------- recordings

    /// The folder recordings are saved in (created if needed).
    pub fn recordings_dir(&self) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.files)
            .with_context(|| format!("Couldn't create {}", self.files.display()))?;
        Ok(self.files.clone())
    }

    /// A new, unused file name for a recording (the caller writes the WAV there).
    pub fn new_recording_path(&self, profile: i64) -> Result<PathBuf> {
        Ok(self
            .recordings_dir()?
            .join(format!("take-{profile}-{}.wav", now_nanos() / 1_000_000)))
    }

    /// Remembers a recording the app saved at `path`.
    pub fn add_recording(
        &self,
        profile: i64,
        lesson: Option<&str>,
        path: &Path,
        seconds: f32,
    ) -> Result<i64> {
        let file = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .context("bad recording path")?;
        self.conn.execute(
            "INSERT INTO recordings (profile_id, lesson_id, file, seconds, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![profile, lesson, file, seconds as f64, now()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// A profile's recordings, newest first (optionally only one lesson's).
    pub fn recordings(&self, profile: i64, lesson: Option<&str>) -> Result<Vec<Recording>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, lesson_id, file, seconds, created_at FROM recordings
             WHERE profile_id = ?1 AND (?2 IS NULL OR lesson_id = ?2) ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map(params![profile, lesson], |r| {
            Ok(Recording {
                id: r.get(0)?,
                lesson: r.get(1)?,
                path: self.files.join(r.get::<_, String>(2)?),
                seconds: r.get::<_, f64>(3)? as f32,
                created_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Deletes a recording and its file.
    pub fn delete_recording(&self, id: i64) -> Result<()> {
        let file: Option<String> = self
            .conn
            .query_row("SELECT file FROM recordings WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        if let Some(f) = file {
            let _ = std::fs::remove_file(self.files.join(f));
        }
        self.conn
            .execute("DELETE FROM recordings WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------------------------------------------------------------- backup

    /// Writes one profile (settings, progress, history, songs, and recordings) to a
    /// `.fretstudy` file: a zip with `fretstudy.json` plus the recordings' WAV files.
    pub fn export_profile(&self, profile: i64, path: &Path) -> Result<()> {
        use std::io::Write;
        let row = self
            .conn
            .query_row(
                "SELECT name, avatar, age_band, pin_hash, placement_done, settings, created_at FROM profiles WHERE id = ?1",
                [profile],
                |r| {
                    Ok(serde_json::json!({
                        "name": r.get::<_, String>(0)?,
                        "avatar": r.get::<_, String>(1)?,
                        "age_band": r.get::<_, String>(2)?,
                        "pin_hash": r.get::<_, Option<String>>(3)?,
                        "placement_done": r.get::<_, i64>(4)?,
                        "settings": r.get::<_, String>(5)?,
                        "created_at": r.get::<_, i64>(6)?,
                    }))
                },
            )
            .optional()?
            .context("That profile doesn't exist.")?;
        let mut tables = serde_json::Map::new();
        for table in BACKUP_TABLES {
            tables.insert(
                table.to_string(),
                serde_json::Value::Array(self.rows_of(table, profile)?),
            );
        }
        let doc = serde_json::json!({
            "format": 1,
            "app": "FretStudy",
            "exported_at": now(),
            "profile": row,
            "tables": tables,
        });
        let file = std::fs::File::create(path)
            .with_context(|| format!("Couldn't create {}", path.display()))?;
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("fretstudy.json", opts)?;
        zip.write_all(serde_json::to_string_pretty(&doc)?.as_bytes())?;
        for r in self.recordings(profile, None)? {
            if let Ok(bytes) = std::fs::read(&r.path) {
                let name = r
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                zip.start_file(format!("recordings/{name}"), opts)?;
                zip.write_all(&bytes)?;
            }
        }
        zip.finish()?;
        Ok(())
    }

    /// Reads a `.fretstudy` file and adds it as a new profile. Returns the new profile's id.
    pub fn import_profile(&self, path: &Path) -> Result<i64> {
        use std::io::Read;
        let file = std::fs::File::open(path)
            .with_context(|| format!("Couldn't open {}", path.display()))?;
        let mut zip = zip::ZipArchive::new(file).context("That isn't a FretStudy backup file.")?;
        let doc: serde_json::Value = {
            let mut entry = zip
                .by_name("fretstudy.json")
                .context("That isn't a FretStudy backup file.")?;
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            serde_json::from_str(&text)?
        };
        anyhow::ensure!(
            doc["format"] == 1,
            "This backup was made by a newer FretStudy. Please update first."
        );
        let p = &doc["profile"];
        let mut name = p["name"].as_str().unwrap_or("Player").to_string();
        let taken: Vec<String> = self.profiles()?.into_iter().map(|p| p.name).collect();
        // The PIN is tied to the name, so a renamed profile starts without one.
        let mut pin_hash = p["pin_hash"].as_str();
        if taken.contains(&name) {
            name = format!("{name} (imported)");
            pin_hash = None;
        }
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO profiles (name, avatar, age_band, pin_hash, placement_done, settings, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                name,
                p["avatar"].as_str().unwrap_or("🎸"),
                p["age_band"].as_str().unwrap_or("adult"),
                pin_hash,
                p["placement_done"].as_i64().unwrap_or(0),
                p["settings"].as_str().unwrap_or("{}"),
                p["created_at"].as_i64().unwrap_or_else(now),
            ],
        )?;
        let id = tx.last_insert_rowid();
        let mut copied_files = Vec::new();
        for table in BACKUP_TABLES {
            let Some(rows) = doc["tables"][table].as_array() else {
                continue;
            };
            for row in rows {
                let Some(obj) = row.as_object() else { continue };
                let mut cols = vec!["profile_id".to_string()];
                let mut vals: Vec<rusqlite::types::Value> = vec![id.into()];
                for (k, v) in obj {
                    if k == "id" || k == "profile_id" {
                        continue;
                    }
                    let mut v = json_to_sql(v);
                    // Recordings get a fresh file name so nothing is overwritten.
                    if table == "recordings"
                        && k == "file"
                        && let rusqlite::types::Value::Text(old) = &v
                    {
                        let fresh = format!("take-{id}-{}-{old}", now_nanos() / 1_000_000);
                        copied_files.push((old.clone(), fresh.clone()));
                        v = rusqlite::types::Value::Text(fresh);
                    }
                    cols.push(k.clone());
                    vals.push(v);
                }
                anyhow::ensure!(
                    cols.iter()
                        .all(|c| c.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')),
                    "Bad column name in backup"
                );
                let sql = format!(
                    "INSERT OR IGNORE INTO {table} ({}) VALUES ({})",
                    cols.join(", "),
                    (1..=cols.len())
                        .map(|i| format!("?{i}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                tx.execute(&sql, rusqlite::params_from_iter(vals))?;
            }
        }
        let dir = self.recordings_dir()?;
        for (old, fresh) in copied_files {
            if let Ok(mut entry) = zip.by_name(&format!("recordings/{old}")) {
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes)?;
                std::fs::write(dir.join(&fresh), bytes)?;
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// Every row of a profile's table, as JSON objects keyed by column name.
    fn rows_of(&self, table: &str, profile: i64) -> Result<Vec<serde_json::Value>> {
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT * FROM {table} WHERE profile_id = ?1"))?;
        let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt.query_map([profile], |r| {
            let mut obj = serde_json::Map::new();
            for (i, n) in names.iter().enumerate() {
                let v = match r.get::<_, rusqlite::types::Value>(i)? {
                    rusqlite::types::Value::Null => serde_json::Value::Null,
                    rusqlite::types::Value::Integer(x) => x.into(),
                    rusqlite::types::Value::Real(x) => x.into(),
                    rusqlite::types::Value::Text(t) => t.into(),
                    rusqlite::types::Value::Blob(_) => serde_json::Value::Null,
                };
                obj.insert(n.clone(), v);
            }
            Ok(serde_json::Value::Object(obj))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // ---------------------------------------------------------------- songs

    /// Saves a song (new when `id` is None). Returns its id.
    pub fn save_song(&self, profile: i64, id: Option<i64>, name: &str, data: &str) -> Result<i64> {
        match id {
            Some(id) => {
                self.conn.execute(
                    "UPDATE songs SET name = ?1, data = ?2, updated_at = ?3 WHERE id = ?4 AND profile_id = ?5",
                    params![name, data, now(), id, profile],
                )?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO songs (profile_id, name, data, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    params![profile, name, data, now()],
                )?;
                Ok(self.conn.last_insert_rowid())
            }
        }
    }

    /// A profile's songs, most recently changed first.
    pub fn songs(&self, profile: i64) -> Result<Vec<SavedSong>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, data, updated_at FROM songs WHERE profile_id = ?1 ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([profile], |r| {
            Ok(SavedSong {
                id: r.get(0)?,
                name: r.get(1)?,
                data: r.get(2)?,
                updated_at: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn delete_song(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM songs WHERE id = ?1", [id])?;
        Ok(())
    }

    // ---------------------------------------------------------------- spaced review

    /// Starts reviewing a lesson (after it's completed). Does nothing if it's already scheduled.
    pub fn schedule_review(&self, profile: i64, lesson: &str, today: i64) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO reviews (profile_id, lesson_id, stage, due_day) VALUES (?1, ?2, 0, ?3)",
            params![profile, lesson, today + REVIEW_GAPS[0]],
        )?;
        Ok(())
    }

    /// Lessons due for review on `today`, most overdue first: (lesson, stage).
    pub fn due_reviews(&self, profile: i64, today: i64) -> Result<Vec<(String, u8)>> {
        let mut stmt = self.conn.prepare(
            "SELECT lesson_id, stage FROM reviews WHERE profile_id = ?1 AND due_day <= ?2 ORDER BY due_day, lesson_id",
        )?;
        let rows = stmt.query_map(params![profile, today], |r| {
            Ok((r.get(0)?, r.get::<_, i64>(1)? as u8))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Records a review: passing moves the lesson to a longer gap; missing it brings it
    /// back tomorrow.
    pub fn review_result(
        &self,
        profile: i64,
        lesson: &str,
        passed: bool,
        today: i64,
    ) -> Result<()> {
        let stage: i64 = self
            .conn
            .query_row(
                "SELECT stage FROM reviews WHERE profile_id = ?1 AND lesson_id = ?2",
                params![profile, lesson],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        let stage = if passed { stage + 1 } else { 0 };
        let gap = if passed {
            REVIEW_GAPS.get(stage as usize).copied().unwrap_or(LATE_GAP)
        } else {
            REVIEW_GAPS[0]
        };
        self.conn.execute(
            "INSERT INTO reviews (profile_id, lesson_id, stage, due_day) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(profile_id, lesson_id) DO UPDATE SET stage = excluded.stage, due_day = excluded.due_day",
            params![profile, lesson, stage, today + gap],
        )?;
        Ok(())
    }

    pub fn record_changes(&self, profile: i64, a: &str, b: &str, count: u32) -> Result<()> {
        self.conn.execute(
            "INSERT INTO change_scores (profile_id, chord_a, chord_b, count, at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![profile, a, b, count as i64, now()],
        )?;
        Ok(())
    }

    /// Best One-Minute Changes score for a chord pair (either order).
    pub fn best_changes(&self, profile: i64, a: &str, b: &str) -> Result<Option<u32>> {
        let best: Option<i64> = self.conn.query_row(
            "SELECT MAX(count) FROM change_scores WHERE profile_id = ?1 AND ((chord_a = ?2 AND chord_b = ?3) OR (chord_a = ?3 AND chord_b = ?2))",
            params![profile, a, b],
            |r| r.get(0),
        )?;
        Ok(best.map(|v| v as u32))
    }
}

/// Today's date as YYYY-MM-DD in UTC (good enough for streaks; avoids a date library).
pub fn today() -> String {
    let days = now().div_euclid(86_400);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_and_import_a_profile() {
        let a = Store::open_in_memory().unwrap();
        let p = a
            .create_profile("Sam", "🦊", AgeBand::Kid, Some("1234"))
            .unwrap();
        let mut settings = a.profile(p).unwrap().unwrap().settings;
        settings.left_handed = true;
        a.save_settings(p, &settings).unwrap();
        a.set_status(p, "L1.1", "completed", Some(0.9)).unwrap();
        a.place_out(p, &["L0.1".to_string()]).unwrap();
        a.record_check(p, "L1.1", 0.9, true).unwrap();
        a.log_practice(p, "2026-10-13", 600, Some("L1.1")).unwrap();
        a.record_changes(p, "G", "C", 31).unwrap();
        a.schedule_review(p, "L1.1", 100).unwrap();
        a.save_song(p, None, "Tune", "{\"x\":1}").unwrap();
        let wav = a.new_recording_path(p).unwrap();
        std::fs::write(&wav, b"RIFFdata").unwrap();
        a.add_recording(p, Some("L7.12"), &wav, 3.0).unwrap();

        let file = std::env::temp_dir().join(format!("fretstudy-backup-{}.fretstudy", now_nanos()));
        a.export_profile(p, &file).unwrap();

        // Import into a different computer that already has a "Sam".
        let b = Store::open_in_memory().unwrap();
        b.create_profile("Sam", "🐱", AgeBand::Adult, None).unwrap();
        let q = b.import_profile(&file).unwrap();
        let prof = b.profile(q).unwrap().unwrap();
        assert_eq!(prof.name, "Sam (imported)");
        assert!(
            !prof.has_pin,
            "a renamed profile can't keep a name-tied PIN"
        );
        assert!(prof.settings.left_handed);
        let progress = b.progress(q).unwrap();
        assert_eq!(progress.len(), 2);
        assert_eq!(b.recent_checks(q, 5).unwrap().len(), 1);
        assert_eq!(
            b.practice_days(q, 7).unwrap(),
            vec![("2026-10-13".to_string(), 600)]
        );
        assert_eq!(b.best_changes(q, "C", "G").unwrap(), Some(31));
        assert_eq!(b.due_reviews(q, 200).unwrap().len(), 1);
        assert_eq!(b.songs(q).unwrap()[0].name, "Tune");
        let recs = b.recordings(q, None).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(std::fs::read(&recs[0].path).unwrap(), b"RIFFdata");
        // On a computer without a "Sam", the name and PIN carry over.
        let c = Store::open_in_memory().unwrap();
        let r = c.import_profile(&file).unwrap();
        assert!(c.check_pin(r, "1234").unwrap());
        assert!(!c.check_pin(r, "0000").unwrap());
        std::fs::remove_file(file).ok();

        // Not a backup file.
        let junk = std::env::temp_dir().join(format!("junk-{}.fretstudy", now_nanos()));
        std::fs::write(&junk, b"hello").unwrap();
        assert!(b.import_profile(&junk).is_err());
        std::fs::remove_file(junk).ok();
    }

    #[test]
    fn recordings_and_songs() {
        let s = Store::open_in_memory().unwrap();
        let p = s.create_profile("Sam", "🦊", AgeBand::Kid, None).unwrap();
        let path = s.new_recording_path(p).unwrap();
        std::fs::write(&path, b"RIFF").unwrap();
        let id = s.add_recording(p, Some("L7.12"), &path, 12.5).unwrap();
        let all = s.recordings(p, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].path, path);
        assert_eq!(s.recordings(p, Some("L7.12")).unwrap().len(), 1);
        assert!(s.recordings(p, Some("L1.1")).unwrap().is_empty());
        s.delete_recording(id).unwrap();
        assert!(!path.exists());
        assert!(s.recordings(p, None).unwrap().is_empty());

        let song = s.save_song(p, None, "My Song", "{}").unwrap();
        s.save_song(p, Some(song), "My Song 2", "{\"a\":1}")
            .unwrap();
        let songs = s.songs(p).unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].name, "My Song 2");
        s.delete_song(song).unwrap();
        assert!(s.songs(p).unwrap().is_empty());
    }

    #[test]
    fn reviews_space_out_and_reset() {
        let s = Store::open_in_memory().unwrap();
        let p = s.create_profile("Sam", "🦊", AgeBand::Kid, None).unwrap();
        let day = 20_000;
        s.schedule_review(p, "L1.1", day).unwrap();
        assert!(s.due_reviews(p, day).unwrap().is_empty());
        assert_eq!(
            s.due_reviews(p, day + 1).unwrap(),
            vec![("L1.1".to_string(), 0)]
        );
        // Scheduling again doesn't reset it.
        s.review_result(p, "L1.1", true, day + 1).unwrap();
        s.schedule_review(p, "L1.1", day + 1).unwrap();
        assert!(s.due_reviews(p, day + 3).unwrap().is_empty());
        assert_eq!(s.due_reviews(p, day + 4).unwrap().len(), 1);
        // Missing it brings it back tomorrow at stage 0.
        s.review_result(p, "L1.1", false, day + 4).unwrap();
        assert_eq!(
            s.due_reviews(p, day + 5).unwrap(),
            vec![("L1.1".to_string(), 0)]
        );
        // Past the last gap it comes back every 60 days.
        for _ in 0..5 {
            s.review_result(p, "L1.1", true, day).unwrap();
        }
        assert!(s.due_reviews(p, day + 59).unwrap().is_empty());
        assert_eq!(s.due_reviews(p, day + 60).unwrap().len(), 1);
    }

    #[test]
    fn profiles_and_settings_round_trip() {
        let s = Store::open_in_memory().unwrap();
        let kid = s
            .create_profile("Sam", "🦊", AgeBand::Kid, Some("1234"))
            .unwrap();
        let adult = s
            .create_profile("Alex", "🎸", AgeBand::Adult, None)
            .unwrap();
        let all = s.profiles().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "Sam");
        assert!(all[0].has_pin);
        assert_eq!(all[0].settings.look, "playful");
        assert_eq!(all[1].settings.look, "clean");
        assert!(s.check_pin(kid, "1234").unwrap());
        assert!(!s.check_pin(kid, "0000").unwrap());
        assert!(s.check_pin(adult, "").unwrap());

        let mut settings = all[1].settings.clone();
        settings.left_handed = true;
        settings.latency_ms = Some(42.5);
        s.save_settings(adult, &settings).unwrap();
        assert_eq!(s.profile(adult).unwrap().unwrap().settings, settings);
    }

    #[test]
    fn progress_rules() {
        let s = Store::open_in_memory().unwrap();
        let p = s.create_profile("Sam", "🦊", AgeBand::Kid, None).unwrap();
        s.set_status(p, "L0.1", "in_progress", None).unwrap();
        s.set_status(p, "L0.1", "completed", Some(0.8)).unwrap();
        s.set_status(p, "L0.1", "in_progress", Some(0.5)).unwrap(); // reviewing never downgrades
        let prog = s.progress(p).unwrap();
        assert_eq!(prog["L0.1"].status, "completed");
        assert_eq!(prog["L0.1"].best_score, Some(0.8));

        s.place_out(p, &["L0.1".into(), "L0.2".into()]).unwrap();
        let prog = s.progress(p).unwrap();
        assert_eq!(
            prog["L0.1"].status, "completed",
            "completed stays completed"
        );
        assert_eq!(prog["L0.2"].status, "placed_out");
        s.clear_placed_out(p).unwrap();
        let prog = s.progress(p).unwrap();
        assert!(!prog.contains_key("L0.2"));
        assert!(prog.contains_key("L0.1"));
    }

    #[test]
    fn deleting_a_profile_removes_its_data() {
        let s = Store::open_in_memory().unwrap();
        let p = s.create_profile("Sam", "🦊", AgeBand::Kid, None).unwrap();
        s.set_status(p, "L0.1", "completed", Some(1.0)).unwrap();
        s.record_changes(p, "Em", "G-easy", 18).unwrap();
        s.delete_profile(p).unwrap();
        assert!(s.profiles().unwrap().is_empty());
        assert!(s.progress(p).unwrap().is_empty());
    }

    #[test]
    fn practice_and_changes() {
        let s = Store::open_in_memory().unwrap();
        let p = s.create_profile("Sam", "🦊", AgeBand::Kid, None).unwrap();
        s.log_practice(p, "2026-10-12", 600, Some("L0.1")).unwrap();
        s.log_practice(p, "2026-10-13", 300, None).unwrap();
        s.log_practice(p, "2026-10-13", 120, None).unwrap();
        assert_eq!(
            s.practice_days(p, 7).unwrap(),
            vec![("2026-10-12".into(), 600), ("2026-10-13".into(), 420)]
        );
        s.record_changes(p, "Em", "G-easy", 14).unwrap();
        s.record_changes(p, "G-easy", "Em", 21).unwrap();
        assert_eq!(s.best_changes(p, "Em", "G-easy").unwrap(), Some(21));
        assert_eq!(s.best_changes(p, "C", "G").unwrap(), None);
    }

    #[test]
    fn reopening_keeps_data_and_migrations_run_once() {
        let dir = std::env::temp_dir().join(format!("fretstudy-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("t.db");
        let _ = std::fs::remove_file(&path);
        {
            let s = Store::open(&path).unwrap();
            s.create_profile("Sam", "🦊", AgeBand::Kid, None).unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.profiles().unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
