//! SQLite storage. Synchronous: always called from blocking threads.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::model::{Flexibility, Item, Product};

/// The whole V1 schema, created at once: the offers, readings, baskets and
/// purchases tables are filled by later features, but exist from day one so no
/// migration is ever needed between them. Money is `INTEGER` cents, times are
/// milliseconds since the Unix epoch.
const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS products (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    detail         TEXT NOT NULL DEFAULT '',
    flexibility    TEXT NOT NULL DEFAULT 'strict'
                   CHECK (flexibility IN ('strict', 'format', 'brand', 'cheapest')),
    fixed_provider TEXT,
    aisle          TEXT
);
-- One sheet per (product, drive); 'pending' until a human confirms it.
CREATE TABLE IF NOT EXISTS offers (
    id           INTEGER PRIMARY KEY,
    product      TEXT NOT NULL,
    provider     TEXT NOT NULL,
    reference    TEXT NOT NULL,
    label        TEXT NOT NULL DEFAULT '',
    ean          TEXT,
    unit_qty     REAL,
    unit         TEXT,
    status       TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed')),
    confirmed_by TEXT,
    confirmed_at INTEGER,
    UNIQUE (product, provider)
);
CREATE TABLE IF NOT EXISTS items (
    id         INTEGER PRIMARY KEY,
    product    TEXT,
    text       TEXT NOT NULL,
    qty        REAL NOT NULL,
    added_by   TEXT NOT NULL,
    channel    TEXT NOT NULL,
    added_at   INTEGER NOT NULL,
    removed_at INTEGER,
    removed_by TEXT,
    done_at    INTEGER,
    basket     INTEGER
);
CREATE INDEX IF NOT EXISTS items_open ON items(product) WHERE removed_at IS NULL AND done_at IS NULL;
-- A price seen in a store at a given time.
CREATE TABLE IF NOT EXISTS readings (
    id               INTEGER PRIMARY KEY,
    offer            INTEGER NOT NULL REFERENCES offers(id),
    store            TEXT NOT NULL,
    price_cents      INTEGER NOT NULL,
    unit_price_cents INTEGER,
    promo            INTEGER NOT NULL DEFAULT 0,
    available        INTEGER NOT NULL DEFAULT 1,
    read_at          INTEGER NOT NULL,
    read_by          TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS readings_offer ON readings(offer, store, read_at);
CREATE TABLE IF NOT EXISTS providers (
    id              TEXT PRIMARY KEY,
    enabled         INTEGER NOT NULL DEFAULT 1,
    store           TEXT NOT NULL DEFAULT '',
    min_order_cents INTEGER NOT NULL DEFAULT 0,
    fee_cents       INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS baskets (
    id         INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    status     TEXT NOT NULL,
    options    TEXT NOT NULL DEFAULT '[]',   -- JSON
    deviations TEXT NOT NULL DEFAULT '[]',   -- JSON
    chosen     TEXT,
    accepted   TEXT NOT NULL DEFAULT '[]',   -- JSON
    chosen_by  TEXT,
    chosen_at  INTEGER
);
-- How many times a deviation of a kind was refused for a product.
CREATE TABLE IF NOT EXISTS refusals (
    product TEXT NOT NULL,
    kind    TEXT NOT NULL,
    count   INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (product, kind)
);
CREATE TABLE IF NOT EXISTS purchases (
    id          INTEGER PRIMARY KEY,
    product     TEXT NOT NULL,
    offer       INTEGER,
    provider    TEXT NOT NULL,
    store       TEXT NOT NULL,
    qty         REAL NOT NULL,
    price_cents INTEGER NOT NULL,
    at          INTEGER NOT NULL,
    source      TEXT NOT NULL,
    basket      INTEGER,
    status      TEXT NOT NULL DEFAULT 'confirmed'
);
";

const ITEM_COLUMNS: &str = "id, product, text, qty, added_by, channel, added_at";
/// An item is on the list while neither removed nor bought.
const OPEN: &str = "removed_at IS NULL AND done_at IS NULL";

#[derive(Debug, thiserror::Error)]
pub enum PantryError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}

pub struct Store {
    path: PathBuf,
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn ts(ms: u64) -> i64 {
    i64::try_from(ms).unwrap_or(i64::MAX)
}

#[allow(clippy::cast_sign_loss)]
fn ms(ts: i64) -> u64 {
    ts.max(0) as u64
}

/// Two free-text lines are the same need when their words match once trimmed
/// and lowercased.
fn same_words(text: &str) -> String {
    text.trim().to_lowercase()
}

fn item_row(r: &Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: r.get(0)?,
        product: r.get(1)?,
        text: r.get(2)?,
        qty: r.get(3)?,
        added_by: r.get(4)?,
        channel: r.get(5)?,
        added_at: ms(r.get(6)?),
    })
}

fn product_row(r: &Row<'_>) -> rusqlite::Result<Product> {
    let flexibility: String = r.get(3)?;
    Ok(Product {
        id: r.get(0)?,
        name: r.get(1)?,
        detail: r.get(2)?,
        flexibility: Flexibility::parse(&flexibility).unwrap_or_default(),
        fixed_provider: r.get(4)?,
        aisle: r.get(5)?,
    })
}

