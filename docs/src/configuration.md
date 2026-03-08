# Configuration

rsshell uses a TOML configuration file located at `~/.config/rsshell/config.toml`.

Run `rsshell init-config` to generate a default config file.

## Sections

### `[prompt]`

The prompt is built from an ordered list of parts. Each part has a `text` and optional styling.

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

#### Variables

| Variable | Description |
|----------|-------------|
| `{user}` | Current username |
| `{host}` | Hostname |
| `{cwd}` | Current working directory (`~` for home) |
| `{cwd_basename}` | Basename of current directory |
| `{git_branch}` | Current git branch (empty if not in a repo) |
| `{git_dirty}` | `dirty` or `clean` (empty if not in a repo) |
| `{git_status}` | `*` if dirty, empty if clean or not in a repo |
| `{git_sha}` | Full git commit SHA |
| `{git_sha_short}` | Short git commit SHA |
| `{git_repo}` | Git repository name |
| `{date}` | Current date (YYYY-MM-DD) |
| `{time}` | Current time (HH:MM:SS) |
| `{shell}` | Shell name (`rsshell`) |
| `{newline}` | A newline character |
| `{$}` | A literal `$` |

#### Colors

Standard: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`

Bright: `bright_black` (also `gray`/`grey`), `bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`, `bright_white`

Use `none` (default) for no color.

Both `color` (foreground) and `bg` (background) accept any of the above.

#### Styles

| Style | Description |
|-------|-------------|
| `bold = true` | Bold text |
| `dim = true` | Dimmed text |
| `italic = true` | Italic text |
| `underline = true` | Underlined text |
| `strikethrough = true` | Strikethrough text |

When `show_exit_code` is true, non-zero exit codes are shown as `[code]` before the prompt in the configured `exit_code_color`.

#### Example: prompt with git branch and dirty indicator

```toml
[[prompt.parts]]
text = "{user}"
color = "green"

[[prompt.parts]]
text = " "

[[prompt.parts]]
text = "{cwd}"
color = "blue"
bold = true

[[prompt.parts]]
text = " ({git_branch}{git_status})"
color = "yellow"

[[prompt.parts]]
text = "{newline}{$} "
```

#### Example: two-line prompt with date

```toml
[[prompt.parts]]
text = "[{date} {time}] "
color = "gray"
dim = true

[[prompt.parts]]
text = "{user}@{host}"
color = "green"

[[prompt.parts]]
text = ":"

[[prompt.parts]]
text = "{cwd}"
color = "bright_blue"
bold = true

[[prompt.parts]]
text = "{newline}{$} "
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
