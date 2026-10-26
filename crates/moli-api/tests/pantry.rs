//! `/api/cellier`: the pantry's REST routes, and who may use them.

use moli_pantry::Pantry;
use moli_runtime::{Hub, HubOptions};
use serde_json::{Value as Json, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

const PIN: &str = "482915";

async fn serve(hub: &Hub, options: &moli_api::Options) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let app = moli_api::router(hub.clone(), CancellationToken::new(), options);
    let app = app.into_make_service_with_connect_info::<std::net::SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

/// Status, the cookie `name` it sets (if any), body.
async fn request(
    addr: &str,
    method: &str,
    path: &str,
    body: &Json,
    extra: &str,
    name: &str,
) -> (u16, Option<String>, Json) {
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let body = body.to_string();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\
         Content-Type: application/json\r\n{extra}Content-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await.unwrap();
    stream.write_all(body.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let raw = String::from_utf8_lossy(&raw).to_string();
    let status = raw[9..12].parse().unwrap();
    let (headers, body) = raw.split_once("\r\n\r\n").unwrap();
    let cookie = headers.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.eq_ignore_ascii_case("set-cookie") && v.trim().starts_with(&format!("{name}=")))
            .then(|| v.trim().split(';').next().unwrap_or_default().to_owned())
    });
    let body = if headers
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        dechunk(body)
    } else {
        body.to_owned()
    };
    let body = serde_json::from_str(&body).unwrap_or(Json::Null);
    (status, cookie, body)
}

fn dechunk(mut body: &str) -> String {
    let mut out = String::new();
    while let Some((size, rest)) = body.split_once("\r\n") {
        let size = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if size == 0 {
            break;
        }
        out.push_str(&rest[..size]);
        body = rest[size..].trim_start_matches("\r\n");
    }
    out
}

async fn call(addr: &str, method: &str, path: &str, body: &Json, extra: &str) -> (u16, Json) {
    let (status, _, body) = request(addr, method, path, body, extra, "").await;
    (status, body)
}

