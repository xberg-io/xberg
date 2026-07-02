//! Durable, WAL-mode SQLite [`RehydrationStore`].
//!
//! One table, `(tenant, id)` primary key. Blocking `rusqlite` calls are
//! routed through `tokio::task::spawn_blocking`, mirroring the pattern
//! `xberg-rag`'s SQLite backend uses: the connection is `Send` but not
//! `Sync`, so it is wrapped in `Arc<Mutex<Connection>>`, and the mutex is
//! locked only inside the blocking closure — never held across an `.await`.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rusqlite::{Connection, params};

use crate::DocumentId;
use crate::error::{StoreError, StoreResult};
use crate::rehydration::RehydrationStore;
use crate::tenant::TenantCtx;

fn now_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn backend_err(e: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Backend(Box::new(e))
}

/// Durable rehydration map store backed by an on-disk SQLite database in WAL
/// mode. Survives process restart — this is the backend selected when
/// `XBERG_REHYDRATION_DB_PATH` is set.
#[derive(Clone)]
pub struct SqliteRehydrationStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteRehydrationStore {
    /// Open (or create) the database at `path` and ensure the schema exists.
    pub fn open(path: impl AsRef<Path>) -> StoreResult<Self> {
        let conn = Connection::open(path.as_ref()).map_err(backend_err)?;
        Self::init(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    /// Open an in-memory SQLite database. Test-only: not durable across drop
    /// (SQLite's `:memory:` databases are per-connection).
    #[cfg(test)]
    pub fn open_in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory().map_err(backend_err)?;
        Self::init(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    fn init(conn: &Connection) -> StoreResult<()> {
        conn.pragma_update(None, "journal_mode", "WAL").map_err(backend_err)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS rehydration_maps (
                tenant     TEXT NOT NULL,
                id         TEXT NOT NULL,
                blob       BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (tenant, id)
            );",
        )
        .map_err(backend_err)?;
        Ok(())
    }
}

#[async_trait]
impl RehydrationStore for SqliteRehydrationStore {
    async fn put_map(&self, ctx: &TenantCtx, blob: Vec<u8>) -> StoreResult<DocumentId> {
        let id = format!("reh_{}", uuid::Uuid::new_v4());
        let conn = Arc::clone(&self.conn);
        let tenant = ctx.tenant.0.clone();
        let row_id = id.clone();
        let created_at = now_unix();
        tokio::task::spawn_blocking(move || -> StoreResult<()> {
            let conn = conn.lock().expect("rehydration db mutex poisoned");
            conn.execute(
                "INSERT INTO rehydration_maps (tenant, id, blob, created_at) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(tenant, id) DO UPDATE SET blob = excluded.blob, created_at = excluded.created_at",
                params![tenant, row_id, blob, created_at],
            )
            .map_err(backend_err)?;
            Ok(())
        })
        .await
        .map_err(backend_err)??;
        Ok(DocumentId(id))
    }

    async fn get_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<Option<Vec<u8>>> {
        let conn = Arc::clone(&self.conn);
        let tenant = ctx.tenant.0.clone();
        let row_id = id.0.clone();
        tokio::task::spawn_blocking(move || -> StoreResult<Option<Vec<u8>>> {
            let conn = conn.lock().expect("rehydration db mutex poisoned");
            let mut stmt = conn
                .prepare("SELECT blob FROM rehydration_maps WHERE tenant = ?1 AND id = ?2")
                .map_err(backend_err)?;
            let mut rows = stmt.query(params![tenant, row_id]).map_err(backend_err)?;
            match rows.next().map_err(backend_err)? {
                Some(row) => Ok(Some(row.get::<_, Vec<u8>>(0).map_err(backend_err)?)),
                None => Ok(None),
            }
        })
        .await
        .map_err(backend_err)?
    }

    async fn delete_map(&self, ctx: &TenantCtx, id: &DocumentId) -> StoreResult<bool> {
        let conn = Arc::clone(&self.conn);
        let tenant = ctx.tenant.0.clone();
        let row_id = id.0.clone();
        tokio::task::spawn_blocking(move || -> StoreResult<bool> {
            let conn = conn.lock().expect("rehydration db mutex poisoned");
            let changed = conn
                .execute(
                    "DELETE FROM rehydration_maps WHERE tenant = ?1 AND id = ?2",
                    params![tenant, row_id],
                )
                .map_err(backend_err)?;
            Ok(changed > 0)
        })
        .await
        .map_err(backend_err)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_then_get_round_trips() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![1, 2, 3]).await.expect("put");
        assert!(id.0.starts_with("reh_"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get"), Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn get_missing_id_returns_none() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let ctx = TenantCtx::default_tenant();
        let missing = DocumentId("reh_nonexistent".to_string());
        assert_eq!(store.get_map(&ctx, &missing).await.expect("get"), None);
    }

    #[tokio::test]
    async fn different_tenants_do_not_see_each_others_maps() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let tenant_a = TenantCtx::new("tenant-a", "user");
        let tenant_b = TenantCtx::new("tenant-b", "user");
        let id = store.put_map(&tenant_a, vec![9, 9, 9]).await.expect("put");
        assert_eq!(store.get_map(&tenant_a, &id).await.expect("get a"), Some(vec![9, 9, 9]));
        assert_eq!(store.get_map(&tenant_b, &id).await.expect("get b"), None);
    }

    #[tokio::test]
    async fn delete_removes_the_map() {
        let store = SqliteRehydrationStore::open_in_memory().expect("open");
        let ctx = TenantCtx::default_tenant();
        let id = store.put_map(&ctx, vec![7]).await.expect("put");
        assert!(store.delete_map(&ctx, &id).await.expect("delete"));
        assert_eq!(store.get_map(&ctx, &id).await.expect("get after delete"), None);
    }

    /// The money test: a map written by one store instance is still readable
    /// after that instance is dropped and a fresh instance opens the same
    /// on-disk file — i.e. it actually survives a process restart, unlike
    /// the moka in-memory backend.
    #[tokio::test]
    async fn map_survives_store_drop_and_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("rehydration.sqlite3");
        let ctx = TenantCtx::default_tenant();

        let id = {
            let store = SqliteRehydrationStore::open(&db_path).expect("open");
            store.put_map(&ctx, vec![4, 2, 4, 2]).await.expect("put")
        }; // `store` dropped here — simulates process restart.

        let reopened = SqliteRehydrationStore::open(&db_path).expect("reopen");
        assert_eq!(
            reopened.get_map(&ctx, &id).await.expect("get after reopen"),
            Some(vec![4, 2, 4, 2]),
            "map must survive a store drop + reopen against the same file"
        );
    }
}
