//! The Unix socket: a same-uid listener and the server's lifecycle.
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rmcp::ServiceExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;

use crate::handler::{client_label, Handler};
use crate::Backend;

/// The only directory name `bind` will create and lock down to `0700`.
const SOCKET_DIR: &str = "mcp";

pub struct McpServer {
    stop: watch::Sender<bool>,
    socket: PathBuf,
}

fn bind(socket: &Path) -> io::Result<UnixListener> {
    let dir = socket
        .parent()
        .filter(|dir| dir.file_name().is_some_and(|name| name == SOCKET_DIR))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "the MCP socket must sit in its own directory named `mcp`",
            )
        })?;
    std::fs::create_dir_all(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    match UnixListener::bind(socket) {
        Ok(listener) => Ok(listener),
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
            // A socket left by a crash: no one answers on it, so it is stale.
            // Anything that is not a socket is left alone.
            let is_socket = std::fs::symlink_metadata(socket)?.file_type().is_socket();
            if !is_socket || std::os::unix::net::UnixStream::connect(socket).is_ok() {
                return Err(error);
            }
            std::fs::remove_file(socket)?;
            UnixListener::bind(socket)
        }
        Err(error) => Err(error),
    }
}

fn update(clients: &Mutex<Vec<String>>, change: impl FnOnce(&mut Vec<String>)) -> Vec<String> {
    let mut list = clients
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    change(&mut list);
    list.clone()
}

/// Serves one same-uid connection until the client leaves or the server stops.
async fn serve(
    stream: UnixStream,
    backend: Arc<dyn Backend>,
    clients: Arc<Mutex<Vec<String>>>,
    mut stopped: watch::Receiver<bool>,
) {
    let (read, write) = stream.into_split();
    let service = tokio::select! {
        _ = stopped.changed() => return,
        served = Handler::new(backend.clone()).serve((read, write)) => match served {
            Ok(service) => service,
            Err(_) => return,
        },
    };
    let name = client_label(service.peer_info().as_ref().map(|info| &info.client_info));
    backend.connected(&update(&clients, |list| list.push(name.clone())));
    // On stop, cancel the service and still wait for its transport to close.
    let cancel = service.cancellation_token();
    let watcher = tokio::spawn(async move {
        let _ = stopped.changed().await;
        cancel.cancel();
    });
    let _ = service.waiting().await;
    watcher.abort();
    backend.connected(&update(&clients, |list| {
        if let Some(at) = list.iter().position(|n| *n == name) {
            list.remove(at);
        }
    }));
}

impl McpServer {
    /// Listens on `socket`, a path inside a directory named exactly `mcp`
    /// (`<app data dir>/mcp/mcp.sock`). That directory is created if needed
    /// and set to `0700`; any other parent is refused with an
    /// `InvalidInput` error, so the app data dir itself is never chmodded.
    /// The socket is `0600`, a stale socket from a crash is replaced once,
    /// and a peer with another uid is closed before any byte is read.
    pub async fn start(socket: PathBuf, backend: Arc<dyn Backend>) -> io::Result<McpServer> {
        let listener = bind(&socket)?;
        if let Err(error) =
            std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
        {
            let _ = std::fs::remove_file(&socket);
            return Err(error);
        }
        let (stop, mut stopped) = watch::channel(false);
        let clients: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        // SAFETY: getuid has no preconditions and cannot fail.
        let me = unsafe { libc::getuid() };
        tokio::spawn(async move {
            loop {
                let stream = tokio::select! {
                    _ = stopped.changed() => break,
                    accepted = listener.accept() => match accepted {
                        Ok((stream, _)) => stream,
                        // A persistent error (out of descriptors) must not spin.
                        Err(_) => {
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            continue;
                        }
                    },
                };
                if !matches!(stream.peer_cred(), Ok(cred) if cred.uid() == me) {
                    drop(stream);
                    continue;
                }
                tokio::spawn(serve(
                    stream,
                    backend.clone(),
                    clients.clone(),
                    stopped.clone(),
                ));
            }
        });
        Ok(McpServer { stop, socket })
    }

    /// Drops every client and removes the socket.
    pub fn stop(self) {
        let _ = self.stop.send(true);
        let _ = std::fs::remove_file(&self.socket);
    }
}
