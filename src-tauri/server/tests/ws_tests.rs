/// WebSocket E2E tests — two-client lock/unlock/update scenarios.
///
/// We spin up a real Axum server on a random port and connect two
/// tokio-tungstenite clients to it.
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message as TMsg};
use futures::{SinkExt, StreamExt};

static DB_COUNTER: AtomicU64 = AtomicU64::new(200); // offset from other test files

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Start an Axum server on a random port, return (addr, token_a, token_b).
/// Two distinct users are registered so lock-conflict tests work correctly.
async fn start_server() -> (String, String, String) {
    let db_id = DB_COUNTER.fetch_add(1, Ordering::Relaxed);
    let db_name = format!("wstest_{}", db_id);

    let pool =
        server::db::in_memory_pool_named(&db_name).expect("Failed to create in-memory pool");
    server::db::run_migrations(&pool)
        .await
        .expect("Migration failed");

    let config = server::config::Config {
        host: "127.0.0.1".to_string(),
        port: 0, // not used when binding manually
        database_url: format!("file:{}?mode=memory&cache=shared", db_name),
        jwt_secret: "ws-test-secret".to_string(),
        jwt_access_expiry_secs: 1800,
        jwt_refresh_expiry_secs: 604800,
        ws_lock_timeout_secs: 1, // 1-second timeout for fast tests
        allowed_origins: vec![],
        auth_rate_limit_per_min: 1000,
    };

    let router = server::app::build_router(pool, config);

    // Bind to an OS-assigned port.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().expect("No local addr").to_string();
    let addr_clone = addr.clone();

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("Server error");
    });

    let client = reqwest::Client::new();

    // Register user A and get token.
    client
        .post(format!("http://{}/api/auth/register", addr_clone))
        .json(&json!({"username":"wsuser_a","email":"ws_a@test.com","password":"password123"}))
        .send()
        .await
        .unwrap();
    let resp_a: Value = client
        .post(format!("http://{}/api/auth/login", addr_clone))
        .json(&json!({"username":"wsuser_a","password":"password123"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token_a = resp_a["access_token"].as_str().unwrap().to_string();

    // Register user B and get token.
    client
        .post(format!("http://{}/api/auth/register", addr_clone))
        .json(&json!({"username":"wsuser_b","email":"ws_b@test.com","password":"password123"}))
        .send()
        .await
        .unwrap();
    let resp_b: Value = client
        .post(format!("http://{}/api/auth/login", addr_clone))
        .json(&json!({"username":"wsuser_b","password":"password123"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let token_b = resp_b["access_token"].as_str().unwrap().to_string();

    (addr_clone, token_a, token_b)
}

/// Receive the next JSON text message from a WebSocket, skipping non-text frames.
async fn recv_json(
    ws: &mut futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Value {
    loop {
        match ws.next().await.expect("Stream ended").expect("WS error") {
            TMsg::Text(txt) => {
                return serde_json::from_str(&txt).expect("Invalid JSON")
            }
            _ => continue, // skip Ping/Binary/etc.
        }
    }
}

// ─── Test: lock conflict between two clients ─────────────────────────────────

#[tokio::test]
async fn test_ws_two_clients_lock_conflict() {
    let (addr, token_a, token_b) = start_server().await;
    let project_id = "fake-project-id"; // WS doesn't validate project existence for locking

    let ws_url_a = format!("ws://{}/api/projects/{}/ws?token={}", addr, project_id, token_a);
    let ws_url_b = format!("ws://{}/api/projects/{}/ws?token={}", addr, project_id, token_b);

    // Connect client A.
    let (ws_a, _) = connect_async(&ws_url_a).await.expect("Client A connect failed");
    let (mut tx_a, mut rx_a) = ws_a.split();

    // Connect client B.
    let (ws_b, _) = connect_async(&ws_url_b).await.expect("Client B connect failed");
    let (mut tx_b, mut rx_b) = ws_b.split();

    // Both clients receive the initial "locks" snapshot (empty).
    let snap_a = recv_json(&mut rx_a).await;
    assert_eq!(snap_a["type"], "locks");
    let snap_b = recv_json(&mut rx_b).await;
    assert_eq!(snap_b["type"], "locks");

    // Client A locks segment "seg-1".
    tx_a.send(TMsg::Text(
        json!({"type": "lock", "segment_id": "seg-1"}).to_string().into(),
    ))
    .await
    .unwrap();

    // Client A receives the broadcast.
    let lock_a = recv_json(&mut rx_a).await;
    assert_eq!(lock_a["type"], "segment:lock");
    assert_eq!(lock_a["segment_id"], "seg-1");

    // Client B also receives the broadcast.
    let lock_b = recv_json(&mut rx_b).await;
    assert_eq!(lock_b["type"], "segment:lock");
    assert_eq!(lock_b["segment_id"], "seg-1");

    // Client B tries to lock the same segment — should get an error (direct).
    tx_b.send(TMsg::Text(
        json!({"type": "lock", "segment_id": "seg-1"}).to_string().into(),
    ))
    .await
    .unwrap();

    let err_b = recv_json(&mut rx_b).await;
    assert_eq!(err_b["type"], "error");
    assert!(err_b["message"].as_str().unwrap().contains("locked"));

    // Client A unlocks — broadcast received by both.
    tx_a.send(TMsg::Text(
        json!({"type": "unlock", "segment_id": "seg-1"}).to_string().into(),
    ))
    .await
    .unwrap();

    let unlock_a = recv_json(&mut rx_a).await;
    assert_eq!(unlock_a["type"], "segment:unlock");
    assert_eq!(unlock_a["segment_id"], "seg-1");

    let unlock_b = recv_json(&mut rx_b).await;
    assert_eq!(unlock_b["type"], "segment:unlock");
    assert_eq!(unlock_b["segment_id"], "seg-1");
}

// ─── Test: auto-unlock after disconnect timeout ───────────────────────────────

#[tokio::test]
async fn test_ws_auto_unlock_on_disconnect() {
    let (addr, token_a, token_b) = start_server().await;
    let project_id = "fake-project-id-2";

    // Client A connects and locks a segment.
    let ws_url_a = format!("ws://{}/api/projects/{}/ws?token={}", addr, project_id, token_a);
    let (ws_a, _) = connect_async(&ws_url_a).await.expect("Client A failed");
    let (mut tx_a, mut rx_a) = ws_a.split();

    // Client B connects to receive broadcasts.
    let ws_url_b = format!("ws://{}/api/projects/{}/ws?token={}", addr, project_id, token_b);
    let (ws_b, _) = connect_async(&ws_url_b).await.expect("Client B failed");
    let (mut _tx_b, mut rx_b) = ws_b.split();

    // Consume initial lock snapshots.
    let _ = recv_json(&mut rx_a).await; // locks:[]
    let _ = recv_json(&mut rx_b).await; // locks:[]

    // Client A locks "seg-auto".
    tx_a.send(TMsg::Text(
        json!({"type": "lock", "segment_id": "seg-auto"}).to_string().into(),
    ))
    .await
    .unwrap();

    // Both receive segment:lock.
    let _ = recv_json(&mut rx_a).await;
    let _ = recv_json(&mut rx_b).await;

    // Client A disconnects (drop the sender, send Close).
    tx_a.send(TMsg::Close(None)).await.unwrap();
    drop(tx_a);
    drop(rx_a);

    // Wait slightly longer than ws_lock_timeout_secs (1s) for the grace-period unlock.
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

    // Client B should receive a segment:unlock broadcast.
    let unlock = recv_json(&mut rx_b).await;
    assert_eq!(unlock["type"], "segment:unlock");
    assert_eq!(unlock["segment_id"], "seg-auto");
}
