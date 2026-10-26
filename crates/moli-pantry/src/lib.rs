//! Pantry (« Le Cellier »): the household shopping list, the products it is
//! made of, and the price comparison of grocery drives.
//!
//! Humans (dashboard) and AI agents (MCP) work on the same list. Everything
//! lives in one SQLite file; amounts are `i64` cents and times are `u64`
//! milliseconds since the epoch, like `moli-history`. This crate holds the
//! storage and the pure logic; it opens nothing until the host asks for it.

mod model;
mod providers;
mod store;

pub use model::{
    Flexibility, Item, Offer, OfferStatus, Product, ProviderInfo, ProviderKind, ProviderSettings,
    Reading,
};
pub use providers::PROVIDERS;
pub use store::{PantryError, Store};
