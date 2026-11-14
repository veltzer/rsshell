//! Built-in `ssh` command: an SSH client with OpenSSH config support.
//!
//! Resolves the target host through `~/.ssh/config` (see
//! [`crate::ssh_config`]) and connects via russh:
//!
//! - **direct** — TCP to `host:port`
//! - **ProxyJump** — a `direct-tcpip` tunnel through the jump host
//!   (`user@jump:port` syntax supported)
//! - **ProxyCommand** — the named process is spawned with its stdin/stdout
//!   wired to the SSH transport, exactly like OpenSSH does it; `%h`, `%p`,
//!   `%r` and `%%` tokens are expanded first. This is what makes `ncat`,
//!   `cloudflared access ssh`, `tailcat ssh` etc. work without any
//!   per-tool adaptation.
//!
//! Authentication tries, in order: the configured identity file (with an
//! optional passphrase prompt), keys held by `ssh-agent`, default identity
//! files, then a password typed interactively. Interactive sessions request
//! a PTY sized to the local terminal and put the terminal into raw mode.

use std::path::PathBuf;
use std::sync::Arc;

use russh::client::{self, Handle};
use russh::keys::PublicKeyOrCertificate;
use russh::keys::agent::client::AgentClient;
use russh::keys::known_hosts::{check_known_hosts, learn_known_hosts};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg, PublicKey};
use russh::{ChannelMsg, Disconnect, Error as RusshError};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ssh_config::{SshHostConfig, expand_tokens, load_user_ssh_config};

/// Exit code reported when the SSH connection or authentication fails,
/// mirroring OpenSSH.
pub const SSH_FAILURE: i32 = 255;

// ───── CLI argument parsing ─────

/// Parsed `ssh` command line.
#[derive(Debug, Default)]
pub struct SshArgs {
    /// Destination: `host`, `user@host`, `ssh://user@host:port/path`,
    /// or a `~/.ssh/config` alias.
    pub destination: String,
    /// `-p port`
    pub port: Option<u16>,
    /// `-l user` (or from `user@host`)
    pub user: Option<String>,
    /// `-i identity_file`
    pub identity_file: Option<PathBuf>,
    /// `-F config_file`
    pub config_file: Option<PathBuf>,
    /// `-o ProxyCommand=...` style options (ProxyCommand/ProxyJump used).
    pub options: Vec<(String, String)>,
    /// Remote command; none means an interactive shell.
    pub command: Vec<String>,
}

/// Parse ssh-style arguments. Options are only recognised *before* the
/// destination; everything after the destination is the remote command,
/// exactly like OpenSSH (`ssh host ls -la` passes `-la` through).
pub fn parse_args(args: &[String]) -> Result<SshArgs, String> {
    let mut parsed = SshArgs::default();
    let mut iter = args.iter().peekable();
    let mut positional: Vec<String> = Vec::new();
    while let Some(arg) = iter.next() {
        if arg == "--" {
            positional.extend(iter.cloned());
            break;
        }
        // After the destination, everything is the remote command.
        if !positional.is_empty() || !arg.starts_with('-') {
            positional.push(arg.clone());
            continue;
        }
        match arg.as_str() {
            "-p" => {
                let v = iter.next().ok_or("ssh: option -p requires an argument")?;
                parsed.port = Some(
                    v.parse::<u16>()
                        .map_err(|_| format!("ssh: invalid port: {v}"))?,
                );
            }
            "-l" => {
                let v = iter.next().ok_or("ssh: option -l requires an argument")?;
                parsed.user = Some(v.clone());
            }
            "-i" => {
                let v = iter.next().ok_or("ssh: option -i requires an argument")?;
                parsed.identity_file = Some(crate::ssh_config::expand_tilde_path(v));
            }
            "-F" => {
                let v = iter.next().ok_or("ssh: option -F requires an argument")?;
                parsed.config_file = Some(crate::ssh_config::expand_tilde_path(v));
            }
            "-v" | "-vv" | "-vvv" => { /* verbose logging: accepted, unused */ }
            "-o" => {
                let v = iter.next().ok_or("ssh: option -o requires an argument")?;
                let (key, value) = v
                    .split_once('=')
                    .ok_or_else(|| format!("ssh: invalid -o option (expected Key=Value): {v}"))?;
                parsed
                    .options
                    .push((key.trim().to_owned(), value.trim().to_owned()));
            }
            other => {
                return Err(format!("ssh: unsupported option: {other}"));
            }
        }
    }
    if positional.is_empty() {
        return Err("ssh: destination required".to_owned());
    }
    parsed.destination = positional.remove(0);
    parsed.command = positional;
    Ok(parsed)
}

