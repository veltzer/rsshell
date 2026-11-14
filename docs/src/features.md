# Features

## Built-in commands

| Command | Description |
|---------|-------------|
| `cd [dir]` | Change directory (defaults to home) |
| `pwd` | Print working directory |
| `echo [args...]` | Print arguments |
| `export VAR=value` | Set environment variable |
| `unset VAR` | Remove environment variable |
| `source file` | Execute commands from file |
| `type cmd` | Show command type (builtin or path) |
| `history` | Show command history |
| `ssh [options] dest [cmd]` | SSH client with OpenSSH config support (see below) |
| `exit [code]` | Exit the shell |

## SSH (built-in `ssh` command)

rsshell ships a native SSH client built on `russh`. It reads the same
`~/.ssh/config` OpenSSH does, so your existing host aliases, users, ports,
identity files and proxies all work:

```text
ssh prod-server                      # alias from ~/.ssh/config
ssh admin@10.0.0.5 -p 2222           # explicit user/port
ssh devbox "docker ps"               # run a remote command, exit code is returned
ssh -F ~/custom-config -i ~/key host # custom config and identity
```

### ProxyCommand

Hosts that sit behind a tunnel are reached exactly the way OpenSSH does it:
the configured command is executed with its stdin/stdout wired to the SSH
transport, so any tool that can relay bytes works unmodified — `ncat`,
`cloudflared access ssh`, `tailcat ssh`, `socat`, …

```text
# ~/.ssh/config
Host secure-app
    HostName secure-app.internal
    ProxyCommand cloudflared access ssh --hostname %h

Host tunnelled
    ProxyCommand ncat --proxy proxy.corp:1080 %h %p
```

```text
ssh secure-app                       # connects through cloudflared
```

The `%h` (host), `%p` (port), `%r` (remote user) and `%%` tokens are
expanded before the command runs, as in `ssh_config(5)`. The proxy process
is spawned without a terminal window and dies with the SSH session.
`-o ProxyCommand=...` overrides the config value for one invocation.

### ProxyJump

```text
Host target
    ProxyJump bastion                # or: ssh -J bastion target (alias style)
```

The jump host is itself resolved through `~/.ssh/config`, so a bastion
behind its own ProxyCommand works too. The tunnel is a `direct-tcpip`
channel through the jump host.

### Authentication

Tried in order: the configured `IdentityFile` (passphrase prompted when
needed), keys held by `ssh-agent`, the default identity files, then an
interactive password (three tries). On first contact with a host the
server key fingerprint is shown and, after confirmation, remembered in
`~/.ssh/known_hosts`; a changed key aborts the connection with OpenSSH's
warning.

## Pipes

Commands can be piped together:

```text
ls -la | grep ".rs" | sort
cat file.txt | wc -l
```

## Aliases

Define aliases in your config file:

```toml
[aliases]
ll = "ls -la"
gs = "git status"
```

## Variable expansion

```text
export NAME=world
echo "hello $NAME"
echo "path is ${HOME}/bin"
```

## Glob expansion

```text
ls *.txt
cat src/**/*.rs
```
