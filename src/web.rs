use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    routing::{get, post},
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
</style>
</head>
<body>
<h1>slave-mcp &mdash; Human Interface</h1>
<div id="status">Waiting for agent requests...</div>
<div id="question-box">
  <div id="question-label">Agent request</div>
  <div id="question"></div>
</div>
<textarea id="response" placeholder="Type your response here..."></textarea>
<button id="submit-btn" onclick="submitResponse()">Send Response</button>
<div id="feedback"></div>

<script>
let polling = true;
let hasPending = false;

async function poll() {
  if (!polling) return;
  try {
    const res = await fetch('/api/pending');
    const data = await res.json();
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
    }
  } catch (e) {
    // ignore transient errors
  }
  setTimeout(poll, 2000);
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
    } else {
      resetUI('');
      setFeedback('Request already answered by another interface.', 'err');
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
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"));
    (headers, HTML)
}

async fn get_pending(State(pending): State<Arc<PendingState>>) -> Json<PendingResponse> {
    match pending.peek_message().await {
        Some(message) => Json(PendingResponse { pending: true, message: Some(message) }),
        None => Json(PendingResponse { pending: false, message: None }),
    }
}

async fn submit_response(
    State(pending): State<Arc<PendingState>>,
    Json(body): Json<RespondBody>,
) -> (StatusCode, Json<RespondResponse>) {
    match pending.try_take().await {
        Some(req) => {
            let _ = req.response_tx.send(body.response);
            (StatusCode::OK, Json(RespondResponse { ok: true, error: None }))
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

pub async fn run_web_server(port: u16, pending: Arc<PendingState>) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/", get(index_page))
        .route("/api/pending", get(get_pending))
        .route("/api/respond", post(submit_response))
        .with_state(pending);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    tracing::info!("Web interface on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
