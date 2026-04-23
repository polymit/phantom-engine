use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("connection refused at {addr} — is the MCP server running?")]
    Connection { addr: String },

    #[error("HTTP {status}: {detail}")]
    Http { status: u16, detail: String },

    #[error("server returned error: {0}")]
    Rpc(String),

    #[error("failed to serialize request: {0}")]
    Serialization(String),

    #[error("{0}")]
    Setup(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}