/// Split `user@host` / `user@host:port` / `ssh://user@host:port/path` into
/// `(user, host, port)` parts. Port is `None` when not spelled out.
pub fn split_destination(destination: &str) -> (Option<String>, String, Option<u16>) {
    let mut rest = destination;
    let mut user = None;
    let mut port = None;
    if let Some(url) = rest.strip_prefix("ssh://") {
        let (auth, hostpart) = match url.split_once('@') {
            Some((a, h)) => (Some(a), h),
            None => (None, url),
        };
        user = auth.filter(|a| !a.is_empty()).map(str::to_owned);
        let (host, p) = match hostpart.split_once(':') {
            Some((h, p)) if !p.is_empty() => {
                port = p.parse::<u16>().ok();
                (h, p)
            }
            Some((h, _)) => (h, ""),
            None => (hostpart, ""),
        };
        let _ = p;
        let host = host.split('/').next().unwrap_or(host);
        return (user, host.to_owned(), port);
    }
    if let Some((u, h)) = rest.split_once('@') {
        if !u.is_empty() {
            user = Some(u.to_owned());
        }
        rest = h;
    }
    if let Some((h, p)) = rest.rsplit_once(':')
        && !h.contains(']')
        && let Ok(parsed) = p.parse::<u16>()
    {
        port = Some(parsed);
        rest = h;
    }
    (user, rest.to_owned(), port)
}

// ───── Server host key verification ─────

struct ClientHandler {
    verify: Arc<dyn Fn(&PublicKeyOrCertificate) -> bool + Send + Sync>,
}

impl client::Handler for ClientHandler {
    type Error = RusshError;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        Ok((self.verify)(server_public_key))
    }
}

/// Decision about a host key after the known_hosts check.
enum HostKeyDecision {
    AcceptAndRemember,
    AcceptOnce,
    Reject,
}

/// Check `~/.ssh/known_hosts`; prompt the user on first contact or
/// mismatch — the same flow OpenSSH uses.
fn verify_host_key(host: &str, port: u16, key: &PublicKey) -> HostKeyDecision {
    match check_known_hosts(host, port, key) {
        Ok(true) => HostKeyDecision::AcceptAndRemember,
        Ok(false) => {
            let fp = key.fingerprint(russh::keys::HashAlg::Sha256);
            print!(
                "The authenticity of host '{host}' ({port}) can't be established.\n\
                 Key fingerprint is {fp}.\n\
                 Do you want to continue connecting (yes/no)? "
            );
            let _ = std::io::Write::flush(&mut std::io::stdout());
            let mut answer = String::new();
            if std::io::stdin().read_line(&mut answer).is_err() {
                return HostKeyDecision::Reject;
            }
            match answer.trim().to_lowercase().as_str() {
                "yes" | "y" => HostKeyDecision::AcceptAndRemember,
                _ => {
                    eprintln!("Host key verification failed.");
                    HostKeyDecision::Reject
                }
            }
        }
        Err(russh::keys::Error::KeyChanged { line }) => {
            eprintln!(
                "@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
                 @    WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED!     @\n\
                 @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@\n\
                 Host key for '{host}' has changed (known_hosts line {line}).\n\
                 This could indicate a man-in-the-middle attack, or that the\n\
                 server was reinstalled. Refusing to connect."
            );
            HostKeyDecision::Reject
        }
        Err(_) => HostKeyDecision::AcceptOnce,
    }
}

