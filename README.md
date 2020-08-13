# Modtorio

Dependencies:
* libsqlite3-dev

Sample .env for development:
```
MODTORIO_PORTAL_USERNAME=fillme
MODTORIO_PORTAL_TOKEN=fillme
```

Sample `modtorio.toml` for development:
```
[general]
log_level = "debug"

[cache]
expiry = 60

[network]
listen = ["[::1]:1337"]
```

To use the `./sample` directory for development, copy a valid Factorio Linux headless server into it (or at the very least, the server executable into `sample/bin/x64`).
