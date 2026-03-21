use tokio::sync::oneshot;

/// A request from the agent to the human
pub struct HumanRequest {
    pub message: String,
    pub response_tx: oneshot::Sender<String>,
}
