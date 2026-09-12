//! SQLite repositories for ChemLab (users + sessions).

use chrono::{DateTime, Duration, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

pub type DbPool = Pool<Sqlite>;

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("user not found")]
    UserNotFound,
    #[error("email already registered")]
    EmailTaken,
    #[error("session not found or expired")]
    SessionInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Open (or create) a SQLite database and run migrations.
pub async fn connect(database_url: &str) -> Result<DbPool, DbError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Convenience helper for tests and local defaults.
pub async fn connect_file(path: impl AsRef<Path>) -> Result<DbPool, DbError> {
    let url = format!("sqlite://{}", path.as_ref().display());
    connect(&url).await
}

pub async fn insert_user(
    pool: &DbPool,
    email: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<UserRecord, DbError> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now();

    let result = sqlx::query(
        r#"
        INSERT INTO users (id, email, display_name, password_hash, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await;

    match result {
        Ok(_) => Ok(UserRecord {
            id,
            email: email.to_string(),
            display_name: display_name.to_string(),
            password_hash: password_hash.to_string(),
            created_at,
        }),
        Err(sqlx::Error::Database(err)) if err.is_unique_violation() => Err(DbError::EmailTaken),
        Err(err) => Err(DbError::Sqlx(err)),
    }
}

pub async fn find_user_by_email(pool: &DbPool, email: &str) -> Result<UserRecord, DbError> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, display_name, password_hash, created_at
        FROM users WHERE email = ? COLLATE NOCASE
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::UserNotFound)?;

    Ok(row.into())
}

pub async fn find_user_by_id(pool: &DbPool, id: &str) -> Result<UserRecord, DbError> {
    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT id, email, display_name, password_hash, created_at
        FROM users WHERE id = ?
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::UserNotFound)?;

    Ok(row.into())
}

pub async fn create_session(
    pool: &DbPool,
    user_id: &str,
    token_hash: &str,
    ttl: Duration,
) -> Result<SessionRecord, DbError> {
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    let expires_at = created_at + ttl;

    sqlx::query(
        r#"
        INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(token_hash)
    .bind(expires_at.to_rfc3339())
    .bind(created_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(SessionRecord {
        id,
        user_id: user_id.to_string(),
        token_hash: token_hash.to_string(),
        expires_at,
        created_at,
    })
}

pub async fn find_valid_session_by_token_hash(
    pool: &DbPool,
    token_hash: &str,
) -> Result<(SessionRecord, UserRecord), DbError> {
    let now = Utc::now().to_rfc3339();
    let row = sqlx::query_as::<_, SessionUserRow>(
        r#"
        SELECT
            s.id as session_id,
            s.user_id,
            s.token_hash,
            s.expires_at,
            s.created_at as session_created_at,
            u.id as user_id_full,
            u.email,
            u.display_name,
            u.password_hash,
            u.created_at as user_created_at
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        WHERE s.token_hash = ? AND s.expires_at > ?
        "#,
    )
    .bind(token_hash)
    .bind(now)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::SessionInvalid)?;

    Ok(row.into())
}

pub async fn delete_session_by_token_hash(pool: &DbPool, token_hash: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// Drop every session for this user so login can issue a replacement token.
pub async fn delete_sessions_for_user(pool: &DbPool, user_id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    email: String,
    display_name: String,
    password_hash: String,
    created_at: String,
}

impl From<UserRow> for UserRecord {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            password_hash: row.password_hash,
            created_at: parse_dt(&row.created_at),
        }
    }
}

#[derive(sqlx::FromRow)]
struct SessionUserRow {
    session_id: String,
    user_id: String,
    token_hash: String,
    expires_at: String,
    session_created_at: String,
    #[allow(dead_code)]
    user_id_full: String,
    email: String,
    display_name: String,
    password_hash: String,
    user_created_at: String,
}

impl From<SessionUserRow> for (SessionRecord, UserRecord) {
    fn from(row: SessionUserRow) -> Self {
        let session = SessionRecord {
            id: row.session_id,
            user_id: row.user_id.clone(),
            token_hash: row.token_hash,
            expires_at: parse_dt(&row.expires_at),
            created_at: parse_dt(&row.session_created_at),
        };
        let user = UserRecord {
            id: row.user_id,
            email: row.email,
            display_name: row.display_name,
            password_hash: row.password_hash,
            created_at: parse_dt(&row.user_created_at),
        };
        (session, user)
    }
}

fn parse_dt(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> DbPool {
        // Shared-cache in-memory DB so the pool's connections see the same schema.
        connect("sqlite::memory:?cache=shared")
            .await
            .expect("connect")
    }

    #[tokio::test]
    async fn migrations_create_users_and_sessions() {
        let pool = test_pool().await;
        let user = insert_user(&pool, "lab@chemlab.local", "Lab Rat", "hash")
            .await
            .unwrap();
        assert_eq!(user.email, "lab@chemlab.local");

        let session = create_session(&pool, &user.id, "tokhash", Duration::hours(24))
            .await
            .unwrap();
        assert_eq!(session.user_id, user.id);

        let (found_session, found_user) = find_valid_session_by_token_hash(&pool, "tokhash")
            .await
            .unwrap();
        assert_eq!(found_session.id, session.id);
        assert_eq!(found_user.email, "lab@chemlab.local");
    }

    #[tokio::test]
    async fn duplicate_email_is_rejected() {
        let pool = test_pool().await;
        insert_user(&pool, "dup@chemlab.local", "One", "hash")
            .await
            .unwrap();
        let err = insert_user(&pool, "DUP@chemlab.local", "Two", "hash")
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::EmailTaken));
    }

    #[tokio::test]
    async fn delete_sessions_for_user_rejects_old_and_allows_new() {
        let pool = test_pool().await;
        let user = insert_user(&pool, "lab@chemlab.local", "Lab Rat", "hash")
            .await
            .unwrap();
        create_session(&pool, &user.id, "oldhash", Duration::hours(24))
            .await
            .unwrap();

        delete_sessions_for_user(&pool, &user.id).await.unwrap();
        let err = find_valid_session_by_token_hash(&pool, "oldhash")
            .await
            .unwrap_err();
        assert!(matches!(err, DbError::SessionInvalid));

        let session = create_session(&pool, &user.id, "newhash", Duration::hours(24))
            .await
            .unwrap();
        let (found, found_user) = find_valid_session_by_token_hash(&pool, "newhash")
            .await
            .unwrap();
        assert_eq!(found.id, session.id);
        assert_eq!(found_user.id, user.id);
    }
}
