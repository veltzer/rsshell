# Configuration

rsshell uses a TOML configuration file located at `~/.config/rsshell/config.toml`.

Run `rsshell init-config` to generate a default config file.

## Sections

### `[prompt]`

The prompt is built from an ordered list of parts. Each part has a `text`, an optional `color`, and an optional `bold` flag.

```toml
[prompt]
show_exit_code = true
exit_code_color = "red"

[[prompt.parts]]
text = "{user}"
color = "green"

[[prompt.parts]]
text = "@"

[[prompt.parts]]
text = "{host}"
color = "green"

[[prompt.parts]]
text = ":"

[[prompt.parts]]
text = "{cwd}"
color = "blue"
bold = true

[[prompt.parts]]
text = "$ "
```

**Available variables** for the `text` field:
- `{user}` - Current username
- `{host}` - Hostname
- `{cwd}` - Current working directory (with `~` for home)
- `{git_branch}` - Current git branch (empty if not in a repo)

**Available colors**: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`, `none` (default, no color)

**bold**: set to `true` to make the part bold (default: `false`)

When `show_exit_code` is true, non-zero exit codes are shown as `[code]` before the prompt in the configured `exit_code_color`.

#### Example: prompt with git branch

```toml
[[prompt.parts]]
text = "{cwd}"
color = "blue"
bold = true

[[prompt.parts]]
text = " ({git_branch})"
color = "yellow"

[[prompt.parts]]
text = "$ "
```

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