// ───── Transport ─────

/// The byte stream handed to russh. Either a TCP socket, or the stdio of a
/// ProxyCommand child process. Holding the child keeps the tunnel alive;
/// dropping the stream (session over) kills it (`kill_on_drop`).
/// Variants are boxed to keep the enum small (clippy::large_enum_variant).
enum ProxyStream {
    Tcp(tokio::net::TcpStream),
    Proxy(Box<ProxyChild>),
    /// A `direct-tcpip` channel from a jump host, used as a byte stream.
    Channel(Box<russh::ChannelStream<russh::client::Msg>>),
}

/// A ProxyCommand tunnel: SSH bytes flow to the child's stdin and come
/// back from its stdout. The child handle keeps the process alive and
/// kills it when the tunnel is dropped.
struct ProxyChild {
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
    _child: tokio::process::Child,
}

impl AsyncRead for ProxyStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            ProxyStream::Tcp(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            // SSH bytes coming back from the proxy process' stdout.
            ProxyStream::Proxy(p) => std::pin::Pin::new(&mut p.stdout).poll_read(cx, buf),
            ProxyStream::Channel(s) => std::pin::Pin::new(s.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ProxyStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            ProxyStream::Tcp(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            // SSH bytes we send go to the proxy process' stdin.
            ProxyStream::Proxy(p) => std::pin::Pin::new(&mut p.stdin).poll_write(cx, buf),
            ProxyStream::Channel(s) => std::pin::Pin::new(s.as_mut()).poll_write(cx, buf),
        }
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            ProxyStream::Tcp(s) => std::pin::Pin::new(s).poll_flush(cx),
            ProxyStream::Proxy(p) => std::pin::Pin::new(&mut p.stdin).poll_flush(cx),
            ProxyStream::Channel(s) => std::pin::Pin::new(s.as_mut()).poll_flush(cx),
        }
    }
    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            ProxyStream::Tcp(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            ProxyStream::Proxy(p) => std::pin::Pin::new(&mut p.stdin).poll_shutdown(cx),
            ProxyStream::Channel(s) => std::pin::Pin::new(s.as_mut()).poll_shutdown(cx),
        }
    }
}

// ───── Entry point ─────

/// Run the `ssh` builtin with already word-split arguments. Returns a
/// process exit code like external commands do (255 on failure).
pub fn run(args: &[String]) -> i32 {
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return SSH_FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("ssh: cannot start async runtime: {e}");
            return SSH_FAILURE;
        }
    };
    runtime.block_on(async_run(parsed))
}

/// Fully resolved connection parameters.
struct Resolved {
    host: String,
    port: u16,
    user: String,
    identity: Option<PathBuf>,
    proxy_command: Option<String>,
    proxy_jump: Option<String>,
}

