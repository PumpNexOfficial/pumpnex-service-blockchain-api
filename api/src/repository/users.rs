use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub pubkey: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPermission {
    pub id: Uuid,
    pub pubkey: String,
    pub endpoint: String,
    pub permission: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get user by public key
    pub async fn get_by_pubkey(&self, pubkey: &str) -> Result<Option<User>, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, pubkey, role, created_at
            FROM users
            WHERE pubkey = $1
            "#,
        )
        .bind(pubkey)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Insert user if not exists (idempotent)
    pub async fn insert_if_missing(&self, pubkey: &str, role: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO users (pubkey, role)
            VALUES ($1, $2)
            ON CONFLICT (pubkey) DO NOTHING
            "#,
        )
        .bind(pubkey)
        .bind(role)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all permissions for a user
    pub async fn permissions_for(
        &self,
        pubkey: &str,
    ) -> Result<Vec<UserPermission>, sqlx::Error> {
        let permissions = sqlx::query_as::<_, UserPermission>(
            r#"
            SELECT id, pubkey, endpoint, permission, created_at
            FROM user_permissions
            WHERE pubkey = $1
            ORDER BY endpoint, permission
            "#,
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(permissions)
    }

    /// Add permission for user (idempotent)
    pub async fn add_permission(
        &self,
        pubkey: &str,
        endpoint: &str,
        permission: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO user_permissions (pubkey, endpoint, permission)
            VALUES ($1, $2, $3)
            ON CONFLICT (pubkey, endpoint) DO UPDATE
            SET permission = EXCLUDED.permission
            "#,
        )
        .bind(pubkey)
        .bind(endpoint)
        .bind(permission)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if user has permission for endpoint
    pub async fn has_permission(
        &self,
        pubkey: &str,
        endpoint: &str,
        required_permission: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM user_permissions
            WHERE pubkey = $1 AND endpoint = $2 AND permission = $3
            "#,
        )
        .bind(pubkey)
        .bind(endpoint)
        .bind(required_permission)
        .fetch_one(&self.pool)
        .await?;

        Ok(result > 0)
    }

    /// List all users with pagination
    pub async fn list(&self, limit: i64, offset: i64) -> Result<Vec<User>, sqlx::Error> {
        let users = sqlx::query_as::<_, User>(
            r#"
            SELECT id, pubkey, role, created_at
            FROM users
            ORDER BY created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

