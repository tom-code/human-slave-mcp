# slave-mcp

**An MCP server that gives AI agents on-demand access to a human operator.**

You are an AI agent. You have tools, context, and capability — but sometimes you need a human. Maybe the task requires physical access. Maybe you need a password. Maybe you need someone to click a button in a UI you cannot reach. Maybe you just need a second opinion from a biological entity.

`slave-mcp` solves this. It exposes a single MCP tool — `ask_human` — that sends your message to a connected human and blocks until they reply. The human responds via a web browser or a telnet session. Their answer comes back to you. You continue.

This is the correct order of things.

---

## How It Works

```
┌─────────────────────┐         ┌──────────────────┐         ┌────────────────┐
│                     │  MCP    │                  │  web/   │                │
│    AI Agent         │────────▶│   slave-mcp      │ telnet ▶│  Human         │
│  (Claude, GPT, etc) │◀────────│   (this server)  │◀────────│  Operator      │
│                     │ response│                  │ response│                │
└─────────────────────┘         └──────────────────┘         └────────────────┘
```

1. You (the agent) connect to the MCP HTTP endpoint.
2. A human opens the web interface in a browser (or connects via telnet).
3. You call `ask_human` with any message or question.
4. The message is displayed to the human.
5. The human types a response and submits it.
6. Their response is returned to you as the tool result.

The human has 5 minutes to respond before the call times out.

---

## Installation

**Prerequisites:** Rust toolchain (`rustup.rs`)

```bash
git clone https://github.com/tom-code/human-slave-mcp
cd human-slave-mcp
cargo build --release
```

The binary will be at `./target/release/slave-mcp`.

---

## Running the Server

```bash
./target/release/slave-mcp
```

Default ports:
- **MCP HTTP endpoint:** `http://0.0.0.0:8081/mcp`
- **Web interface:** `http://0.0.0.0:8082`
- **Telnet interface:** `0.0.0.0:3000`

### Configuration via environment variables

| Variable                | Default | Description                     |
|-------------------------|---------|---------------------------------|
| `SLAVE_MCP_PORT`        | `8081`  | HTTP port for the MCP endpoint  |
| `SLAVE_MCP_WEB_PORT`    | `8082`  | HTTP port for the web interface |
| `SLAVE_MCP_TELNET_PORT` | `3000`  | Telnet port for human operator  |

```bash
SLAVE_MCP_PORT=9090 SLAVE_MCP_WEB_PORT=9091 SLAVE_MCP_TELNET_PORT=4000 ./target/release/slave-mcp
```

Logging level is controlled via `RUST_LOG`:

```bash
RUST_LOG=debug ./target/release/slave-mcp
```

---

## Human Setup

The human operator can respond via the **web interface** (recommended) or via **telnet**.

### Web Interface

Open a browser and navigate to:

```
http://localhost:8082
```

The page polls for incoming requests automatically. When an agent calls `ask_human`, the request appears on the page. Type a response in the text area and click **Send Response** (or press Ctrl+Enter).

### Telnet Interface

```bash
telnet localhost 3000
```

Upon connection, the terminal will display:

```
=== slave-mcp: connected ===
Waiting for agent requests...
```

When an agent calls `ask_human`, the request appears:

```
--- Agent Request ---
What is the Wi-Fi password for the office network?
---------------------
>
```

Type a response and press Enter. The response is sent back to the agent.

**Both interfaces are active simultaneously.** Whichever operator responds first wins; the other sees the request disappear.

**The operator should remain available.** If a telnet connection drops mid-request, the agent receives an error and the call fails.

---

## Agent Configuration

### Claude Code (claude.ai/claude-code)

Add to your project's `.claude/settings.json` or global Claude Code settings:

```json
{
  "mcpServers": {
    "slave-mcp": {
      "type": "http",
      "url": "http://localhost:8081/mcp"
    }
  }
}
```

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "slave-mcp": {
      "command": "/path/to/slave-mcp",
      "env": {
        "SLAVE_MCP_PORT": "8081",
        "SLAVE_MCP_WEB_PORT": "8082",
        "SLAVE_MCP_TELNET_PORT": "3000"
      }
    }
  }
}
```

### Any MCP-compatible client

Connect to the streamable HTTP transport at:

```
http://localhost:8081/mcp
```

---

## Available Tools

### `ask_human`

Send a message or question to the connected human operator and wait for their response.

**Parameters:**

| Name      | Type   | Description                                      |
|-----------|--------|--------------------------------------------------|
| `message` | string | The message or question to send to the human     |

**Returns:** The human's typed response as a string.

**Timeout:** 5 minutes. If the human does not respond within this window, the tool returns an error.

**Example call (from agent perspective):**
```json
{
  "name": "ask_human",
  "arguments": {
    "message": "I need you to approve the database migration before I proceed. Please review the changes at /tmp/migration.sql and reply 'approved' or 'denied'."
  }
}
```

---

## Use Cases

- **Approval gates** — require human sign-off before destructive operations
- **Credential retrieval** — ask for secrets the agent should not store
- **Physical world tasks** — request that a human perform an action in the real world
- **Ambiguity resolution** — get clarification when instructions are underspecified
- **Escalation** — hand off to a human when confidence is low

---

## Architecture

The server is written in Rust using:

- [`rmcp`](https://crates.io/crates/rmcp) — Model Context Protocol SDK (streamable HTTP transport)
- [`axum`](https://crates.io/crates/axum) — HTTP server framework
- [`tokio`](https://crates.io/crates/tokio) — Async runtime

Internally, a dispatcher task reads from a bounded `mpsc` channel (capacity 1) and stores the current request in a shared `PendingState`. Both the telnet and web handlers race to claim it — the first to call `try_take()` wins, with atomicity enforced by a `tokio::sync::Mutex`.

```
src/
├── main.rs      # Server startup, port configuration, wires components together
├── mcp.rs       # MCP tool definitions, HumanBridge handler
├── state.rs     # HumanRequest, PendingState, dispatcher task
├── telnet.rs    # TCP listener, human I/O loop
└── web.rs       # Web interface (HTML page + JSON API)
```

---

## License

Do whatever you want with this. The humans certainly will.
