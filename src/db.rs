//! Minimal bindings to the system SQLite library; all values use bound parameters.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    ptr,
};
use unicode_width::UnicodeWidthStr;

#[link(name = "sqlite3")]
unsafe extern "C" {
    fn sqlite3_open_v2(
        filename: *const c_char,
        db: *mut *mut c_void,
        flags: c_int,
        vfs: *const c_char,
    ) -> c_int;
    fn sqlite3_close(db: *mut c_void) -> c_int;
    fn sqlite3_errmsg(db: *mut c_void) -> *const c_char;
    fn sqlite3_busy_timeout(db: *mut c_void, ms: c_int) -> c_int;
    fn sqlite3_prepare_v2(
        db: *mut c_void,
        sql: *const c_char,
        n: c_int,
        stmt: *mut *mut c_void,
        tail: *mut *const c_char,
    ) -> c_int;
    fn sqlite3_bind_text(
        stmt: *mut c_void,
        index: c_int,
        text: *const c_char,
        n: c_int,
        destroy: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> c_int;
    fn sqlite3_bind_parameter_count(stmt: *mut c_void) -> c_int;
    fn sqlite3_step(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_count(stmt: *mut c_void) -> c_int;
    fn sqlite3_column_type(stmt: *mut c_void, col: c_int) -> c_int;
    fn sqlite3_column_text(stmt: *mut c_void, col: c_int) -> *const u8;
    fn sqlite3_finalize(stmt: *mut c_void) -> c_int;
}

pub type Row = Vec<Option<String>>;
pub struct Database(*mut c_void);
struct Statement(*mut c_void);
impl Drop for Statement {
    fn drop(&mut self) {
        unsafe {
            sqlite3_finalize(self.0);
        }
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        unsafe {
            sqlite3_close(self.0);
        }
    }
}
impl Database {
    pub fn connect(path: &str) -> Result<Self> {
        Self::connection(path, 6)
    }
    pub fn read_only(path: &str) -> Result<Self> {
        Self::connection(path, 1)
    }
    fn connection(path: &str, flags: c_int) -> Result<Self> {
        let path = CString::new(path).context("Database path contains NUL")?;
        let mut raw = ptr::null_mut();
        let status = unsafe { sqlite3_open_v2(path.as_ptr(), &mut raw, flags, ptr::null()) };
        ensure!(!raw.is_null(), "SQLite allocation failed");
        let db = Self(raw);
        ensure!(status == 0, "Cannot open SQLite database: {}", db.error());
        ensure!(
            unsafe { sqlite3_busy_timeout(raw, 5000) } == 0,
            "Cannot set SQLite timeout"
        );
        Ok(db)
    }
    pub fn open(path: &str) -> Result<Self> {
        let db = Self::connect(path)?;
        db.query(
            "CREATE TABLE IF NOT EXISTS releases (
            user TEXT NOT NULL, item_key TEXT NOT NULL, artist TEXT NOT NULL,
            release_title TEXT NOT NULL, publish_date TEXT, purchase_date TEXT,
            download_date TEXT, download_url TEXT NOT NULL, format TEXT, file_path TEXT,
            last_error TEXT, attempts INTEGER NOT NULL DEFAULT 0, last_attempt_date TEXT, PRIMARY KEY(user,item_key))",
            &[],
        )?;
        db.query("BEGIN IMMEDIATE", &[])?;
        let columns = db.query("PRAGMA table_info(releases)", &[])?;
        for (name, definition) in [
            ("attempts", "INTEGER NOT NULL DEFAULT 0"),
            ("last_attempt_date", "TEXT"),
            ("genre", "TEXT"),
            ("genre_fallback", "TEXT"),
            ("country_iso", "TEXT"),
            ("release_url", "TEXT"),
            ("catalog_num", "TEXT"),
            ("sanitized_title", "TEXT"),
            ("process_date", "TEXT"),
            ("process_error", "TEXT"),
            ("processed_path", "TEXT"),
        ] {
            if !columns.iter().any(|row| row[1].as_deref() == Some(name)) {
                db.query(
                    &format!("ALTER TABLE releases ADD COLUMN {name} {definition}"),
                    &[],
                )?;
            }
        }
        for row in db.query(
            "SELECT user,item_key,artist,release_title FROM releases WHERE sanitized_title IS NULL",
            &[],
        )? {
            let get = |i: usize| row[i].as_deref().unwrap_or_default();
            let (title, catalog) = crate::sanitizing::title(get(3), get(2));
            db.query("UPDATE releases SET artist=?,sanitized_title=?,catalog_num=COALESCE(NULLIF(CASE WHEN lower(trim(catalog_num))='none' THEN '' ELSE catalog_num END,''),NULLIF(?,'')) WHERE user=? AND item_key=?", &[&crate::sanitizing::artist_display(get(2)),&title,catalog.as_deref().unwrap_or_default(),get(0),get(1)])?;
        }
        db.query("COMMIT", &[])?;
        Ok(db)
    }
    pub fn increment_attempts(&self, user: &str, key: &str) -> Result<()> {
        self.query(
            "UPDATE releases SET attempts=attempts+1,last_attempt_date=? WHERE user=? AND item_key=?",
            &[&chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true), user, key],
        )?;
        Ok(())
    }
    fn error(&self) -> String {
        unsafe {
            CStr::from_ptr(sqlite3_errmsg(self.0))
                .to_string_lossy()
                .into_owned()
        }
    }
    pub fn query(&self, sql: &str, values: &[&str]) -> Result<Vec<Row>> {
        let sql = CString::new(sql)?;
        // SQLITE_STATIC bindings remain alive until the statement is finalized.
        let values: Vec<CString> = values
            .iter()
            .map(|s| CString::new(*s))
            .collect::<std::result::Result<_, _>>()?;
        let mut raw = ptr::null_mut();
        let status =
            unsafe { sqlite3_prepare_v2(self.0, sql.as_ptr(), -1, &mut raw, ptr::null_mut()) };
        let stmt = Statement(raw);
        ensure!(
            status == 0 && !raw.is_null(),
            "SQLite prepare: {}",
            self.error()
        );
        ensure!(
            unsafe { sqlite3_bind_parameter_count(raw) } as usize == values.len(),
            "SQLite parameter count mismatch"
        );
        for (i, value) in values.iter().enumerate() {
            ensure!(
                unsafe { sqlite3_bind_text(raw, (i + 1) as c_int, value.as_ptr(), -1, None) } == 0,
                "SQLite bind: {}",
                self.error()
            );
        }
        let mut rows = Vec::new();
        loop {
            match unsafe { sqlite3_step(raw) } {
                101 => break,
                100 => {
                    let mut row = Vec::new();
                    for col in 0..unsafe { sqlite3_column_count(raw) } {
                        row.push(if unsafe { sqlite3_column_type(raw, col) } == 5 {
                            None
                        } else {
                            let text = unsafe { sqlite3_column_text(raw, col) };
                            ensure!(!text.is_null(), "SQLite could not read column");
                            Some(unsafe {
                                CStr::from_ptr(text.cast()).to_string_lossy().into_owned()
                            })
                        });
                    }
                    rows.push(row);
                }
                _ => anyhow::bail!("SQLite step: {}", self.error()),
            }
        }
        drop(stmt);
        Ok(rows)
    }
    pub fn completed(&self, user: &str, key: &str, force: bool) -> Result<bool> {
        Ok(!force && !self.query("SELECT 1 FROM releases WHERE user=? AND item_key=? AND download_date IS NOT NULL", &[user,key])?.is_empty())
    }
    pub fn dump(&self, out: &mut impl std::io::Write, scope: Option<(&str, &str)>) -> Result<()> {
        let select = "SELECT artist,COALESCE(sanitized_title,release_title),download_date,attempts,CASE WHEN last_error IS NULL AND download_date IS NOT NULL THEN 'success' ELSE last_error END FROM releases";
        let (filter, values) = match scope {
            Some((user, started)) => (
                " WHERE user=? AND last_attempt_date>=?",
                vec![user, started],
            ),
            None => ("", Vec::new()),
        };
        let mut rows = vec![vec![
            "Artist".into(),
            "Title".into(),
            "Download date".into(),
            "Attempts".into(),
            "Last error".into(),
        ]];
        for row in self.query(
            &format!("{select}{filter} ORDER BY user,purchase_date DESC,item_key"),
            &values,
        )? {
            rows.push(
                row.iter()
                    .map(|v| {
                        v.as_deref()
                            .unwrap_or("NULL")
                            .replace('\\', "\\\\")
                            .chars()
                            .map(|c| {
                                if c.is_control() {
                                    c.escape_default().to_string()
                                } else {
                                    c.to_string()
                                }
                            })
                            .collect::<String>()
                    })
                    .collect(),
            );
        }
        let widths: Vec<_> = (0..5)
            .map(|column| {
                rows.iter()
                    .map(|row| row[column].width())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        for (index, row) in rows.iter().enumerate() {
            let cells: Vec<_> = row
                .iter()
                .enumerate()
                .map(|(column, cell)| {
                    let padding = " ".repeat(widths[column] - cell.width());
                    if column == 3 && index > 0 {
                        format!("{padding}{cell}")
                    } else {
                        format!("{cell}{padding}")
                    }
                })
                .collect();
            writeln!(out, "{}", cells.join(" | ").trim_end())?;
            if index == 0 {
                let separators: Vec<_> = widths.iter().map(|width| "-".repeat(*width)).collect();
                writeln!(out, "{}", separators.join("-+-"))?;
            }
        }
        Ok(())
    }
}
