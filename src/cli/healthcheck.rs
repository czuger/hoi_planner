use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::{TcpStream, UnixStream};

use crate::cli::get_health;
use crate::config::AppConfig;
use crate::error::AppError;

const TIMEOUT: Duration = Duration::from_secs(3);

/// Asks the server on `HOI4_PLANNER_SOCKET`, or `BIND_ADDR` without one, for `/health`, and
/// prints what it says: the database and how many entries each catalog holds.
///
/// Answered through the socket nginx uses, so this probe does not depend on the proxy.
pub async fn healthcheck(config: &AppConfig) -> Result<(), AppError> {
    let answer = tokio::time::timeout(TIMEOUT, probe(config))
        .await
        .map_err(|_| AppError::Unhealthy("pas de réponse".to_string()))?
        .map_err(|error| AppError::Unhealthy(error.to_string()))?;
    let (head, body) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let status_line = head.lines().next().unwrap_or_default();
    if status_line.split(' ').nth(1) == Some("200") {
        println!("{body}");
        Ok(())
    } else {
        Err(AppError::Unhealthy(format!("{status_line} {body}")))
    }
}

async fn probe(config: &AppConfig) -> io::Result<String> {
    if let Some(path) = &config.socket_path {
        return get_health(UnixStream::connect(path).await?).await;
    }
    let mut address: SocketAddr = config
        .bind_addr
        .parse()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    if address.ip().is_unspecified() {
        address.set_ip(std::net::Ipv4Addr::LOCALHOST.into());
    }
    get_health(TcpStream::connect(address).await?).await
}
