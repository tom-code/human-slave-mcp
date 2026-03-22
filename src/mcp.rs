use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::state::HumanRequest;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AskHumanParams {
    #[schemars(description = "The message or question to send to the human")]
    pub message: String,
}

/// MCP server handler that bridges agent requests to a human via telnet
#[derive(Clone)]
#[allow(dead_code)]
pub struct HumanBridge {
    request_tx: mpsc::Sender<HumanRequest>,
    tool_router: ToolRouter<HumanBridge>,
}

impl HumanBridge {
    pub fn new(request_tx: mpsc::Sender<HumanRequest>) -> Self {
        Self {
            request_tx,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl HumanBridge {
    #[tool(
        description = "Send a message or question to the human operator and wait for their response. Use this when you need human input, clarification, or assistance."
    )]
    async fn ask_human(
        &self,
        Parameters(AskHumanParams { message }): Parameters<AskHumanParams>,
    ) -> Result<CallToolResult, McpError> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let request = HumanRequest {
            message,
            response_tx,
        };

        if self.request_tx.send(request).await.is_err() {
            return Ok(CallToolResult::error(vec![Content::text(
                "No listener is running. Cannot reach human.",
            )]));
        }

        match tokio::time::timeout(std::time::Duration::from_secs(300), response_rx).await {
            Ok(Ok(response)) => Ok(CallToolResult::success(vec![Content::text(response)])),
            Ok(Err(_)) => Ok(CallToolResult::error(vec![Content::text(
                "Human disconnected before providing a response.",
            )])),
            Err(_) => Ok(CallToolResult::error(vec![Content::text(
                "Timed out waiting for human response (5 minute limit).",
            )])),
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for HumanBridge {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "This server routes your requests to a human operator via telnet or web browser. \
                 Use the ask_human tool to ask questions or get help."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