async fn async_run(parsed: SshArgs) -> i32 {
    let config = match &parsed.config_file {
        Some(path) => crate::ssh_config::parse_ssh_config(path),
        None => load_user_ssh_config(),
    };

    let (url_user, url_host, url_port) = split_destination(&parsed.destination);
    let host_cfg = config.resolve(&url_host, url_port.unwrap_or(22));

    let resolved = Resolved {
        host: host_cfg.hostname.clone(),
        port: parsed.port.unwrap_or(host_cfg.port),
        user: parsed
            .user
            .clone()
            .or(url_user)
            .or_else(|| host_cfg.user.clone())
            .unwrap_or_else(local_user_name),
        identity: parsed
            .identity_file
            .clone()
            .or(host_cfg.identity_file.clone()),
        proxy_command: parsed
            .options
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("proxycommand"))
            .map(|(_, v)| v.clone())
            .or(host_cfg.proxy_command.clone()),
        proxy_jump: parsed
            .options
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("proxyjump"))
            .map(|(_, v)| v.clone())
            .or(host_cfg.proxy_jump.clone()),
    };

    // ProxyCommand wins over ProxyJump, as in OpenSSH.
    let stream = if let Some(cmd) = &resolved.proxy_command {
        match spawn_proxy_command(cmd, &host_cfg, &resolved.user) {
            Ok((stdin, stdout, child)) => ProxyStream::Proxy(Box::new(ProxyChild {
                stdin,
                stdout,
                _child: child,
            })),
            Err(e) => {
                eprintln!("ssh: proxy command failed: {e}");
                return SSH_FAILURE;
            }
        }
    } else if let Some(jump) = &resolved.proxy_jump {
        match connect_via_jump(jump, &resolved.host, resolved.port, &config).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("ssh: {e}");
                return SSH_FAILURE;
            }
        }
    } else {
        match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tokio::net::TcpStream::connect((resolved.host.as_str(), resolved.port)),
        )
        .await
        {
            Ok(Ok(s)) => ProxyStream::Tcp(s),
            Ok(Err(e)) => {
                eprintln!(
                    "ssh: connect to host {} port {}: {e}",
                    resolved.host, resolved.port
                );
                return SSH_FAILURE;
            }
            Err(_) => {
                eprintln!(
                    "ssh: connect to host {} port {}: timed out",
                    resolved.host, resolved.port
                );
                return SSH_FAILURE;
            }
        }
    };

    connect_and_run(stream, resolved, parsed).await
}

fn local_user_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .ok()
        .filter(|u| !u.is_empty())
        .or_else(|| {
            nix::unistd::User::from_uid(nix::unistd::getuid())
                .ok()
                .flatten()
                .map(|u| u.name)
        })
        .unwrap_or_else(|| "root".to_owned())
}

// ───── ProxyCommand ─────

/// Spawn the ProxyCommand process with its stdio wired as async pipes.
/// Returns `(stdin, stdout, child)`; the child is kept alive implicitly by
/// the stdio pipes and reaped when both are dropped.
fn spawn_proxy_command(
    template: &str,
    cfg: &SshHostConfig,
    remote_user: &str,
) -> Result<
    (
        tokio::process::ChildStdin,
        tokio::process::ChildStdout,
        tokio::process::Child,
    ),
    String,
> {
    let command_line = expand_tokens(template, cfg, remote_user);
    let words = crate::helpers::parse_command_line(&command_line);
    let (program, args) = words
        .split_first()
        .ok_or_else(|| "empty ProxyCommand".to_owned())?;

    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        // Proxy diagnostics flow to our stderr, like OpenSSH.
        .stderr(std::process::Stdio::inherit())
        // Kill the proxy process if the SSH session dies with us.
        .kill_on_drop(true);

    let child = command
        .spawn()
        .map_err(|e| format!("cannot execute '{program}': {e}"))?;
    let mut child = child;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "proxy command has no stdin".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "proxy command has no stdout".to_owned())?;
    Ok((stdin, stdout, child))
}

// ───── ProxyJump ─────

/// Parse `user@host:port` / `user@host` / `host` jump specs.
pub fn parse_jump_spec(spec: &str) -> (Option<String>, String, u16) {
    let (user, rest) = match spec.split_once('@') {
        Some((u, h)) if !u.is_empty() => (Some(u.to_owned()), h),
        _ => (None, spec),
    };
    let (host, port) = match rest.split_once(':') {
        Some((h, p)) if !p.is_empty() => (h, p.parse::<u16>().unwrap_or(22)),
        _ => (rest, 22),
    };
    (user, host.to_owned(), port)
}

