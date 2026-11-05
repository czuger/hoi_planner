//! Where the server listens, and the probe `healthcheck` sends there.
//!
//! On the server the app listens on a Unix socket: nginx runs in a container and reaches it
//! through a host directory bind-mounted into it, so no port is opened at all. Without
//! `SOCKET_PATH` (`cargo run`), it listens on TCP at `BIND_ADDR`.

use std::fs::{self, Permissions};
use std::future::Future;
use std::io;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;

use axum::Router;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, UnixListener};

use crate::config::AppConfig;
use crate::error::AppError;

/// nginx's worker runs as another user: it must be able to connect. Who may reach the
/// directory is the directory's business.
const SOCKET_MODE: u32 = 0o666;

/// Serves on the socket when there is one, on TCP otherwise, until SIGTERM or Ctrl-C.
pub async fn listen(router: Router, config: &AppConfig) -> Result<(), AppError> {
    let (address, served) = match &config.socket_path {
        Some(path) => (
            path.display().to_string(),
            serve_unix(router, path, shutdown_signal()).await,
        ),
        None => (
            config.bind_addr.clone(),
            serve_tcp(router, &config.bind_addr).await,
        ),
    };
    served.map_err(|source| AppError::Listen { address, source })
}

async fn serve_tcp(router: Router, address: &str) -> io::Result<()> {
    let listener = TcpListener::bind(address).await?;
    tracing::info!("listening on http://{address}");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Serves on a Unix socket until `shutdown` resolves, then removes the socket.
///
/// # Errors
/// Fails when the path is taken by something that is not a socket, or the socket cannot be set up.
pub async fn serve_unix(
    router: Router,
    path: &Path,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> io::Result<()> {
    remove_stale_socket(path)?;
    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, Permissions::from_mode(SOCKET_MODE))?;
    tracing::info!("listening on unix:{}", path.display());
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await?;
    fs::remove_file(path)
}

/// A socket left by a crash would make `bind` fail; anything else there is not ours to delete.
fn remove_stale_socket(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => fs::remove_file(path),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} existe et n'est pas une socket", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "cannot listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

/// A bare `GET /health` over `stream`; returns the whole answer, status line first.
///
/// The server needs no curl, and this needs no extra crate.
///
/// # Errors
/// Fails when reading or writing `stream` does.
pub async fn get_health(mut stream: impl AsyncRead + AsyncWrite + Unpin) -> io::Result<String> {
    stream
        .write_all(b"GET /health HTTP/1.0\r\nHost: localhost\r\n\r\n")
        .await?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await?;
    Ok(String::from_utf8_lossy(&response).into_owned())
}

#[cfg(test)]
mod tests {
    use axum::routing::get;
    use tokio::net::UnixStream;

    use super::*;

    #[test]
    fn a_stale_socket_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stale.sock");
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        remove_stale_socket(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn a_regular_file_at_the_socket_path_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not_a_socket");
        fs::write(&path, "keep me").unwrap();
        assert_eq!(
            remove_stale_socket(&path).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "keep me");
    }

    #[tokio::test]
    async fn serves_health_on_a_socket_and_removes_it_on_shutdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.sock");
        let router = Router::new().route("/health", get(|| async { "ok" }));
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn({
            let path = path.clone();
            async move {
                serve_unix(router, &path, async {
                    stopped.await.ok();
                })
                .await
            }
        });

        let mut stream = None;
        for _ in 0..100 {
            if let Ok(connected) = UnixStream::connect(&path).await {
                stream = Some(connected);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let answer = get_health(stream.unwrap()).await.unwrap();
        assert_eq!(answer.split(' ').nth(1), Some("200"), "{answer}");
        assert!(answer.ends_with("ok"), "{answer}");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            SOCKET_MODE
        );

        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        assert!(!path.exists());
    }
}
