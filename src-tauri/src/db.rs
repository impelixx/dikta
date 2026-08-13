use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: i64,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub duration_ms: i64,
    pub confidence_avg: f64,
    pub word_count: i64,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayStat {
    pub day: String,
    pub total_ms: i64,
    pub words: i64,
    pub sessions: i64,
    pub confidence_avg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStats {
    pub total_ms_today: i64,
    pub total_ms_week: i64,
    pub total_ms_all: i64,
    pub words_today: i64,
    pub words_all: i64,
    pub sessions_today: i64,
    pub sessions_all: i64,
}

fn data_dir() -> Result<PathBuf> {
    let mut dir = dirs::data_dir().context("no data dir")?;
    dir.push("dikta");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn open() -> Result<Db> {
    let mut path = data_dir()?;
    path.push("dikta.sqlite3");
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS recordings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            created_at TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            confidence_avg REAL NOT NULL,
            word_count INTEGER NOT NULL,
            mode TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_recordings_created_at ON recordings(created_at);
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    Ok(Db(Mutex::new(conn)))
}

pub fn insert_recording(
    conn: &Connection,
    text: &str,
    duration_ms: i64,
    confidence_avg: f64,
    mode: &str,
) -> Result<Recording> {
    let created_at = Utc::now();
    let word_count = text.split_whitespace().count() as i64;
    conn.execute(
        "INSERT INTO recordings (text, created_at, duration_ms, confidence_avg, word_count, mode)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            text,
            created_at.to_rfc3339(),
            duration_ms,
            confidence_avg,
            word_count,
            mode
        ],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Recording {
        id,
        text: text.to_string(),
        created_at,
        duration_ms,
        confidence_avg,
        word_count,
        mode: mode.to_string(),
    })
}

pub fn search_recordings(conn: &Connection, query: &str, limit: i64) -> Result<Vec<Recording>> {
    let mut stmt = conn.prepare(
        "SELECT id, text, created_at, duration_ms, confidence_avg, word_count, mode
         FROM recordings WHERE text LIKE ?2 ORDER BY created_at DESC LIMIT ?1",
    )?;
    let like_pattern = format!("%{}%", query.trim());
    let rows = stmt.query_map(params![limit, like_pattern], |row| {
        let created_at: String = row.get(2)?;
        Ok(Recording {
            id: row.get(0)?,
            text: row.get(1)?,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            duration_ms: row.get(3)?,
            confidence_avg: row.get(4)?,
            word_count: row.get(5)?,
            mode: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn delete_recording(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM recordings WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn overall_stats(conn: &Connection) -> Result<OverallStats> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let week_ago = (Utc::now() - chrono::Duration::days(7)).to_rfc3339();

    let total_ms_today: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_ms), 0) FROM recordings WHERE created_at LIKE ?1",
        params![format!("{}%", today)],
        |r| r.get(0),
    )?;
    let total_ms_week: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_ms), 0) FROM recordings WHERE created_at >= ?1",
        params![week_ago],
        |r| r.get(0),
    )?;
    let total_ms_all: i64 =
        conn.query_row("SELECT COALESCE(SUM(duration_ms), 0) FROM recordings", [], |r| r.get(0))?;
    let words_today: i64 = conn.query_row(
        "SELECT COALESCE(SUM(word_count), 0) FROM recordings WHERE created_at LIKE ?1",
        params![format!("{}%", today)],
        |r| r.get(0),
    )?;
    let words_all: i64 =
        conn.query_row("SELECT COALESCE(SUM(word_count), 0) FROM recordings", [], |r| r.get(0))?;
    let sessions_today: i64 = conn.query_row(
        "SELECT COUNT(*) FROM recordings WHERE created_at LIKE ?1",
        params![format!("{}%", today)],
        |r| r.get(0),
    )?;
    let sessions_all: i64 =
        conn.query_row("SELECT COUNT(*) FROM recordings", [], |r| r.get(0))?;

    Ok(OverallStats {
        total_ms_today,
        total_ms_week,
        total_ms_all,
        words_today,
        words_all,
        sessions_today,
        sessions_all,
    })
}

pub fn daily_stats(conn: &Connection, days: i64) -> Result<Vec<DayStat>> {
    let since = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT substr(created_at, 1, 10) as day,
                COALESCE(SUM(duration_ms), 0),
                COALESCE(SUM(word_count), 0),
                COUNT(*),
                COALESCE(AVG(confidence_avg), 0)
         FROM recordings
         WHERE created_at >= ?1
         GROUP BY day
         ORDER BY day ASC",
    )?;
    let rows = stmt.query_map(params![since], |r| {
        Ok(DayStat {
            day: r.get(0)?,
            total_ms: r.get(1)?,
            words: r.get(2)?,
            sessions: r.get(3)?,
            confidence_avg: r.get(4)?,
        })
    })?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE recordings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                created_at TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                confidence_avg REAL NOT NULL,
                word_count INTEGER NOT NULL,
                mode TEXT NOT NULL
            );
            CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            "#,
        )
        .unwrap();
        conn
    }

    #[test]
    fn insert_and_search() {
        let conn = mem_conn();
        insert_recording(&conn, "привет как дела", 1200, 0.92, "push_to_talk").unwrap();
        insert_recording(&conn, "включи свет на кухне", 900, 0.88, "toggle").unwrap();

        let all = search_recordings(&conn, "", 10).unwrap();
        assert_eq!(all.len(), 2);

        let found = search_recordings(&conn, "привет", 10).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].text.contains("привет"));
    }

    #[test]
    fn delete_removes_row() {
        let conn = mem_conn();
        let rec = insert_recording(&conn, "тест", 500, 0.9, "toggle").unwrap();
        delete_recording(&conn, rec.id).unwrap();
        let all = search_recordings(&conn, "", 10).unwrap();
        assert_eq!(all.len(), 0);
    }

    #[test]
    fn stats_aggregate() {
        let conn = mem_conn();
        insert_recording(&conn, "один два три", 1000, 0.9, "toggle").unwrap();
        insert_recording(&conn, "четыре пять", 2000, 0.8, "toggle").unwrap();
        let stats = overall_stats(&conn).unwrap();
        assert_eq!(stats.sessions_today, 2);
        assert_eq!(stats.words_today, 5);
        assert_eq!(stats.total_ms_today, 3000);
    }

    #[test]
    fn settings_roundtrip() {
        let conn = mem_conn();
        set_setting(&conn, "hotkey_push", "CmdOrCtrl+Shift+Space").unwrap();
        assert_eq!(
            get_setting(&conn, "hotkey_push").unwrap(),
            Some("CmdOrCtrl+Shift+Space".to_string())
        );
        set_setting(&conn, "hotkey_push", "F13").unwrap();
        assert_eq!(get_setting(&conn, "hotkey_push").unwrap(), Some("F13".to_string()));
    }
}
