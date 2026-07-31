//! Category CRUD (`03-data-model.md` §3.6).

use crate::error::{CoreError, Result};
use crate::model::Category;
use crate::util;
use rusqlite::{Connection, params};

pub fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get("id")?,
        name: row.get("name")?,
        color_hue: row.get("color_hue")?,
        icon: row.get("icon")?,
        sort_order: row.get("sort_order")?,
        is_builtin: row.get::<_, i64>("is_builtin")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT * FROM categories ORDER BY sort_order, name")?;
    let rows = stmt.query_map([], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: &str) -> Result<Category> {
    conn.query_row(
        "SELECT * FROM categories WHERE id = ?",
        params![id],
        row_from,
    )
    .map_err(CoreError::from)
}

pub fn get_by_name(conn: &Connection, name: &str) -> Result<Vec<Category>> {
    let mut stmt = conn.prepare("SELECT * FROM categories WHERE lower(name) = lower(?)")?;
    let rows = stmt.query_map(params![name], row_from)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Resolve a category by id, unique name, or a builtin English alias. Ambiguous
/// names error.
pub fn resolve(conn: &Connection, id_or_name: &str) -> Result<Category> {
    // Builtin English aliases (the CLI spec examples use these tokens).
    let alias_id = match id_or_name.to_ascii_lowercase().as_str() {
        "work" => Some("cat_work"),
        "health" => Some("cat_health"),
        "study" => Some("cat_study"),
        "rest" => Some("cat_rest"),
        "personal" => Some("cat_personal"),
        "other" => Some("cat_other"),
        _ => None,
    };
    if let Some(id) = alias_id
        && let Ok(c) = get(conn, id)
    {
        return Ok(c);
    }
    // Then try exact id match.
    if let Ok(c) = get(conn, id_or_name) {
        return Ok(c);
    }
    let by_name = get_by_name(conn, id_or_name)?;
    match by_name.len() {
        0 => Err(CoreError::NotFound(format!("category '{id_or_name}'"))),
        1 => Ok(by_name[0].clone()),
        _ => Err(CoreError::AmbiguousCategory(id_or_name.to_string())),
    }
}

pub struct NewCategory {
    pub name: String,
    pub color_hue: f64,
    pub icon: Option<String>,
}

pub fn create(conn: &Connection, input: NewCategory) -> Result<Category> {
    if !(0.0..=360.0).contains(&input.color_hue) {
        return Err(CoreError::InvalidArgument(
            "color_hue must be 0..=360".into(),
        ));
    }
    let id = util::new_id();
    let now = util::now_iso();
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO categories (id, name, color_hue, icon, sort_order, is_builtin, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6)",
        params![id, input.name, input.color_hue, input.icon, next_order, now],
    )?;
    get(conn, &id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    let n = conn.execute("DELETE FROM categories WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(CoreError::NotFound(format!("category '{id}'")));
    }
    Ok(())
}