/// Connect to `target:port` through the jump host: an SSH session is
/// opened to the jump host (itself resolved through `~/.ssh/config`, so a
/// jump host behind a ProxyCommand works too) and a `direct-tcpip` channel
/// carries the bytes to the target. The channel is converted to a byte
/// stream and used as the SSH transport for the real connection.
async fn connect_via_jump(
    jump_spec: &str,
    target_host: &str,
    target_port: u16,
    config: &crate::ssh_config::SshConfig,
) -> Result<ProxyStream, String> {
    if jump_spec.contains(',') {
        return Err("multiple jump hosts are not supported yet".to_owned());
    }
    let (jump_user_arg, jump_host_alias, jump_port) = parse_jump_spec(jump_spec);
    let jump_cfg = config.resolve(&jump_host_alias, jump_port);

    let jump_resolved = Resolved {
        host: jump_cfg.hostname.clone(),
        port: jump_cfg.port,
        user: jump_user_arg
            .or(jump_cfg.user.clone())
            .unwrap_or_else(local_user_name),
        identity: jump_cfg.identity_file.clone(),
        proxy_command: jump_cfg.proxy_command.clone(),
        proxy_jump: None, // no nested jumps for now
    };

    // Transport to the jump host itself (TCP or its own ProxyCommand).
    let jump_stream: ProxyStream = if let Some(cmd) = &jump_resolved.proxy_command {
        let (stdin, stdout, child) = spawn_proxy_command(cmd, &jump_cfg, &jump_resolved.user)?;
        ProxyStream::Proxy(Box::new(ProxyChild {
            stdin,
            stdout,
            _child: child,
        }))
    } else {
        let s = tokio::net::TcpStream::connect((jump_resolved.host.as_str(), jump_resolved.port))
            .await
            .map_err(|e| {
                format!(
                    "cannot reach jump host {}:{}: {e}",
                    jump_resolved.host, jump_resolved.port
                )
            })?;
        ProxyStream::Tcp(s)
    };

    let handler = ClientHandler {
        verify: Arc::new(|_| true),
    };
    let config = Arc::new(client::Config::default());
    let handle = client::connect_stream(config, jump_stream, handler)
        .await
        .map_err(|e| {
            format!(
                "handshake with jump host {}:{} failed: {e}",
                jump_resolved.host, jump_resolved.port
            )
        })?;
    let handle = authenticate(
        handle,
        &jump_resolved.user,
        jump_resolved.identity.as_deref(),
    )
    .await
    .map_err(|e| format!("jump host authentication failed: {e}"))?;

    // Ask the jump host to open a TCP channel to the target.
    let channel = handle
        .channel_open_direct_tcpip(target_host, u32::from(target_port), "127.0.0.1", 0)
        .await;
    let channel = match channel {
        Ok(c) => c,
        Err(e) => {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "", "English")
                .await;
            return Err(format!(
                "jump host {} refused tunnel to {target_host}:{target_port}: {e}",
                jump_resolved.host
            ));
        }
    };
    let stream = channel.into_stream();
    Ok(ProxyStream::Channel(Box::new(stream)))
}

// ───── Authentication ─────

async fn authenticate(
    mut handle: Handle<ClientHandler>,
    user: &str,
    identity: Option<&std::path::Path>,
) -> Result<Handle<ClientHandler>, String> {
    // 1. The configured identity file.
    if let Some(path) = identity {
        match try_identity_file(&mut handle, user, path, true).await {
            Ok(true) => return Ok(handle),
            Ok(false) => {}
            Err(e) => eprintln!("ssh: {e}"),
        }
    }

    // 2. Keys held by ssh-agent.
    if let Ok(mut agent) = AgentClient::connect_env().await
        && let Ok(identities) = agent.request_identities().await
    {
        for agent_identity in identities {
            let key = agent_identity.public_key().into_owned();
            let ok = handle
                .authenticate_publickey_with(user, key, None, &mut agent)
                .await
                .is_ok_and(|res| res.success());
            if ok {
                return Ok(handle);
            }
        }
    }

    // 3. Default identity files (skip encrypted ones silently).
    if let Some(home) = dirs::home_dir() {
        for name in ["id_ed25519", "id_ecdsa", "id_rsa"] {
            let path = home.join(".ssh").join(name);
            if path.exists()
                && Some(path.as_path()) != identity
                && try_identity_file(&mut handle, user, &path, false)
                    .await
                    .is_ok_and(|ok| ok)
            {
                return Ok(handle);
            }
        }
    }

    // 4. Interactive password (up to three tries).
    for _ in 0..3 {
        let Some(password) = prompt_password(&format!("{user}@'s password: ")) else {
            break;
        };
        let result = handle
            .authenticate_password(user, &password)
            .await
            .map_err(|e| format!("password authentication error: {e}"))?;
        if result.success() {
            return Ok(handle);
        }
        eprintln!("Permission denied, please try again.");
    }

    Err(format!("all authentication methods failed for {user}"))
}

