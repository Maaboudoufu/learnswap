//! Database connection and the one dialect difference we have to handle.
//!
//! The app talks to SQLite or Postgres through sqlx's `Any` driver, chosen at
//! runtime from `DATABASE_URL`. Everything else in the codebase is written
//! against `AnyPool` and does not care which one is behind it.

use sqlx::any::{AnyPoolOptions, install_default_drivers};
use sqlx::{AnyPool, Executor};

/// Opens the pool and brings the schema up to date.
///
/// Accepts anything sqlx does, e.g. `sqlite://learnswap.db?mode=rwc`,
/// `sqlite::memory:` or `postgres://user:pass@localhost/learnswap`.
pub async fn connect(url: &str) -> Result<AnyPool, sqlx::Error> {
    // Must happen before the first connect or `Any` has no backends registered.
    install_default_drivers();

    let pool = AnyPoolOptions::new()
        // An in-memory SQLite database is per-connection: a second connection
        // would open a second, empty database. One connection keeps tests sane.
        .max_connections(if is_memory_sqlite(url) { 1 } else { 5 })
        .connect(url)
        .await?;

    if is_sqlite(url) {
        // SQLite ignores foreign keys unless asked, and defaults to a lock that
        // makes concurrent readers fail while a write is open.
        pool.execute("PRAGMA foreign_keys = ON").await?;
        pool.execute("PRAGMA journal_mode = WAL").await?;
    }

    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}

fn is_sqlite(url: &str) -> bool {
    url.starts_with("sqlite:")
}

fn is_memory_sqlite(url: &str) -> bool {
    is_sqlite(url) && url.contains(":memory:")
}

pub fn is_postgres(url: &str) -> bool {
    url.starts_with("postgres:") || url.starts_with("postgresql:")
}

/// Rewrites `?` placeholders into Postgres' `$1, $2, ...` form.
///
/// sqlx's `Any` driver does **not** normalise placeholders — it passes the SQL
/// through untouched, so `?` works on SQLite and `$N` on Postgres. Rather than
/// keep two copies of every query, all SQL in this crate is written with `?`
/// and rewritten here when the pool is talking to Postgres.
///
/// ponytail: counts every `?` in the string. Safe because none of our queries
/// contain a `?` inside a string literal or a cast; if one ever does, this will
/// mangle it, and that is the point to reach for a real query builder.
pub fn rebind(sql: &str, postgres: bool) -> String {
    if !postgres {
        return sql.to_string();
    }
    let mut out = String::with_capacity(sql.len() + 8);
    let mut n = 0;
    for ch in sql.chars() {
        if ch == '?' {
            n += 1;
            out.push('$');
            out.push_str(&n.to_string());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_sql_is_untouched() {
        let sql = "SELECT * FROM users WHERE email = ? AND name = ?";
        assert_eq!(rebind(sql, false), sql);
    }

    #[test]
    fn postgres_placeholders_are_numbered_in_order() {
        assert_eq!(
            rebind("SELECT * FROM users WHERE email = ? AND name = ?", true),
            "SELECT * FROM users WHERE email = $1 AND name = $2"
        );
    }

    #[test]
    fn url_scheme_detection() {
        assert!(is_sqlite("sqlite://learnswap.db"));
        assert!(is_memory_sqlite("sqlite::memory:"));
        assert!(!is_memory_sqlite("sqlite://learnswap.db"));
        assert!(is_postgres("postgres://u:p@localhost/db"));
        assert!(is_postgres("postgresql://u:p@localhost/db"));
        assert!(!is_postgres("sqlite::memory:"));
    }
}
