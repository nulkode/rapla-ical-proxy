use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub id: i64,
    pub name: String,
    pub rapla_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaType {
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "modify")]
    Modify,
    #[serde(rename = "add")]
    Add,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayDelta {
    pub id: Uuid,
    pub calendar_id: i64,
    pub r#type: DeltaType,
    pub match_key: Option<String>,
    pub event_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS calendars (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                rapla_url TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS overlay_deltas (
                id TEXT PRIMARY KEY,
                calendar_id INTEGER NOT NULL REFERENCES calendars(id),
                type TEXT NOT NULL CHECK(type IN ('delete', 'modify', 'add')),
                match_key TEXT,
                event_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn list_calendars(&self) -> rusqlite::Result<Vec<Calendar>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, rapla_url FROM calendars ORDER BY id")?;
        stmt.query_map([], |row| {
            Ok(Calendar {
                id: row.get(0)?,
                name: row.get(1)?,
                rapla_url: row.get(2)?,
            })
        })?
        .collect()
    }

    pub fn add_calendar(&self, name: &str, rapla_url: &str) -> rusqlite::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO calendars (name, rapla_url) VALUES (?1, ?2)",
            params![name, rapla_url],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_calendar(&self, id: i64) -> rusqlite::Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM overlay_deltas WHERE calendar_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM calendars WHERE id = ?1", params![id])
    }

    pub fn get_calendar(&self, id: i64) -> rusqlite::Result<Option<Calendar>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, name, rapla_url FROM calendars WHERE id = ?1")?;
        stmt.query_row(params![id], |row| {
            Ok(Calendar {
                id: row.get(0)?,
                name: row.get(1)?,
                rapla_url: row.get(2)?,
            })
        })
        .optional()
    }

    pub fn list_deltas(&self, calendar_id: i64) -> rusqlite::Result<Vec<OverlayDelta>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, calendar_id, type, match_key, event_json FROM overlay_deltas WHERE calendar_id = ?1",
        )?;
        stmt.query_map(params![calendar_id], |row| {
            let id_str: String = row.get(0)?;
            Ok(OverlayDelta {
                id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil()),
                calendar_id: row.get(1)?,
                r#type: serde_json::from_str(&format!("\"{}\"", row.get::<_, String>(2)?))
                    .unwrap_or(DeltaType::Delete),
                match_key: row.get(3)?,
                event_json: row.get(4)?,
            })
        })?
        .collect()
    }

    pub fn add_delta(
        &self,
        calendar_id: i64,
        r#type: DeltaType,
        match_key: Option<String>,
        event_json: Option<String>,
    ) -> rusqlite::Result<OverlayDelta> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4();
        conn.execute(
            "INSERT INTO overlay_deltas (id, calendar_id, type, match_key, event_json) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.to_string(),
                calendar_id,
                match r#type {
                    DeltaType::Delete => "delete",
                    DeltaType::Modify => "modify",
                    DeltaType::Add => "add",
                },
                match_key,
                event_json,
            ],
        )?;
        Ok(OverlayDelta {
            id,
            calendar_id,
            r#type,
            match_key,
            event_json,
        })
    }

    pub fn delete_delta(&self, id: Uuid) -> rusqlite::Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM overlay_deltas WHERE id = ?1",
            params![id.to_string()],
        )
    }
}