/// Try one private key file. With `ask_passphrase`, an encrypted key gets a
/// passphrase prompt; otherwise encrypted keys are skipped silently.
async fn try_identity_file(
    handle: &mut Handle<ClientHandler>,
    user: &str,
    path: &std::path::Path,
    ask_passphrase: bool,
) -> Result<bool, String> {
    let key: PrivateKey = match russh::keys::load_secret_key(path, None) {
        Ok(k) => k,
        Err(russh::keys::Error::KeyIsEncrypted) if ask_passphrase => {
            let prompt = format!("Enter passphrase for key '{}': ", path.display());
            let Some(passphrase) = prompt_password(&prompt) else {
                return Ok(false);
            };
            russh::keys::load_secret_key(path, Some(&passphrase))
                .map_err(|e| format!("cannot load key {}: {e}", path.display()))?
        }
        Err(russh::keys::Error::KeyIsEncrypted) => return Ok(false),
        Err(e) => return Err(format!("cannot load key {}: {e}", path.display())),
    };
    let hash = handle
        .best_supported_rsa_hash()
        .await
        .ok()
        .flatten()
        .flatten();
    let result = handle
        .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash))
        .await
        .map_err(|e| format!("publickey authentication error: {e}"))?;
    Ok(result.success())
}

/// Read a password from the terminal with echo disabled.
fn prompt_password(prompt: &str) -> Option<String> {
    use crossterm::tty::IsTty;
    if !std::io::stdin().is_tty() {
        return None;
    }
    print!("{prompt}");
    use std::io::Write;
    let _ = std::io::stdout().flush();

    crossterm::terminal::enable_raw_mode().ok()?;
    // Read one line (until \n or \r) character by character in raw mode.
    let mut password = String::new();
    let mut stdin = std::io::stdin();
    let mut byte = [0u8; 1];
    loop {
        match std::io::Read::read(&mut stdin, &mut byte) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                if byte[0] == b'\n' || byte[0] == b'\r' {
                    break;
                }
                if byte[0] == 3 || byte[0] == 4 {
                    // Ctrl-C / Ctrl-D.
                    let _ = crossterm::terminal::disable_raw_mode();
                    println!();
                    return None;
                }
                password.push(byte[0] as char);
            }
        }
    }
    let _ = crossterm::terminal::disable_raw_mode();
    println!();
    if password.is_empty() {
        None
    } else {
        Some(password)
    }
}

// ───── Session ─────

