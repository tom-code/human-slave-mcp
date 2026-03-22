use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::state::PendingState;

const HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>slave-mcp: Human Interface</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: monospace; background: #1a1a1a; color: #e0e0e0; padding: 2rem; max-width: 800px; margin: 0 auto; }
  h1 { font-size: 1.2rem; color: #888; margin-bottom: 1.5rem; border-bottom: 1px solid #333; padding-bottom: 0.5rem; }
  #status { font-size: 0.85rem; color: #666; margin-bottom: 1rem; }
  #status.active { color: #4caf50; }
  #question-box { display: none; background: #2a2a2a; border: 1px solid #444; border-radius: 4px; padding: 1rem; margin-bottom: 1rem; }
  #question-label { font-size: 0.75rem; color: #888; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem; }
  #question { white-space: pre-wrap; word-break: break-word; line-height: 1.5; }
  textarea { width: 100%; height: 120px; background: #2a2a2a; border: 1px solid #555; border-radius: 4px; color: #e0e0e0; font-family: monospace; font-size: 0.95rem; padding: 0.75rem; resize: vertical; outline: none; display: none; }
  textarea:focus { border-color: #888; }
  button { margin-top: 0.75rem; padding: 0.5rem 1.5rem; background: #3a3a3a; border: 1px solid #666; border-radius: 4px; color: #e0e0e0; font-family: monospace; cursor: pointer; display: none; }
  button:hover { background: #4a4a4a; }
  button:disabled { opacity: 0.4; cursor: default; }
  #feedback { margin-top: 0.75rem; font-size: 0.85rem; min-height: 1.2em; }
  #feedback.ok { color: #4caf50; }
  #feedback.err { color: #f44336; }
  #server-status { font-size: 0.8rem; margin-bottom: 1rem; display: flex; align-items: center; gap: 0.4rem; }
  .dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block; }
  .dot.online { background: #4caf50; }
  .dot.offline { background: #f44336; }
  #history-section { margin-top: 2rem; border-top: 1px solid #333; padding-top: 1rem; }
  #history-heading { font-size: 0.75rem; color: #666; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 1rem; }
  #history-list { display: flex; flex-direction: column; gap: 1rem; }
  .history-entry { background: #222; border: 1px solid #333; border-radius: 4px; overflow: hidden; }
  .history-q { padding: 0.6rem 0.75rem; border-bottom: 1px solid #333; }
  .history-a { padding: 0.6rem 0.75rem; background: #1e2e1e; }
  .history-q-label { font-size: 0.7rem; color: #666; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.3rem; display: flex; justify-content: space-between; }
  .history-a-label { font-size: 0.7rem; color: #4a7a4a; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.3rem; }
  .history-time { font-size: 0.7rem; color: #555; text-transform: none; letter-spacing: 0; }
  .history-q-text { white-space: pre-wrap; word-break: break-word; line-height: 1.4; color: #ccc; font-size: 0.9rem; }
  .history-a-text { white-space: pre-wrap; word-break: break-word; line-height: 1.4; color: #8bc98b; font-size: 0.9rem; }
  #history-empty { font-size: 0.85rem; color: #444; }
</style>
</head>
<body>
<h1>slave-mcp &mdash; Human Interface</h1>
<div id="server-status"><span id="server-dot" class="dot online"></span><span id="server-label">Server Online</span></div>
<div id="status">Waiting for agent requests...</div>
<div id="question-box">
  <div id="question-label">Agent request</div>
  <div id="question"></div>
</div>
<textarea id="response" placeholder="Type your response here..."></textarea>
<button id="submit-btn" onclick="submitResponse()">Send Response</button>
<div id="feedback"></div>

<div id="history-section">
  <div id="history-heading">History</div>
  <div id="history-list"><div id="history-empty">No exchanges yet.</div></div>
</div>

<script>
let polling = true;
let hasPending = false;
let serverOnline = true;
let historyCount = 0;

function setServerStatus(online) {
  if (online === serverOnline) return;
  serverOnline = online;
  document.getElementById('server-dot').className = 'dot ' + (online ? 'online' : 'offline');
  document.getElementById('server-label').textContent = online ? 'Server Online' : 'Server Offline';
  if (!online) resetUI('Server offline.');
  else if (document.getElementById('status').textContent === 'Server offline.') resetUI('');
}

async function poll() {
  if (!polling) return;
  try {
    const res = await fetch('/api/pending');
    const data = await res.json();
    setServerStatus(true);
    if (data.pending && !hasPending) {
      hasPending = true;
      document.getElementById('status').textContent = 'Request pending — please respond:';
      document.getElementById('status').className = 'active';
      document.getElementById('question').textContent = data.message;
      document.getElementById('question-box').style.display = 'block';
      document.getElementById('response').style.display = 'block';
      document.getElementById('submit-btn').style.display = 'inline-block';
      document.getElementById('response').value = '';
      document.getElementById('response').focus();
      document.getElementById('feedback').textContent = '';
    } else if (!data.pending && hasPending) {
      resetUI('Request was handled elsewhere.');
      refreshHistory();
    }
  } catch (e) {
    setServerStatus(false);
  }
  setTimeout(poll, 2000);
}

async function refreshHistory() {
  try {
    const res = await fetch('/api/history');
    const entries = await res.json();
    if (entries.length === historyCount) return;
    historyCount = entries.length;
    const list = document.getElementById('history-list');
    if (entries.length === 0) {
      list.innerHTML = '<div id="history-empty">No exchanges yet.</div>';
      return;
    }
    list.innerHTML = '';
    // Show most recent at top
    for (let i = entries.length - 1; i >= 0; i--) {
      const e = entries[i];
      const timeStr = e.timestamp ? new Date(e.timestamp * 1000).toLocaleTimeString([], {hour: '2-digit', minute: '2-digit', second: '2-digit'}) : '';
      const div = document.createElement('div');
      div.className = 'history-entry';
      div.innerHTML =
        '<div class="history-q"><div class="history-q-label"><span>Agent</span><span class="history-time"></span></div><div class="history-q-text"></div></div>' +
        '<div class="history-a"><div class="history-a-label">You</div><div class="history-a-text"></div></div>';
      div.querySelector('.history-q-text').textContent = e.question;
      div.querySelector('.history-a-text').textContent = e.answer;
      div.querySelector('.history-time').textContent = timeStr;
      list.appendChild(div);
    }
  } catch (_) {}
}

async function submitResponse() {
  const response = document.getElementById('response').value.trim();
  if (!response) return;
  document.getElementById('submit-btn').disabled = true;
  try {
    const res = await fetch('/api/respond', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ response }),
    });
    const data = await res.json();
    if (data.ok) {
      resetUI('');
      setFeedback('Response sent.', 'ok');
      refreshHistory();
    } else {
      resetUI('');
      setFeedback('Request already answered by another interface.', 'err');
      refreshHistory();
    }
  } catch (e) {
    setFeedback('Error sending response: ' + e.message, 'err');
    document.getElementById('submit-btn').disabled = false;
  }
}

function resetUI(msg) {
  hasPending = false;
  document.getElementById('status').textContent = msg || 'Waiting for agent requests...';
  document.getElementById('status').className = '';
  document.getElementById('question-box').style.display = 'none';
  document.getElementById('response').style.display = 'none';
  document.getElementById('submit-btn').style.display = 'none';
  document.getElementById('submit-btn').disabled = false;
}

function setFeedback(msg, cls) {
  const el = document.getElementById('feedback');
  el.textContent = msg;
  el.className = cls;
}

document.getElementById('response').addEventListener('keydown', function(e) {
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) submitResponse();
});

poll();
refreshHistory();
</script>
</body>
</html>
"#;

#[derive(Serialize)]
struct PendingResponse {
    pending: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Deserialize)]
pub struct RespondBody {
    response: String,
}

#[derive(Serialize)]
struct RespondResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn index_page() -> (HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (headers, HTML)
}

async fn get_pending(State(pending): State<Arc<PendingState>>) -> Json<PendingResponse> {
    match pending.peek_message().await {
        Some(message) => Json(PendingResponse {
            pending: true,
            message: Some(message),
        }),
        None => Json(PendingResponse {
            pending: false,
            message: None,
        }),
    }
}

async fn submit_response(
    State(pending): State<Arc<PendingState>>,
    Json(body): Json<RespondBody>,
) -> (StatusCode, Json<RespondResponse>) {
    match pending.try_take().await {
        Some(req) => {
            pending
                .push_history(req.message.clone(), body.response.clone())
                .await;
            let _ = req.response_tx.send(body.response);
            (
                StatusCode::OK,
                Json(RespondResponse {
                    ok: true,
                    error: None,
                }),
            )
        }
        None => (
            StatusCode::OK,
            Json(RespondResponse {
                ok: false,
                error: Some("No pending request or already answered".to_string()),
            }),
        ),
    }
}

async fn get_history(State(pending): State<Arc<PendingState>>) -> Json<Vec<crate::state::HistoryEntry>> {
    Json(pending.get_history().await)
}

pub async fn run_web_server(port: u16, pending: Arc<PendingState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index_page))
        .route("/api/pending", get(get_pending))
        .route("/api/respond", post(submit_response))
        .route("/api/history", get(get_history))
        .with_state(pending);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Web interface on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
