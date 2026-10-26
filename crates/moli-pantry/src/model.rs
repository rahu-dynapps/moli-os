//! Serializable types shared by the store, the REST routes and the MCP tools.

use serde::{Deserialize, Serialize};

/// How far a product may be swapped for another when a drive lacks it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flexibility {
    /// Exactly this product, nothing else.
    #[default]
    Strict,
    /// Same product, another pack size.
    Format,
    /// Same kind of product, another brand.
    Brand,
    /// Whatever is cheapest.
    Cheapest,
}

impl Flexibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Format => "format",
            Self::Brand => "brand",
            Self::Cheapest => "cheapest",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "strict" => Some(Self::Strict),
            "format" => Some(Self::Format),
            "brand" => Some(Self::Brand),
            "cheapest" => Some(Self::Cheapest),
            _ => None,
        }
    }
}

/// Something the household buys again and again.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub flexibility: Flexibility,
    /// Always bought from this drive, whatever the comparison says.
    #[serde(default)]
    pub fixed_provider: Option<String>,
    #[serde(default)]
    pub aisle: Option<String>,
}

/// One line of the shopping list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    /// Known product, if the line was matched to one.
    pub product: Option<String>,
    pub text: String,
    pub qty: f64,
    /// Who first asked for it (a person, or an agent's name).
    pub added_by: String,
    /// Where it came from: `ui`, `agent`, `telegram`…
    pub channel: String,
    pub added_at: u64,
}

/// Whether a human has checked that a store sheet is the right product.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferStatus {
    /// Found by an agent, not checked yet.
    #[default]
    Pending,
    /// A human confirmed it is the right product.
    Confirmed,
}

impl OfferStatus {
    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "confirmed" => Some(Self::Confirmed),
            _ => None,
        }
    }
}

/// A product as one store chain sells it (« fiche par enseigne »).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Offer {
    #[serde(default)]
    pub id: i64,
    pub product: String,
    /// Chain id, see [`crate::PROVIDERS`].
    pub provider: String,
    /// The chain's own reference for the product.
    pub reference: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub ean: Option<String>,
    /// Quantity in the pack, to compare unit prices.
    #[serde(default)]
    pub unit_qty: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub status: OfferStatus,
}

/// A price seen for an offer in a store, at a given time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reading {
    /// The offer (sheet) id.
    pub offer: i64,
    /// The store the price applies to (a drive has one per site).
    pub store: String,
    pub price_cents: i64,
    #[serde(default)]
    pub unit_price_cents: Option<i64>,
    /// Promotion label, when the price is a special offer.
    #[serde(default)]
    pub promo: Option<String>,
    pub available: bool,
    pub read_at: u64,
    /// Who read it (a person, or an agent's name).
    pub read_by: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// Groceries collected at a store.
    Drive,
    /// Delivered by post.
    Online,
}

/// A store chain the Cellier knows how to compare.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ProviderKind,
    pub site: &'static str,
}

/// What the household set for a chain.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub id: String,
    pub enabled: bool,
    /// The store used in this chain (a drive has one per site).
    #[serde(default)]
    pub store: String,
    #[serde(default)]
    pub min_order_cents: i64,
    #[serde(default)]
    pub fee_cents: i64,
}