async fn connect_and_run(stream: ProxyStream, resolved: Resolved, parsed: SshArgs) -> i32 {
    let host = resolved.host.clone();
    let port = resolved.port;
    let remember_key = Arc::new(std::sync::Mutex::new(None::<PublicKey>));

    let remember = remember_key.clone();
    let handler = ClientHandler {
        verify: Arc::new(move |key| match key {
            PublicKeyOrCertificate::PublicKey { key: pk, .. } => {
                match verify_host_key(&host, port, pk) {
                    HostKeyDecision::AcceptAndRemember => {
                        if let Ok(mut slot) = remember.lock() {
                            *slot = Some(pk.clone());
                        }
                        true
                    }
                    HostKeyDecision::AcceptOnce => true,
                    HostKeyDecision::Reject => false,
                }
            }
            // Certificates: accepted without known_hosts bookkeeping.
            PublicKeyOrCertificate::Certificate(_) => true,
        }),
    };

    let config = Arc::new(client::Config {
        keepalive_interval: Some(std::time::Duration::from_secs(30)),
        keepalive_max: 3,
        ..client::Config::default()
    });

    let handle = match client::connect_stream(config, stream, handler).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!(
                "ssh: handshake with {}:{} failed: {e}",
                resolved.host, resolved.port
            );
            return SSH_FAILURE;
        }
    };

    let authenticated = authenticate(handle, &resolved.user, resolved.identity.as_deref()).await;
    let handle = match authenticated {
        Ok(h) => h,
        Err(e) => {
            eprintln!("ssh: {e}");
            return SSH_FAILURE;
        }
    };

    // Persist an accepted first-contact key into known_hosts.
    if let Ok(slot) = remember_key.lock()
        && let Some(key) = slot.as_ref()
    {
        let _ = learn_known_hosts(&resolved.host, resolved.port, key);
    }

    if parsed.command.is_empty() {
        run_interactive_session(handle).await
    } else {
        run_command_session(handle, &parsed.command).await
    }
}

