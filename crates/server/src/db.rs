use excaildraw_core::ExcalidrawFile;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::{info, warn};

pub struct Database {
    pool: Option<PgPool>,
}

impl Database {
    pub async fn connect() -> Self {
        let url = std::env::var("DATABASE_URL").ok();
        let Some(url) = url else {
            warn!("DATABASE_URL not set; using in-memory storage only");
            return Self { pool: None };
        };

        match PgPoolOptions::new().max_connections(5).connect(&url).await {
            Ok(pool) => {
                if let Err(e) = migrate(&pool).await {
                    warn!("database migration failed: {e}");
                    return Self { pool: None };
                }
                info!("connected to PostgreSQL");
                Self { pool: Some(pool) }
            }
            Err(e) => {
                warn!("failed to connect to PostgreSQL: {e}; using in-memory storage");
                Self { pool: None }
            }
        }
    }

    pub async fn load_room(&self, id: &str) -> Option<ExcalidrawFile> {
        let pool = self.pool.as_ref()?;
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT data FROM rooms WHERE id = $1::uuid",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()??;

        serde_json::from_value(row).ok()
    }

    pub async fn save_room(&self, id: &str, file: &ExcalidrawFile) {
        let Some(pool) = self.pool.as_ref() else {
            return;
        };
        let data = serde_json::to_value(file).unwrap_or_default();
        let _ = sqlx::query(
            r#"
            INSERT INTO rooms (id, data, updated_at)
            VALUES ($1::uuid, $2, NOW())
            ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, updated_at = NOW()
            "#,
        )
        .bind(id)
        .bind(data)
        .execute(pool)
        .await;
    }
}

async fn migrate(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS rooms (
            id UUID PRIMARY KEY,
            data JSONB NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;
    Ok(())
}
