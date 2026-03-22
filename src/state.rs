use std::sync::Arc;

use tokio::sync::{mpsc, oneshot, Mutex, Notify};

/// A request from the agent to the human
pub struct HumanRequest {
    pub message: String,
    pub response_tx: oneshot::Sender<String>,
}

/// Shared state holding the current pending request, accessible by both telnet and web handlers
pub struct PendingState {
    inner: Mutex<Option<HumanRequest>>,
    new_request: Notify,
    slot_cleared: Notify,
}

impl PendingState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            new_request: Notify::new(),
            slot_cleared: Notify::new(),
        }
    }

    /// Store a new request. Caller must ensure the slot is empty first.
    async fn store(&self, request: HumanRequest) {
        *self.inner.lock().await = Some(request);
        self.new_request.notify_waiters();
    }

    /// Try to atomically take the pending request. Returns None if already taken.
    pub async fn try_take(&self) -> Option<HumanRequest> {
        let req = self.inner.lock().await.take();
        if req.is_some() {
            self.slot_cleared.notify_waiters();
        }
        req
    }

    /// Peek at the pending message without taking it.
    pub async fn peek_message(&self) -> Option<String> {
        self.inner.lock().await.as_ref().map(|r| r.message.clone())
    }

    /// Wait until a request is available and take it atomically.
    pub async fn wait_and_take(&self) -> HumanRequest {
        loop {
            {
                if let Some(req) = self.inner.lock().await.take() {
                    self.slot_cleared.notify_waiters();
                    return req;
                }
            }
            self.new_request.notified().await;
        }
    }
}

/// Dispatcher: reads from the mpsc receiver and stores requests into PendingState.
/// Waits for the current slot to be cleared before accepting the next request,
/// preserving backpressure.
pub async fn dispatch_requests(
    mut rx: mpsc::Receiver<HumanRequest>,
    pending: Arc<PendingState>,
) {
    while let Some(request) = rx.recv().await {
        // Wait until the slot is empty
        loop {
            if pending.peek_message().await.is_none() {
                pending.store(request).await;
                break;
            }
            pending.slot_cleared.notified().await;
        }
    }
}
