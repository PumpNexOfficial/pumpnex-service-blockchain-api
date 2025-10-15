/// Redis integration

use crate::config::IntegrationsConfig;
use redis::{aio::ConnectionManager, Client};
use std::time::Duration;

pub async fn init_redis(config: &IntegrationsConfig) -> Option<ConnectionManager> {
    if !config.enable_redis {
        tracing::info!("Redis integration disabled");
        return None;
    }

    if config.redis_url.is_empty() {
        tracing::warn!("Redis enabled but redis_url is empty");
        return None;
    }

    tracing::info!(
        redis_url = %config.redis_url.split('@').last().unwrap_or("***"),
        connect_timeout_ms = %config.redis_connect_timeout_ms,
        command_timeout_ms = %config.redis_command_timeout_ms,
        "Initializing Redis connection"
    );

    match Client::open(config.redis_url.as_str()) {
        Ok(client) => {
            match tokio::time::timeout(
                Duration::from_millis(config.redis_connect_timeout_ms),
                ConnectionManager::new(client),
            )
            .await
            {
                Ok(Ok(manager)) => {
                    tracing::info!("Redis connection initialized successfully");
                    Some(manager)
                }
                Ok(Err(e)) => {
                    tracing::error!(error = %e, "Failed to connect to Redis");
                    None
                }
                Err(_) => {
                    tracing::error!("Redis connection timeout");
                    None
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Failed to create Redis client");
            None
        }
    }
}

pub async fn check_redis_health(manager: &mut ConnectionManager) -> Result<(), String> {
    // NOTE: В redis 0.26 у Cmd::query_async только один generic — тип результата.
    // Нужен &mut ConnectionManager (ConnectionLike).
    match redis::cmd("PING")
        .query_async::<String>(manager)
        .await
    {
        Ok(_pong) => Ok(()),
        Err(e) => Err(format!("Redis health check failed: {}", e)),
    }
}

/// Check if IP is in ban list
pub async fn is_in_ban_list(
    manager: &mut ConnectionManager,
    ip: &str,
    key: &str,
) -> Result<bool, String> {
    redis::cmd("SISMEMBER")
        .arg(key)
        .arg(ip)
        .query_async(manager)
        .await
        .map_err(|e| format!("Redis SISMEMBER error: {}", e))
}

/// Check if IP is in grey list
pub async fn is_in_grey_list(
    manager: &mut ConnectionManager,
    ip: &str,
    key: &str,
) -> Result<bool, String> {
    redis::cmd("SISMEMBER")
        .arg(key)
        .arg(ip)
        .query_async(manager)
        .await
        .map_err(|e| format!("Redis SISMEMBER error: {}", e))
}

/// Add IP to ban list
pub async fn add_to_ban_list(
    manager: &mut ConnectionManager,
    ip: &str,
    ttl_secs: u64,
    key: &str,
) -> Result<(), String> {
    redis::pipe()
        .cmd("SADD").arg(key).arg(ip).query_async::<()>(manager).await
        .map_err(|e| format!("Redis SADD error: {}", e))?;
    redis::cmd("EXPIRE")
        .arg(key)
        .arg(ttl_secs)
        .query_async(manager)
        .await
        .map_err(|e| format!("Redis EXPIRE error: {}", e))
}

/// Add IP to grey list
pub async fn add_to_grey_list(
    manager: &mut ConnectionManager,
    ip: &str,
    ttl_secs: u64,
    key: &str,
) -> Result<(), String> {
    redis::pipe()
        .cmd("SADD").arg(key).arg(ip).query_async::<()>(manager).await
        .map_err(|e| format!("Redis SADD error: {}", e))?;
    redis::cmd("EXPIRE")
        .arg(key)
        .arg(ttl_secs)
        .query_async(manager)
        .await
        .map_err(|e| format!("Redis EXPIRE error: {}", e))
}

/// Get set size
pub async fn get_set_size(
    manager: &mut ConnectionManager,
    key: &str,
) -> Result<u64, String> {
    redis::cmd("SCARD")
        .arg(key)
        .query_async(manager)
        .await
        .map_err(|e| format!("Redis SCARD error: {}", e))
}