fn temp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("moli-api-pantry-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A house with a pantry (or without), served on a free port.
async fn house(name: &str, pantry: bool, demo: bool) -> (String, std::path::PathBuf) {
    let dir = temp(name);
    // The people's passwords are kept in the secrets file.
    let hub = Hub::new(HubOptions {
        secrets: Some((
            dir.join("secrets.enc"),
            moli_runtime::MasterKey::generate().unwrap(),
        )),
        ..HubOptions::default()
    })
    .unwrap();
    let options = moli_api::Options {
        ui_pin: Some(moli_api::UiPin::new(PIN)),
        home_path: Some(dir.join("home.json")),
        pantry: pantry.then(|| Pantry::open(&hub, &dir).unwrap()),
        demo,
        ..moli_api::Options::default()
    };
    (serve(&hub, &options).await, dir)
}

#[tokio::test]
async fn add_then_list() {
    let (addr, dir) = house("add", true, false).await;
    let (status, item) = call(
        &addr,
        "POST",
        "/api/cellier/items",
        &json!({ "text": "lait" }),
        "",
    )
    .await;
    assert_eq!(status, 201, "{item}");
    assert_eq!(item["qty"], 1.0);
    assert_eq!(item["channel"], "ui");

    let (status, view) = call(&addr, "GET", "/api/cellier", &Json::Null, "").await;
    assert_eq!(status, 200, "{view}");
    assert_eq!(view["items"][0]["text"], "lait");
    assert!(!view["items"][0]["added_by"].as_str().unwrap().is_empty());
    assert_eq!(view["threshold"], 0.3);

    // Change, then drop it.
    let path = format!("/api/cellier/items/{}", item["id"]);
    let (status, item) = call(&addr, "PUT", &path, &json!({ "qty": 3 }), "").await;
    assert_eq!(status, 200, "{item}");
    assert_eq!(item["qty"], 3.0);
    let (status, _) = call(&addr, "DELETE", &path, &Json::Null, "").await;
    assert_eq!(status, 204);
    let (status, body) = call(&addr, "DELETE", &path, &Json::Null, "").await;
    assert_eq!(status, 404, "{body}");
    assert!(body["error"].is_string());
    let (_, view) = call(&addr, "GET", "/api/cellier", &Json::Null, "").await;
    assert_eq!(view["items"], json!([]));
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn absent_brick_is_404() {
    let (addr, dir) = house("absent", false, false).await;
    let (status, body) = call(&addr, "GET", "/api/cellier", &Json::Null, "").await;
    assert_eq!(status, 404, "{body}");
    assert!(body["error"].is_string());
    let (status, _) = call(
        &addr,
        "POST",
        "/api/cellier/items",
        &json!({ "text": "lait" }),
        "",
    )
    .await;
    assert_eq!(status, 404);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn guest_is_refused() {
    let (addr, dir) = house("guest", true, false).await;
    // The house's code proves the owner, who invites a guest.
    let (status, cookie, body) = request(
        &addr,
        "POST",
        "/api/session",
        &json!({ "pin": PIN }),
        "",
        "moli_session",
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let owner = format!("Cookie: {}\r\n", cookie.unwrap());
    let invited = json!({ "name": "Sam", "role": "guest", "rooms": ["Salon"] });
    let (status, body) = call(&addr, "POST", "/api/people", &invited, &owner).await;
    assert_eq!(status, 201, "{body}");
    let (status, body) = call(&addr, "POST", "/api/people/sam/invite", &Json::Null, &owner).await;
    assert_eq!(status, 200, "{body}");
    let outside = "X-Forwarded-For: 203.0.113.9\r\n";
    let (status, cookie, body) = request(
        &addr,
        "POST",
        "/api/invitation",
        &json!({ "code": body["code"], "password": "une valise bleue" }),
        outside,
        "moli_person",
    )
    .await;
    assert_eq!(status, 200, "{body}");
    let guest = format!(
        "{outside}Cookie: {}\r\nx-moli-origin: ui\r\n",
        cookie.unwrap()
    );

    let (status, body) = call(&addr, "GET", "/api/cellier", &Json::Null, &guest).await;
    assert_eq!(status, 403, "{body}");
    let (status, body) = call(
        &addr,
        "POST",
        "/api/cellier/items",
        &json!({ "text": "lait" }),
        &guest,
    )
    .await;
    assert_eq!(status, 403, "{body}");
    // The household still reads it.
    let (status, _) = call(&addr, "GET", "/api/cellier", &Json::Null, &owner).await;
    assert_eq!(status, 200);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn demo_cannot_write() {
    let (addr, dir) = house("demo", true, true).await;
    let (status, body) = call(
        &addr,
        "POST",
        "/api/cellier/items",
        &json!({ "text": "lait" }),
        "",
    )
    .await;
    assert_eq!(status, 403, "{body}");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn bad_threshold_is_400() {
    let (addr, dir) = house("threshold", true, false).await;
    let (status, body) = call(
        &addr,
        "PUT",
        "/api/cellier/settings",
        &json!({ "threshold": 2 }),
        "",
    )
    .await;
    assert_eq!(status, 400, "{body}");
    assert!(body["error"].is_string());
    let (status, _) = call(
        &addr,
        "PUT",
        "/api/cellier/settings",
        &json!({ "threshold": 0.4 }),
        "",
    )
    .await;
    assert_eq!(status, 204);
    let (_, view) = call(&addr, "GET", "/api/cellier", &Json::Null, "").await;
    assert_eq!(view["threshold"], 0.4);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn products_baskets_and_errors() {
    let (addr, dir) = house("misc", true, false).await;
    let product = json!({ "id": "couches", "name": "Couches", "flexibility": "format" });
    let (status, body) = call(&addr, "PUT", "/api/cellier/products/couches", &product, "").await;
    assert_eq!(status, 204, "{body}");
    let (status, _) = call(&addr, "PUT", "/api/cellier/products/autre", &product, "").await;
    assert_eq!(status, 400, "the id must match the address");
    let (_, view) = call(&addr, "GET", "/api/cellier", &Json::Null, "").await;
    assert_eq!(view["products"][0]["flexibility"], "format");

    let (status, _) = call(
        &addr,
        "POST",
        "/api/cellier/offers/99/confirm",
        &Json::Null,
        "",
    )
    .await;
    assert_eq!(status, 404);
    let (status, _) = call(
        &addr,
        "POST",
        "/api/cellier/baskets/99/status",
        &json!({ "status": "dropped" }),
        "",
    )
    .await;
    assert_eq!(status, 404, "no such basket");
    let (status, body) = call(
        &addr,
        "POST",
        "/api/cellier/baskets/99/status",
        &json!({ "status": "bogus" }),
        "",
    )
    .await;
    assert_eq!(status, 400, "{body}");
    let (status, _) = call(
        &addr,
        "POST",
        "/api/cellier/baskets/99/choose",
        &json!({ "option": "split", "accepted": [] }),
        "",
    )
    .await;
    assert_eq!(status, 404, "no such basket");
    let (status, _) = call(
        &addr,
        "PUT",
        "/api/cellier/settings",
        &json!({ "providers": [{ "id": "nowhere", "enabled": true }] }),
        "",
    )
    .await;
    assert_eq!(status, 400, "unknown chain");
    let _ = std::fs::remove_dir_all(dir);
}
