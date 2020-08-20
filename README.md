# Modtorio

A wrapper for a headless Linux Factorio server to help manage all its aspects.

This current release is barely working, but it's the first one that does. A lot of aspects are missing, and many more aren't probably working. There are bugs.

Dependencies:
* `libsqlite3-dev`
* `libssl-dev`
* `pkg-config`
* `build-essential` or equivalent

Copy the sample config file `modtorio.toml.sample` into `modtorio.toml` before running. Any values not marked with the comment `# required` are optional and can be left out for sane defaults.

## Development

Copy your mod portal credentials into `.env` (sample in `.env.sample`).

You can use a directory called `./sample` for development (git ignores it), just copy a valid Factorio Linux headless server into it.