/// Interactive session: open a PTY, wire local terminal <-> channel.
async fn run_interactive_session(handle: Handle<ClientHandler>) -> i32 {
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
    use crossterm::tty::IsTty;

    let mut channel = match handle.channel_open_session().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ssh: cannot open session: {e}");
            return SSH_FAILURE;
        }
    };

    let interactive = std::io::stdin().is_tty();
    let (term, w, h) = if interactive {
        let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_owned());
        match crossterm::terminal::size() {
            Ok((w, h)) => (term, u32::from(w), u32::from(h)),
            Err(_) => (term, 80, 24),
        }
    } else {
        ("dumb".to_owned(), 80, 24)
    };

    if let Err(e) = channel.request_pty(false, &term, w, h, 0, 0, &[]).await {
        eprintln!("ssh: pty request failed: {e}");
        return SSH_FAILURE;
    }
    if let Err(e) = channel.request_shell(false).await {
        eprintln!("ssh: shell request failed: {e}");
        return SSH_FAILURE;
    }

    let raw = interactive && enable_raw_mode().is_ok();

    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut buf = vec![0u8; 4096];
    let mut stdin_open = true;
    let mut exit_code: u32 = 0;

    loop {
        tokio::select! {
            read = stdin.read(&mut buf), if stdin_open => {
                match read {
                    Ok(0) | Err(_) => {
                        stdin_open = false;
                        let _ = channel.eof().await;
                    }
                    Ok(n) => {
                        if channel.data(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { ref data })
                    | Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                        if stdout.write_all(data).await.is_err() {
                            break;
                        }
                        let _ = stdout.flush().await;
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        exit_code = exit_status;
                    }
                    Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }

    if raw {
        let _ = disable_raw_mode();
    }
    println!();
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "English")
        .await;
    exit_code as i32
}

/// Non-interactive: run one command, stream output, return its exit code.
async fn run_command_session(handle: Handle<ClientHandler>, command: &[String]) -> i32 {
    let command_line = command.join(" ");
    let mut channel = match handle.channel_open_session().await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ssh: cannot open session: {e}");
            return SSH_FAILURE;
        }
    };
    if let Err(e) = channel.exec(false, command_line).await {
        eprintln!("ssh: exec failed: {e}");
        return SSH_FAILURE;
    }

    let mut stdout = tokio::io::stdout();
    let mut exit_code: u32 = 0;
    loop {
        match channel.wait().await {
            Some(ChannelMsg::Data { ref data }) => {
                if stdout.write_all(data).await.is_err() {
                    break;
                }
                let _ = stdout.flush().await;
            }
            Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                let mut stderr = tokio::io::stderr();
                let _ = stderr.write_all(data).await;
                let _ = stderr.flush().await;
            }
            Some(ChannelMsg::ExitStatus { exit_status }) => {
                exit_code = exit_status;
            }
            Some(ChannelMsg::Close) | None => break,
            _ => {}
        }
    }
    let _ = handle
        .disconnect(Disconnect::ByApplication, "", "English")
        .await;
    exit_code as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_basic() {
        // Options come before the destination; `-la` after it is the remote
        // command, as with OpenSSH.
        let args: Vec<String> = ["-p", "2222", "user@host1", "ls", "-la"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let parsed = parse_args(&args).unwrap();
        assert_eq!(parsed.destination, "user@host1");
        assert_eq!(parsed.port, Some(2222));
        assert_eq!(parsed.command, vec!["ls".to_owned(), "-la".to_owned()]);
    }

    #[test]
    fn test_parse_args_options() {
        let args: Vec<String> = [
            "-o",
            "ProxyCommand=ncat --proxy proxy:1080 %h %p",
            "-i",
            "~/.ssh/mykey",
            "host1",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        let parsed = parse_args(&args).unwrap();
        assert_eq!(
            parsed.options[0],
            (
                "ProxyCommand".to_owned(),
                "ncat --proxy proxy:1080 %h %p".to_owned()
            )
        );
        assert_eq!(
            parsed.identity_file,
            Some(crate::ssh_config::expand_tilde_path("~/.ssh/mykey"))
        );
    }

    #[test]
    fn test_parse_args_errors() {
        let empty: Vec<String> = vec![];
        assert!(parse_args(&empty).is_err());
        let bad: Vec<String> = ["-p".to_owned(), "host".to_owned()].to_vec();
        assert!(parse_args(&bad).is_err());
        let badopt: Vec<String> = ["-o".to_owned(), "novalue".to_owned(), "h".to_owned()].to_vec();
        assert!(parse_args(&badopt).is_err());
    }

    #[test]
    fn test_split_destination() {
        assert_eq!(split_destination("host1"), (None, "host1".to_owned(), None));
        assert_eq!(
            split_destination("admin@host1"),
            (Some("admin".to_owned()), "host1".to_owned(), None)
        );
        assert_eq!(
            split_destination("admin@host1:2222"),
            (Some("admin".to_owned()), "host1".to_owned(), Some(2222))
        );
        assert_eq!(
            split_destination("ssh://host1:2222"),
            (None, "host1".to_owned(), Some(2222))
        );
        assert_eq!(
            split_destination("ssh://root@host1"),
            (Some("root".to_owned()), "host1".to_owned(), None)
        );
    }

    #[test]
    fn test_parse_jump_spec() {
        assert_eq!(parse_jump_spec("bastion"), (None, "bastion".to_owned(), 22));
        assert_eq!(
            parse_jump_spec("admin@bastion:2222"),
            (Some("admin".to_owned()), "bastion".to_owned(), 2222)
        );
        assert_eq!(
            parse_jump_spec("bastion:2222"),
            (None, "bastion".to_owned(), 2222)
        );
    }

    #[tokio::test]
    async fn test_proxy_stream_roundtrip_over_tcp() {
        // ProxyStream over TCP behaves like the socket itself: spin up a
        // tiny echo server and exercise both AsyncRead and AsyncWrite.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 5];
            sock.read_exact(&mut buf).await.unwrap();
            sock.write_all(&buf).await.unwrap();
        });
        let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut stream = ProxyStream::Tcp(tcp);
        use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
        stream.write_all(b"hello").await.unwrap();
        stream.flush().await.unwrap();
        let mut buf = [0u8; 5];
        stream.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");
        server.await.unwrap();
    }
}
