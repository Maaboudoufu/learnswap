//! All database access lives here.
//!
//! Route handlers call these methods and never touch SQL, so the queries stay
//! in one place and stay reviewable. Every statement is written with `?`
//! placeholders and passed through `db::rebind`, which is what makes the same
//! code work on SQLite and Postgres.

use sqlx::{AnyPool, Row};
use uuid::Uuid;

use crate::auth::{hash_password, verify_password};
use crate::db;
use crate::models::{Skill, Swap, User};

/// Columns making up a `User`, in the order `user_from_row` reads them.
const USER_COLUMNS: &str = "id, email, name, headline, bio, teaching, learning";

#[derive(Clone)]
pub struct Store {
    pool: AnyPool,
    postgres: bool,
}

/// Why a registration was refused.
#[derive(Debug)]
pub enum RegisterError {
    EmailTaken,
    Database(sqlx::Error),
    Hash(argon2::password_hash::Error),
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegisterError::EmailTaken => write!(f, "that email is already registered"),
            RegisterError::Database(e) => write!(f, "database error: {e}"),
            RegisterError::Hash(e) => write!(f, "password hashing error: {e}"),
        }
    }
}

impl std::error::Error for RegisterError {}

impl From<sqlx::Error> for RegisterError {
    fn from(e: sqlx::Error) -> Self {
        RegisterError::Database(e)
    }
}

/// The fields needed to create an account. A struct rather than seven string
/// parameters, so call sites cannot silently swap two of them.
pub struct NewAccount<'a> {
    pub email: &'a str,
    pub password: &'a str,
    pub name: &'a str,
    pub headline: &'a str,
    pub bio: &'a str,
    pub teaching: &'a str,
    pub learning: &'a str,
}

impl Store {
    pub fn new(pool: AnyPool, url: &str) -> Self {
        Store {
            pool,
            postgres: db::is_postgres(url),
        }
    }

