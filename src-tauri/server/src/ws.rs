use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex};
use tokio::time::sleep;

use crate::{
    app::AppState,
    auth::jwt::{decode_token, TokenKind},
    db::run_db,
    error::AppError,
};

// ─── Shared State ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SegmentLock {
    pub user_id: String,
    pub username: String,
}

#[derive(Clone)]
pub struct WsState {
    /// Global lock map: segment_id → SegmentLock
    pub locks: Arc<Mutex<HashMap<String, SegmentLock>>>,
    /// Per-project broadcast: project_id → Sender<json-string>
    pub channels: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    /// Seconds to hold a lock after a client disconnects (grace period)
    pub lock_timeout_secs: u64,
}

impl WsState {
    pub fn new(lock_timeout_secs: u64) -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
            channels: Arc::new(Mutex::new(HashMap::new())),
            lock_timeout_secs,
        }
    }
}

// ─── Message Types ───────────────────────────────────────────────────────────

/// Messages from the browser to the server.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Lock {
        segment_id: String,
    },
    Unlock {
        segment_id: String,
    },
    Update {
        segment_id: String,
        target: String,
        status: String,
    },
}

/// Messages from the server to the browser (broadcast or direct).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMsg {
    /// Broadcast: someone locked a segment
    #[serde(rename = "segment:lock")]
    SegmentLocked {
        segment_id: String,
        user_id: String,
        username: String,
    },

    /// Broadcast: a segment was unlocked
    #[serde(rename = "segment:unlock")]
    SegmentUnlocked { segment_id: String },

    /// Broadcast: a translation was saved
    #[serde(rename = "segment:update")]
    SegmentUpdated {
        segment_id: String,
        target: String,
        status: String,
        user_id: String,
    },

    /// Direct: segment already locked by someone else (error)
    #[serde(rename = "error")]
    Error { message: String },

    /// Direct on connect: snapshot of all current locks
    #[serde(rename = "locks")]
    CurrentLocks { locks: Vec<LockInfo> },
}

#[derive(Debug, Clone, Serialize)]
pub struct LockInfo {
    pub segment_id: String,
    pub user_id: String,
    pub username: String,
}

/// Query string for WebSocket upgrade: `?token=<access_token>`
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: String,
}

// ─── Route Handler ───────────────────────────────────────────────────────────

