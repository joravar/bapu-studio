//! Pure-Rust async SSH bastion/jump-host TCP port forwarding via `russh`.
//! Opens a local `127.0.0.1:<ephemeral_port>` TCP listener and forwards every accepted
//! connection through `direct-tcpip` over the SSH session to the target database host:port.

use crate::types::DatabaseConnection;
use anyhow::{bail, Context, Result};
use russh::keys::key::PrivateKeyWithHashAlg;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

struct ClientHandler;

#[async_trait::async_trait]
impl russh::client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Dropping this guard stops the local TCP forwarding loop and closes the SSH session.
pub struct SshTunnelGuard {
    pub local_port: u16,
    accept_task: JoinHandle<()>,
}

impl Drop for SshTunnelGuard {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

/// Returns `(target_host, target_port, optional_tunnel_guard)`.
/// If SSH tunneling is disabled (or a raw connection string is used), returns the configured
/// host/port directly with `None`.
pub async fn effective_target(
    config: &DatabaseConnection,
    default_port: u16,
) -> Result<(String, u16, Option<SshTunnelGuard>)> {
    let dst_host = config.host.as_deref().unwrap_or("localhost").to_string();
    let dst_port: u16 = config
        .port
        .as_deref()
        .and_then(|p| p.parse().ok())
        .unwrap_or(default_port);

    let has_conn_str = config
        .connection_string
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());

    if !config.ssh_enabled || has_conn_str {
        return Ok((dst_host, dst_port, None));
    }

    let ssh_host = config
        .ssh_host
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("SSH tunnel enabled, but SSH host is empty")?;
    let ssh_port: u16 = config
        .ssh_port
        .as_deref()
        .and_then(|p| p.parse().ok())
        .unwrap_or(22);
    let ssh_user = config
        .ssh_username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .context("SSH tunnel enabled, but SSH username is empty")?;

    let guard = open_ssh_tunnel(
        ssh_host,
        ssh_port,
        ssh_user,
        config.ssh_password.as_deref(),
        config.ssh_private_key.as_deref(),
        config.ssh_passphrase.as_deref(),
        dst_host,
        dst_port,
    )
    .await?;

    let port = guard.local_port;
    Ok(("127.0.0.1".to_string(), port, Some(guard)))
}

#[allow(clippy::too_many_arguments)]
async fn open_ssh_tunnel(
    ssh_host: &str,
    ssh_port: u16,
    ssh_user: &str,
    ssh_password: Option<&str>,
    ssh_private_key: Option<&str>,
    ssh_passphrase: Option<&str>,
    dst_host: String,
    dst_port: u16,
) -> Result<SshTunnelGuard> {
    let client_cfg = Arc::new(russh::client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(60)),
        ..Default::default()
    });

    let mut handle = russh::client::connect(client_cfg, (ssh_host, ssh_port), ClientHandler)
        .await
        .with_context(|| format!("connecting to SSH bastion {ssh_host}:{ssh_port}"))?;

    let authed = if let Some(key_pem) = ssh_private_key.filter(|s| !s.trim().is_empty()) {
        let passphrase = ssh_passphrase.filter(|s| !s.is_empty());
        let secret_key = russh::keys::decode_secret_key(key_pem, passphrase)
            .context("decoding SSH private key PEM")?;
        let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(secret_key), None)
            .context("preparing SSH public-key authenticator")?;
        handle
            .authenticate_publickey(ssh_user, key_with_alg)
            .await
            .context("SSH public-key authentication failed")?
    } else if let Some(pw) = ssh_password.filter(|s| !s.is_empty()) {
        handle
            .authenticate_password(ssh_user, pw)
            .await
            .context("SSH password authentication failed")?
    } else {
        handle
            .authenticate_none(ssh_user)
            .await
            .context("SSH authentication failed")?
    };

    if !authed {
        bail!("SSH authentication rejected by {ssh_host}:{ssh_port} for user \"{ssh_user}\"");
    }

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("binding local SSH tunnel listener")?;
    let local_port = listener.local_addr()?.port();
    let handle = Arc::new(handle);

    let accept_task = tokio::spawn(async move {
        while let Ok((mut local_stream, peer_addr)) = listener.accept().await {
            let ssh = Arc::clone(&handle);
            let target_host = dst_host.clone();
            tokio::spawn(async move {
                if let Ok(channel) = ssh
                    .channel_open_direct_tcpip(
                        target_host,
                        dst_port as u32,
                        peer_addr.ip().to_string(),
                        peer_addr.port() as u32,
                    )
                    .await
                {
                    let mut remote_stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut local_stream, &mut remote_stream)
                        .await;
                }
            });
        }
    });

    Ok(SshTunnelGuard {
        local_port,
        accept_task,
    })
}