    /// Connects, migrates and wraps the pool.
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        Ok(Store::new(db::connect(url).await?, url))
    }

    /// The underlying pool. Exposed so tests can assert on what was actually
    /// written; application code should go through the methods below.
    pub fn pool(&self) -> &AnyPool {
        &self.pool
    }

    /// Rewrites `?` placeholders for the backend actually in use.
    fn sql(&self, sql: &str) -> String {
        db::rebind(sql, self.postgres)
    }

    pub async fn count(&self) -> Result<usize, sqlx::Error> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get::<i64, _>("n")? as usize)
    }

    pub async fn all(&self) -> Result<Vec<User>, sqlx::Error> {
        let sql = format!("SELECT {USER_COLUMNS} FROM users ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.iter().map(user_from_row).collect()
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        let sql = self.sql(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = ?"));
        let row = sqlx::query(&sql)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(user_from_row).transpose()
    }

    /// Free-text search over names, headlines and skill tags.
    ///
    /// Filtering happens in SQL so the whole table never has to come back for a
    /// search. `LIKE` with `lower()` is the dialect-neutral way to do this;
    /// Postgres has `ILIKE` but SQLite does not.
    pub async fn search(&self, query: &str) -> Result<Vec<User>, sqlx::Error> {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return self.all().await;
        }
        // SQLite has no default LIKE escape character and Postgres uses
        // backslash; naming it explicitly makes the two agree.
        let sql = self.sql(&format!(
            "SELECT {USER_COLUMNS} FROM users \
             WHERE lower(name)     LIKE ? ESCAPE '\\' \
                OR lower(headline) LIKE ? ESCAPE '\\' \
                OR lower(teaching) LIKE ? ESCAPE '\\' \
                OR lower(learning) LIKE ? ESCAPE '\\' \
             ORDER BY name"
        ));
        let pattern = format!("%{}%", escape_like(&needle));
        let rows = sqlx::query(&sql)
            .bind(pattern.clone())
            .bind(pattern.clone())
            .bind(pattern.clone())
            .bind(pattern)
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(user_from_row).collect()
    }

    /// Every user who can trade skills with `id`, strongest match first.
    ///
    /// ponytail: loads the table and pairs up in Rust. The two-way rule needs
    /// both users' skill lists compared element-wise, which is ugly in portable
    /// SQL; revisit if the member count stops fitting comfortably in memory.
    pub async fn swaps_for(&self, id: Uuid) -> Result<Vec<Swap>, sqlx::Error> {
        let Some(viewer) = self.get(id).await? else {
            return Ok(Vec::new());
        };
        let mut swaps: Vec<Swap> = self
            .all()
            .await?
            .iter()
            .filter_map(|candidate| Swap::between(&viewer, candidate))
            .collect();
        swaps.sort_by(|a, b| {
            b.strength()
                .cmp(&a.strength())
                .then_with(|| a.member.name.cmp(&b.member.name))
        });
        Ok(swaps)
    }

    /// Number of distinct skills anyone has offered or asked for.
    pub async fn distinct_skill_count(&self) -> Result<usize, sqlx::Error> {
        let mut keys: Vec<String> = self
            .all()
            .await?
            .iter()
            .flat_map(|u| u.teaching.iter().chain(&u.learning))
            .map(|s| s.key.clone())
            .collect();
        keys.sort_unstable();
        keys.dedup();
        Ok(keys.len())
    }

    /// The strongest swaps anywhere in the community, for the landing page.
    pub async fn featured_swaps(&self, limit: usize) -> Result<Vec<(User, Swap)>, sqlx::Error> {
        let users = self.all().await?;
        let mut pairs: Vec<(User, Swap)> = Vec::new();
        for (i, viewer) in users.iter().enumerate() {
            for candidate in users.iter().skip(i + 1) {
                if let Some(swap) = Swap::between(viewer, candidate) {
                    pairs.push((viewer.clone(), swap));
                }
            }
        }
        pairs.sort_by_key(|p| std::cmp::Reverse(p.1.strength()));
        pairs.truncate(limit);
        Ok(pairs)
    }

    /// Creates an account. Emails are stored lowercased so the UNIQUE index
    /// makes them case-insensitive.
    pub async fn register(&self, account: NewAccount<'_>) -> Result<Uuid, RegisterError> {
        let email = account.email.trim().to_lowercase();
        let hash = hash_password(account.password).map_err(RegisterError::Hash)?;
        let id = Uuid::new_v4();

        let sql = self.sql(
            "INSERT INTO users (id, email, password_hash, name, headline, bio, teaching, learning) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        );
        let result = sqlx::query(&sql)
            .bind(id.to_string())
            .bind(email)
            .bind(hash)
            .bind(account.name.trim())
            .bind(account.headline.trim())
            .bind(account.bio.trim())
            .bind(Skill::join(&Skill::parse_list(account.teaching)))
            .bind(Skill::join(&Skill::parse_list(account.learning)))
            .execute(&self.pool)
            .await;

        match result {
            Ok(_) => Ok(id),
            // Both backends report a UNIQUE/duplicate-key violation here. There
            // is a race between checking and inserting, so the constraint is
            // what actually enforces uniqueness -- this just names the error.
            Err(e) if is_unique_violation(&e) => Err(RegisterError::EmailTaken),
            Err(e) => Err(RegisterError::Database(e)),
        }
    }

    /// Returns the user's id when the email and password match.
    pub async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let sql = self.sql("SELECT id, password_hash FROM users WHERE email = ?");
        let row = sqlx::query(&sql)
            .bind(email.trim().to_lowercase())
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            // ponytail: returns early for an unknown email, so a wrong address
            // answers faster than a wrong password. Hash a dummy string here if
            // that timing difference ever needs closing.
            return Ok(None);
        };
        if !verify_password(
            password,
            row.try_get::<String, _>("password_hash")?.as_str(),
        ) {
            return Ok(None);
        }
        parse_id(&row.try_get::<String, _>("id")?).map(Some)
    }

    pub async fn email_exists(&self, email: &str) -> Result<bool, sqlx::Error> {
        let sql = self.sql("SELECT 1 AS hit FROM users WHERE email = ?");
        Ok(sqlx::query(&sql)
            .bind(email.trim().to_lowercase())
            .fetch_optional(&self.pool)
            .await?
            .is_some())
    }

    /// Fills an empty database with sample members so a fresh clone has
    /// something to show. Every seeded account shares `SEED_PASSWORD`.
    pub async fn seed_if_empty(&self) -> Result<bool, RegisterError> {
        if self.count().await? > 0 {
            return Ok(false);
        }
        for (email, name, headline, bio, teaching, learning) in SEED_USERS {
            self.register(NewAccount {
                email,
                password: SEED_PASSWORD,
                name,
                headline,
                bio,
                teaching,
                learning,
            })
            .await?;
        }
        Ok(true)
    }
}

