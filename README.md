# rsshell

A simple Unix shell written in Rust.

## Built-in SSH client

rsshell includes a native `ssh` command that understands `~/.ssh/config` —
host aliases, identity files, `ProxyCommand` (ncat, cloudflared, tailcat,
…) and `ProxyJump` all work like they do with OpenSSH:

```text
$ ssh bastion                        # alias from ~/.ssh/config
$ ssh -F config -i key admin@host    # explicit options
$ ssh devbox "uptime"                # one-shot remote command
```

See `docs/src/features.md` for details.
