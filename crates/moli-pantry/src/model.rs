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