fn check_qty(qty: f64) -> Result<(), PantryError> {
    if qty.is_finite() && qty > 0.0 {
        Ok(())
    } else {
        Err(PantryError::Invalid(
            "the quantity must be a number above zero".into(),
        ))
    }
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, PantryError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| {
                PantryError::Invalid(format!("cannot create {}: {e}", dir.display()))
            })?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            path: path.to_path_buf(),
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds a need to the list. When the same need is already open (same
    /// product, or same words for a free-text line) its quantity grows and the
    /// first requester stays the author: two people asking for diapers at the
    /// same time make one line.
    pub fn add_item(
        &self,
        text: &str,
        product: Option<&str>,
        qty: f64,
        by: &str,
        channel: &str,
    ) -> Result<Item, PantryError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(PantryError::Invalid("the item needs a text".into()));
        }
        check_qty(qty)?;
        // One writer at a time: the lookup and the write cannot be interleaved
        // with another request for the same need.
        let mut conn = self.lock();
        let tx = conn.transaction()?;
        let existing: Option<i64> = if let Some(product) = product {
            tx.query_row(
                &format!("SELECT id FROM items WHERE product = ?1 AND {OPEN} ORDER BY id LIMIT 1"),
                [product],
                |r| r.get(0),
            )
            .optional()?
        } else {
            let wanted = same_words(text);
            let mut stmt = tx.prepare(&format!(
                "SELECT id, text FROM items WHERE product IS NULL AND {OPEN} ORDER BY id"
            ))?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
            let mut found = None;
            for row in rows {
                let (id, other) = row?;
                if same_words(&other) == wanted {
                    found = Some(id);
                    break;
                }
            }
            found
        };
        let id = if let Some(id) = existing {
            tx.execute(
                "UPDATE items SET qty = qty + ?2 WHERE id = ?1",
                params![id, qty],
            )?;
            id
        } else {
            tx.execute(
                "INSERT INTO items(product, text, qty, added_by, channel, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![product, text, qty, by, channel, ts(now_ms())],
            )?;
            tx.last_insert_rowid()
        };
        let item = tx.query_row(
            &format!("SELECT {ITEM_COLUMNS} FROM items WHERE id = ?1"),
            [id],
            item_row,
        )?;
        tx.commit()?;
        Ok(item)
    }

    /// Takes a line off the list. It stays in the database (statistics).
    pub fn remove_item(&self, id: i64, by: &str) -> Result<(), PantryError> {
        let n = self.lock().execute(
            &format!("UPDATE items SET removed_at = ?2, removed_by = ?3 WHERE id = ?1 AND {OPEN}"),
            params![id, ts(now_ms()), by],
        )?;
        if n == 0 {
            return Err(PantryError::NotFound(format!("item {id}")));
        }
        Ok(())
    }

    pub fn set_qty(&self, id: i64, qty: f64) -> Result<Item, PantryError> {
        check_qty(qty)?;
        let conn = self.lock();
        let n = conn.execute(
            &format!("UPDATE items SET qty = ?2 WHERE id = ?1 AND {OPEN}"),
            params![id, qty],
        )?;
        if n == 0 {
            return Err(PantryError::NotFound(format!("item {id}")));
        }
        Ok(conn.query_row(
            &format!("SELECT {ITEM_COLUMNS} FROM items WHERE id = ?1"),
            [id],
            item_row,
        )?)
    }

    /// The shopping list: lines neither removed nor bought, oldest first.
    pub fn open_items(&self) -> Result<Vec<Item>, PantryError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(&format!(
            "SELECT {ITEM_COLUMNS} FROM items WHERE {OPEN} ORDER BY id"
        ))?;
        Ok(stmt
            .query_map([], item_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Creates a product, or replaces the one with the same id.
    pub fn save_product(&self, p: &Product) -> Result<(), PantryError> {
        if p.id.trim().is_empty() || p.name.trim().is_empty() {
            return Err(PantryError::Invalid(
                "a product needs an id and a name".into(),
            ));
        }
        self.lock().execute(
            "INSERT INTO products(id, name, detail, flexibility, fixed_provider, aisle)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, detail = excluded.detail,
                 flexibility = excluded.flexibility, fixed_provider = excluded.fixed_provider,
                 aisle = excluded.aisle",
            params![
                p.id,
                p.name,
                p.detail,
                p.flexibility.as_str(),
                p.fixed_provider,
                p.aisle
            ],
        )?;
        Ok(())
    }

    pub fn products(&self) -> Result<Vec<Product>, PantryError> {
        let conn = self.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, detail, flexibility, fixed_provider, aisle FROM products
             ORDER BY name COLLATE NOCASE, id",
        )?;
        Ok(stmt
            .query_map([], product_row)?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::model::{Flexibility, Product};

    fn store(name: &str) -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!("moli-pantry-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("pantry.db");
        (Store::open(&path).unwrap(), dir)
    }

    #[test]
    fn adding_twice_the_same_product_raises_the_quantity() {
        let (store, dir) = store("same-product");
        store
            .add_item("couches", Some("couches"), 1.0, "Élodie", "ui")
            .unwrap();
        store
            .add_item("Couches", Some("couches"), 1.0, "Sam", "agent")
            .unwrap();
        let items = store.open_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].qty, 2.0);
        assert_eq!(items[0].added_by, "Élodie");
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn free_text_items_merge_when_the_same_words() {
        let (store, dir) = store("free-text");
        store
            .add_item("lessive", None, 1.0, "Élodie", "ui")
            .unwrap();
        store.add_item("Lessive  ", None, 1.0, "Sam", "ui").unwrap();
        let items = store.open_items().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].qty, 2.0);
        store
            .add_item("lessive bébé", None, 1.0, "Sam", "ui")
            .unwrap();
        assert_eq!(store.open_items().unwrap().len(), 2);
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn removed_items_leave_the_list() {
        let (store, dir) = store("removed");
        let item = store.add_item("lait", None, 1.0, "Élodie", "ui").unwrap();
        store.remove_item(item.id, "Sam").unwrap();
        assert!(store.open_items().unwrap().is_empty());
        // The line stays in the database, for the statistics.
        let removed: Option<i64> = store
            .lock()
            .query_row(
                "SELECT removed_at FROM items WHERE id = ?1",
                [item.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(removed.is_some());
        // Removing twice, or an unknown line, is a not-found.
        assert!(matches!(
            store.remove_item(item.id, "Sam"),
            Err(PantryError::NotFound(_))
        ));
        // A removed line does not absorb a new request.
        let again = store.add_item("lait", None, 1.0, "Sam", "ui").unwrap();
        assert_ne!(again.id, item.id);
        assert_eq!(again.qty, 1.0);
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn products_round_trip() {
        let (store, dir) = store("products");
        let plain = Product {
            id: "couches".into(),
            name: "Couches taille 4".into(),
            ..Product::default()
        };
        let picky = Product {
            id: "lait".into(),
            name: "Lait demi-écrémé".into(),
            detail: "1 L, UHT".into(),
            flexibility: Flexibility::Brand,
            fixed_provider: Some("ville-a".into()),
            aisle: Some("crèmerie".into()),
        };
        store.save_product(&plain).unwrap();
        store.save_product(&picky).unwrap();
        let mut got = store.products().unwrap();
        got.sort_by(|a, b| a.id.cmp(&b.id));
        assert_eq!(got, vec![plain.clone(), picky]);
        assert_eq!(got[0].flexibility, Flexibility::Strict);
        // Saving again replaces.
        let renamed = Product {
            name: "Couches T4".into(),
            ..plain
        };
        store.save_product(&renamed).unwrap();
        assert_eq!(store.products().unwrap().len(), 2);
        assert!(store.products().unwrap().contains(&renamed));
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn set_qty_rejects_zero_or_negative() {
        let (store, dir) = store("qty");
        let item = store.add_item("pâtes", None, 1.0, "Sam", "ui").unwrap();
        assert!(matches!(
            store.set_qty(item.id, 0.0),
            Err(PantryError::Invalid(_))
        ));
        assert!(matches!(
            store.set_qty(item.id, -1.0),
            Err(PantryError::Invalid(_))
        ));
        assert!(matches!(
            store.set_qty(item.id, f64::NAN),
            Err(PantryError::Invalid(_))
        ));
        assert_eq!(store.set_qty(item.id, 3.0).unwrap().qty, 3.0);
        assert!(matches!(
            store.set_qty(999, 1.0),
            Err(PantryError::NotFound(_))
        ));
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_input_is_refused() {
        let (store, dir) = store("invalid");
        assert!(matches!(
            store.add_item("  ", None, 1.0, "Sam", "ui"),
            Err(PantryError::Invalid(_))
        ));
        assert!(matches!(
            store.add_item("riz", None, 0.0, "Sam", "ui"),
            Err(PantryError::Invalid(_))
        ));
        assert!(matches!(
            store.save_product(&Product::default()),
            Err(PantryError::Invalid(_))
        ));
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_whole_v1_schema_exists_from_the_start() {
        let (store, dir) = store("schema");
        let names: Vec<String> = {
            let conn = store.lock();
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        for table in [
            "baskets",
            "items",
            "offers",
            "products",
            "providers",
            "purchases",
            "readings",
            "refusals",
            "settings",
        ] {
            assert!(
                names.iter().any(|n| n == table),
                "{table} missing: {names:?}"
            );
        }
        // Reopening an existing database keeps its content.
        store.add_item("sel", None, 1.0, "Sam", "ui").unwrap();
        let path = dir.join("pantry.db");
        drop(store);
        let again = Store::open(&path).unwrap();
        assert_eq!(again.open_items().unwrap().len(), 1);
        drop(again);
        let _ = std::fs::remove_dir_all(dir);
    }
}
