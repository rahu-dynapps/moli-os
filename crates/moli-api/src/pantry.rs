//! `/api/cellier`: the household's pantry (« Le Cellier », crate
//! `moli-pantry`): the shopping list, the products, the price comparison.
//!
//! - `GET /api/cellier`: everything at once (list, products, sheets, chains,
//!   threshold, open basket).
//! - `POST /api/cellier/items {text, product?, qty?}`: add a need.
//! - `PUT|DELETE /api/cellier/items/{id}`: change its quantity / remove it.
//! - `PUT /api/cellier/products/{id}`: create or change a product.
//! - `POST /api/cellier/offers/{id}/confirm`: a human checked the sheet.
//! - `PUT /api/cellier/settings {threshold?, providers?}`: the split
//!   threshold and the chains' settings.
//! - `POST /api/cellier/baskets/{id}/choose {option, accepted}`: pick an
//!   option of the basket.
//! - `POST /api/cellier/baskets/{id}/status {status}`: move the basket on.
//!
//! Without a `[pantry]` section every route answers `404`. Guests and the
//! demo house are kept out by the caller check, before any handler.

use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use moli_pantry::{BasketStatus, Pantry, PantryError, Product, ProviderSettings};
use moli_runtime::Hub;
use serde::Deserialize;
use serde_json::json;

use crate::caller::Caller;

/// The channel of everything done from the dashboard.
const CHANNEL: &str = "ui";

#[derive(Clone)]
pub(crate) struct Cellier(pub(crate) Option<Pantry>);

pub(crate) fn routes(pantry: Option<Pantry>) -> Router<Hub> {
    Router::new()
        .route("/api/cellier", get(overview))
        .route("/api/cellier/items", post(add_item))
        .route("/api/cellier/items/{id}", put(set_qty).delete(remove_item))
        .route("/api/cellier/products/{id}", put(save_product))
        .route("/api/cellier/offers/{id}/confirm", post(confirm_offer))
        .route("/api/cellier/settings", put(settings))
        .route("/api/cellier/baskets/{id}/choose", post(choose))
        .route("/api/cellier/baskets/{id}/status", post(set_status))
        .layer(Extension(Cellier(pantry)))
}

fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

fn absent() -> Response {
    error(StatusCode::NOT_FOUND, "the pantry is not set up ([pantry])")
}

/// A refused or failed order, said the same way everywhere.
fn failed(e: &PantryError) -> Response {
    let status = match e {
        PantryError::NotFound(_) => StatusCode::NOT_FOUND,
        PantryError::Invalid(_) => StatusCode::BAD_REQUEST,
        PantryError::Db(_) | PantryError::Internal(_) => {
            tracing::error!(error = %e, "pantry");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    };
    error(status, &e.to_string())
}

/// A result as JSON, or the error.
fn json_or<T: serde::Serialize>(r: Result<T, PantryError>) -> Response {
    match r {
        Ok(v) => Json(v).into_response(),
        Err(e) => failed(&e),
    }
}

/// A result without a body, or the error.
fn done(r: Result<(), PantryError>) -> Response {
    match r {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => failed(&e),
    }
}

/// Who acts: the person's name when Moli knows who it is, else how the
/// caller is described.
fn actor(caller: &Caller) -> String {
    caller
        .person
        .as_ref()
        .map_or_else(|| caller.describe(), |p| p.name.clone())
}

pub(crate) async fn overview(Extension(Cellier(pantry)): Extension<Cellier>) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    json_or(pantry.overview(moli_core::now_ms()).await)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NewItem {
    text: String,
    #[serde(default)]
    product: Option<String>,
    #[serde(default)]
    qty: Option<f64>,
}

pub(crate) async fn add_item(
    Extension(Cellier(pantry)): Extension<Cellier>,
    caller: Caller,
    Json(body): Json<NewItem>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    let added = pantry
        .add_item(
            &body.text,
            body.product.as_deref(),
            body.qty.unwrap_or(1.0),
            &actor(&caller),
            CHANNEL,
        )
        .await;
    match added {
        Ok(item) => (StatusCode::CREATED, Json(item)).into_response(),
        Err(e) => failed(&e),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Qty {
    qty: f64,
}

pub(crate) async fn set_qty(
    Extension(Cellier(pantry)): Extension<Cellier>,
    Path(id): Path<i64>,
    Json(body): Json<Qty>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    json_or(pantry.set_qty(id, body.qty).await)
}

pub(crate) async fn remove_item(
    Extension(Cellier(pantry)): Extension<Cellier>,
    caller: Caller,
    Path(id): Path<i64>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    done(pantry.remove_item(id, &actor(&caller)).await)
}

pub(crate) async fn save_product(
    Extension(Cellier(pantry)): Extension<Cellier>,
    Path(id): Path<String>,
    Json(product): Json<Product>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    if product.id != id {
        return error(
            StatusCode::BAD_REQUEST,
            "the product's id does not match the address",
        );
    }
    done(pantry.save_product(&product).await)
}

pub(crate) async fn confirm_offer(
    Extension(Cellier(pantry)): Extension<Cellier>,
    caller: Caller,
    Path(id): Path<i64>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    done(pantry.confirm_offer(id, &actor(&caller)).await)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Settings {
    #[serde(default)]
    threshold: Option<f64>,
    #[serde(default)]
    providers: Option<Vec<ProviderSettings>>,
}

pub(crate) async fn settings(
    Extension(Cellier(pantry)): Extension<Cellier>,
    Json(body): Json<Settings>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    if let Some(threshold) = body.threshold
        && let Err(e) = pantry.set_split_threshold(threshold).await
    {
        return failed(&e);
    }
    for provider in body.providers.iter().flatten() {
        if let Err(e) = pantry.save_provider_settings(provider).await {
            return failed(&e);
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Choice {
    option: String,
    #[serde(default)]
    accepted: Vec<usize>,
}

pub(crate) async fn choose(
    Extension(Cellier(pantry)): Extension<Cellier>,
    caller: Caller,
    Path(id): Path<i64>,
    Json(body): Json<Choice>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    json_or(
        pantry
            .choose(id, &body.option, &body.accepted, &actor(&caller))
            .await,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Status {
    status: String,
}

pub(crate) async fn set_status(
    Extension(Cellier(pantry)): Extension<Cellier>,
    caller: Caller,
    Path(id): Path<i64>,
    Json(body): Json<Status>,
) -> Response {
    let Some(pantry) = pantry else {
        return absent();
    };
    let Ok(status) = serde_json::from_value::<BasketStatus>(json!(body.status)) else {
        return error(StatusCode::BAD_REQUEST, "unknown basket status");
    };
    json_or(pantry.set_status(id, status, &actor(&caller)).await)
}
