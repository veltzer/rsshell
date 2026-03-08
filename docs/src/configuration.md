# Configuration

rsshell uses a TOML configuration file located at `~/.config/rsshell/config.toml`.

Run `rsshell init-config` to generate a default config file.

## Sections

### `[prompt]`

```toml
[prompt]
format = "{user}@{host}:{cwd}$ "
show_exit_code = true
```

Available prompt variables:
- `{user}` - Current username
- `{host}` - Hostname
- `{cwd}` - Current working directory (with `~` for home)

When `show_exit_code` is true, non-zero exit codes are shown as `[code]` before the prompt.

### `[aliases]`

```toml
[aliases]
ll = "ls -la"
la = "ls -A"
gs = "git status"
```

### `[env]`

Environment variables set at shell startup:

```toml
[env]
EDITOR = "vim"
```

### `[startup]`

Commands to run when the shell starts:

```toml
[startup]
commands = ["echo 'Welcome to rsshell!'"]
```
