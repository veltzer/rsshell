# TOFIX

Findings from a code scan on 2026-10-04.

## High

- `src/commands.rs:38` - `expand_env_vars` runs before `expand_local_vars` (`src/commands.rs:41`) and deletes every `$NAME` that is not in the process environment (`src/helpers.rs:913`), so shell variables set with `X=hello` never expand: `X=hello` then `echo [$X]` prints `[]`. Expand local variables first (or make one expander that consults local vars, then env).

## Medium

- `src/helpers.rs:890` - `expand_env_vars` is applied to the raw line with no quote awareness, so `echo '$HOME'` prints the home directory although the docs (`CLAUDE.md:32`, `docs/src/introduction.md:15`) advertise single-quote handling; skip expansion inside single quotes.
- `src/main.rs:114` - `trimmed.starts_with(' ')` can never be true because `trimmed` is `line.trim()` (`src/main.rs:108`), so `history.ignore_space` is dead and space-prefixed commands are always recorded; test `line.starts_with(' ')` instead.
- `src/commands.rs:122` - redirections are only honoured by `echo` and external commands; builtins such as `pwd > f`, `type x > f` and `history > f` ignore `redir` and print to the terminal without creating the file. Pass the redirection to every builtin that writes output.
- `src/commands.rs:428` - `history` prints the history file, but the session's history is only written to that file on exit (`src/main.rs:171`), so commands from the current session never show; and `history clear` (`src/commands.rs:412`) deletes the file only for the editor to write the in-memory history back on exit. Operate on the editor's in-memory history.
- `Cargo.toml:11` - `tokio` (with `features = ["full"]`) and `serde_json` (`Cargo.toml:18`) are declared but used nowhere in `src/` or `tests/`; remove them to cut build time and the dependency surface.
- `docs/src/installation.md:23` - says GitHub Releases include Windows (x86_64) binaries, but the release matrix in `.github/workflows/ci.yml:144` has no Windows target and `docs/src/platform-limitations.md:136` says Windows was removed; drop Windows from the list.

## Low

- `src/helpers.rs:275` - a config file that fails to parse is silently replaced by `Config::default()`, so one typo discards the user's prompt, aliases and env with no message; print the TOML error to stderr.
- `docs/src/platform-limitations.md:103` - claims `build.rs` shells out to `date`, but `build.rs:78` now formats the timestamp in-process; drop `build.rs` from that section.
- `CLAUDE.md:20` - says there are "Three source files" and that `src/main.rs` has unit tests in `mod tests`; there is also `src/lib.rs`, and the tests live in `tests/*.rs` with no `mod tests` in `src/main.rs`. Update the description.