/// Shared password for the seeded demo accounts. Development convenience only.
pub const SEED_PASSWORD: &str = "learnswap";

type SeedUser = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const SEED_USERS: [SeedUser; 5] = [
    (
        "ana@example.edu",
        "Ana Ruiz",
        "CS junior, weekend ceramicist",
        "Happy to walk through data structures homework. I learn best by explaining things out loud, so teaching is genuinely useful to me too.",
        "Rust, Data Structures, Pottery",
        "Spanish, Guitar",
    ),
    (
        "marcus@example.edu",
        "Marcus Bell",
        "Music ed major",
        "I have taught guitar since high school. Trying to get comfortable enough with code to build a practice-tracking app.",
        "Guitar, Music Theory",
        "Rust, Web Development",
    ),
    (
        "priya@example.edu",
        "Priya Nair",
        "Bilingual, learning to cook properly",
        "Native Spanish and Hindi speaker. I can do conversation practice any weekday evening.",
        "Spanish, Hindi",
        "Baking, Data Structures",
    ),
    (
        "tom@example.edu",
        "Tom Okafor",
        "Line cook turned CS student",
        "Six years in restaurant kitchens. Ask me about bread.",
        "Baking, Knife Skills",
        "Pottery, Music Theory",
    ),
    (
        "lena@example.edu",
        "Lena Fischer",
        "Front-end dev, absolute beginner at pottery",
        "I can get you from zero to a deployed web page in an afternoon.",
        "Web Development, CSS",
        "Pottery, Knife Skills",
    ),
];

fn user_from_row(row: &sqlx::any::AnyRow) -> Result<User, sqlx::Error> {
    Ok(User {
        id: parse_id(&row.try_get::<String, _>("id")?)?,
        email: row.try_get("email")?,
        name: row.try_get("name")?,
        headline: row.try_get("headline")?,
        bio: row.try_get("bio")?,
        teaching: Skill::parse_list(&row.try_get::<String, _>("teaching")?),
        learning: Skill::parse_list(&row.try_get::<String, _>("learning")?),
    })
}

/// Ids are stored as TEXT so the schema is identical on both backends; a value
/// that will not parse means the row was written by something other than us.
fn parse_id(raw: &str) -> Result<Uuid, sqlx::Error> {
    Uuid::parse_str(raw).map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            // Postgres signals unique violations with SQLSTATE 23505; SQLite
            // has no SQLSTATE here, so fall back to its message.
            db_err.code().as_deref() == Some("23505")
                || db_err.message().to_lowercase().contains("unique")
        }
        _ => false,
    }
}

/// Escapes the wildcards `LIKE` would otherwise interpret, so searching for
/// "100%" does not match everything.
fn escape_like(needle: &str) -> String {
    needle
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_wildcards_are_escaped() {
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("plain"), "plain");
    }
}
