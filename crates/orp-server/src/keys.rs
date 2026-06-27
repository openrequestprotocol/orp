use orp_core::{OrpError, PublicKeyBundle};
use sqlx::PgPool;

pub async fn save_user_key(
    pool: &PgPool,
    email: &str,
    bundle: &PublicKeyBundle,
) -> Result<(), OrpError> {
    sqlx::query(
        r#"INSERT INTO orp_user_keys (email, key_id, alg, public_key)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (email) DO UPDATE SET
             key_id = excluded.key_id,
             alg = excluded.alg,
             public_key = excluded.public_key"#,
    )
    .bind(email)
    .bind(&bundle.key_id)
    .bind(&bundle.alg)
    .bind(&bundle.value)
    .execute(pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(())
}

pub async fn load_user_key(pool: &PgPool, email: &str) -> Result<Option<PublicKeyBundle>, OrpError> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT key_id, alg, public_key FROM orp_user_keys WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(row.map(|(key_id, alg, value)| PublicKeyBundle { key_id, alg, value }))
}

pub async fn list_user_keys(pool: &PgPool) -> Result<Vec<PublicKeyBundle>, OrpError> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT key_id, alg, public_key FROM orp_user_keys ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| OrpError::Serialization(e.to_string()))?;
    Ok(rows
        .into_iter()
        .map(|(key_id, alg, value)| PublicKeyBundle { key_id, alg, value })
        .collect())
}

pub async fn resolve_verify_keys(
    pool: &PgPool,
    server_keys: &[PublicKeyBundle],
    sender_email: &str,
    key_id: &str,
) -> Result<Vec<PublicKeyBundle>, OrpError> {
    let mut keys: Vec<PublicKeyBundle> = server_keys
        .iter()
        .filter(|k| k.key_id == key_id)
        .cloned()
        .collect();
    if let Some(user_key) = load_user_key(pool, sender_email).await? {
        if user_key.key_id == key_id {
            keys.push(user_key);
        }
    }
    if keys.is_empty() {
        return Err(orp_core::OrpError::UnknownKey(key_id.into()));
    }
    Ok(keys)
}

pub async fn discovery_public_keys(
    pool: &PgPool,
    server_keys: &[PublicKeyBundle],
) -> Result<Vec<PublicKeyBundle>, OrpError> {
    let mut out = server_keys.to_vec();
    for user_key in list_user_keys(pool).await? {
        if !out.iter().any(|k| k.key_id == user_key.key_id) {
            out.push(user_key);
        }
    }
    Ok(out)
}