/// GET /api/projects/:projectId/ws
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<WsQuery>,
) -> axum::response::Response {
    let claims = match decode_token(&query.token, &state.config.jwt_secret) {
        Ok(c) if c.kind == TokenKind::Access => c,
        _ => {
            return (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_ws(socket, state, project_id, claims.sub, claims.username))
        .into_response()
}

// ─── Connection Handler ──────────────────────────────────────────────────────

async fn handle_ws(
    mut socket: WebSocket,
    state: AppState,
    project_id: String,
    user_id: String,
    username: String,
) {
    // Get (or lazily create) the broadcast channel for this project.
    let tx = {
        let mut channels = state.ws.channels.lock().await;
        channels
            .entry(project_id.clone())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel::<String>(128);
                tx
            })
            .clone()
    };
    let mut rx = tx.subscribe();

    // Push the current lock snapshot so the client knows what is already locked.
    {
        let locks = state.ws.locks.lock().await;
        let current: Vec<LockInfo> = locks
            .iter()
            .map(|(seg_id, lock)| LockInfo {
                segment_id: seg_id.clone(),
                user_id: lock.user_id.clone(),
                username: lock.username.clone(),
            })
            .collect();
        if let Ok(json) = serde_json::to_string(&ServerMsg::CurrentLocks { locks: current }) {
            let _ = socket.send(Message::Text(json.into())).await;
        }
    }

    // Track which segments this connection holds locks on (for cleanup).
    let mut my_segments: Vec<String> = Vec::new();

    // ── Main event loop ──────────────────────────────────────────────────────
    loop {
        tokio::select! {
            // ── Incoming message from the WebSocket client ───────────────
            msg_opt = socket.recv() => {
                match msg_opt {
                    None | Some(Err(_)) => break,
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Text(text))) => {
                        let text_str = text.to_string();
                        let client_msg: ClientMsg = match serde_json::from_str(&text_str) {
                            Ok(m) => m,
                            Err(_) => continue,
                        };

                        match client_msg {
                            // ── Lock ─────────────────────────────────────
                            ClientMsg::Lock { segment_id } => {
                                let outcome = {
                                    let mut locks = state.ws.locks.lock().await;
                                    if let Some(existing) = locks.get(&segment_id) {
                                        if existing.user_id != user_id {
                                            Err(format!(
                                                "Segment is locked by {}",
                                                existing.username
                                            ))
                                        } else {
                                            Ok(false) // already locked by this user
                                        }
                                    } else {
                                        locks.insert(
                                            segment_id.clone(),
                                            SegmentLock {
                                                user_id: user_id.clone(),
                                                username: username.clone(),
                                            },
                                        );
                                        Ok(true) // newly locked
                                    }
                                };

                                match outcome {
                                    Err(msg) => {
                                        let err = serde_json::to_string(&ServerMsg::Error {
                                            message: msg,
                                        })
                                        .unwrap_or_default();
                                        let _ = socket
                                            .send(Message::Text(err.into()))
                                            .await;
                                    }
                                    Ok(true) => {
                                        my_segments.push(segment_id.clone());
                                        let bcast = serde_json::to_string(
                                            &ServerMsg::SegmentLocked {
                                                segment_id,
                                                user_id: user_id.clone(),
                                                username: username.clone(),
                                            },
                                        )
                                        .unwrap_or_default();
                                        let _ = tx.send(bcast);
                                    }
                                    Ok(false) => {}
                                }
                            }

                            // ── Unlock ───────────────────────────────────
                            ClientMsg::Unlock { segment_id } => {
                                let did_unlock = {
                                    let mut locks = state.ws.locks.lock().await;
                                    if locks
                                        .get(&segment_id)
                                        .map(|l| &l.user_id)
                                        == Some(&user_id)
                                    {
                                        locks.remove(&segment_id);
                                        true
                                    } else {
                                        false
                                    }
                                };
                                if did_unlock {
                                    my_segments.retain(|s| s != &segment_id);
                                    let bcast = serde_json::to_string(
                                        &ServerMsg::SegmentUnlocked { segment_id },
                                    )
                                    .unwrap_or_default();
                                    let _ = tx.send(bcast);
                                }
                            }

                            // ── Update (save translation) ─────────────────
                            ClientMsg::Update { segment_id, target, status } => {
                                let pool = state.pool.clone();
                                let now = Utc::now().to_rfc3339();
                                let seg_id = segment_id.clone();
                                let tgt = target.clone();
                                let st = status.clone();
                                let pid = project_id.clone();
                                let uid = user_id.clone();

                                let db_ok = run_db(pool, move |conn| {
                                    let rows_changed = conn
                                        .execute(
                                            "UPDATE segments
                                             SET target = ?1, status = ?2, updated_at = ?3
                                             WHERE id = ?4
                                               AND file_id IN (
                                                   SELECT id FROM project_files
                                                   WHERE project_id IN (
                                                       SELECT id FROM projects
                                                       WHERE id = ?5 AND owner_id = ?6
                                                   )
                                               )",
                                            params![&tgt, &st, &now, &seg_id, &pid, &uid],
                                        )
                                        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
                                    Ok(rows_changed)
                                })
                                .await
                                .map(|n| n > 0)
                                .unwrap_or(false);

                                if db_ok {
                                    let bcast = serde_json::to_string(
                                        &ServerMsg::SegmentUpdated {
                                            segment_id,
                                            target,
                                            status,
                                            user_id: user_id.clone(),
                                        },
                                    )
                                    .unwrap_or_default();
                                    let _ = tx.send(bcast);
                                }
                            }
                        }
                    }
                    Some(Ok(_)) => {} // Ping / Pong / Binary — ignore
                }
            }

            // ── Relay broadcast message to this client ────────────────────
            result = rx.recv() => {
                match result {
                    Ok(json) => {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Some messages were skipped due to buffer overflow; continue.
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    // ── On disconnect: release this connection's locks after the grace period ─
    if !my_segments.is_empty() {
        let timeout = state.ws.lock_timeout_secs;
        let locks_arc = state.ws.locks.clone();
        let tx_cleanup = tx.clone();
        let uid = user_id.clone();

        tokio::spawn(async move {
            if timeout > 0 {
                sleep(Duration::from_secs(timeout)).await;
            }
            let mut lock_map = locks_arc.lock().await;
            for seg_id in my_segments {
                if lock_map.get(&seg_id).map(|l| &l.user_id) == Some(&uid) {
                    lock_map.remove(&seg_id);
                    let msg =
                        serde_json::to_string(&ServerMsg::SegmentUnlocked { segment_id: seg_id })
                            .unwrap_or_default();
                    let _ = tx_cleanup.send(msg);
                }
            }
        });
    }
}
