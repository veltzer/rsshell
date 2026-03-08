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
| `exit [code]` | Exit the shell |

## Pipes

Commands can be piped together:

```
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

```
export NAME=world
echo "hello $NAME"
echo "path is ${HOME}/bin"
```

## Glob expansion

```
ls *.txt
cat src/**/*.rs
```
