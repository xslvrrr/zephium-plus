//! rice fork: relays an MCP client's stdio to the running Zephium. It never
//! launches the browser and carries nothing but the bytes.
use std::path::PathBuf;

fn socket() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("ZEPHIUM_MCP_SOCKET") {
        return Some(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join("Library/Application Support/app.zephium/mcp/mcp.sock"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let Some(path) = socket() else {
        eprintln!("zephium-mcp: HOME is not set.");
        return std::process::ExitCode::FAILURE;
    };
    let Ok(mut stream) = tokio::net::UnixStream::connect(&path).await else {
        eprintln!("Zephium is not running, or Claude access is off in Settings → Connections.");
        return std::process::ExitCode::FAILURE;
    };
    let mut stdio = tokio::io::join(tokio::io::stdin(), tokio::io::stdout());
    match tokio::io::copy_bidirectional(&mut stdio, &mut stream).await {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}
