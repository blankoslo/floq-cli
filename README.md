# Requirements
The Rust build tool and package manager, Cargo, is required.

Installation instructions can be found here: https://www.rust-lang.org/learn/get-started

# Compiling and running from source
Compilation uses cargo features to select what environment you would like to connect to.
See [src/env.rs](src/env.rs) for more details.

Test: (default)
```
cargo build
```

Production:
```
cargo build --no-default-features --features prod
```

## Compiling
While developing use: `cargo build`
When building an executable for future use, then: `cargo build --release` (`--no-default-features --features prod`) and copy the file at `target/release/floq` to `~/.local/bin` (or whatever folder you like store executables).

## Running
`cargo run` (`--no-default-features --features prod`) lets you run the command based on project files.
But if you have compiled the project and moved the executable to somewhere on your PATH you can of course execute it directly like usual.

`cargo run -- SUBCOMMAND [args]`

or

`floq SUBCOMMAND [args]`

# First time connecting to an environment
If it's your first time using this tool, or you have changed the environment then this command must be run in order to authenticate yourself:

`cargo run -- bruker logg-inn` 

or 

`floq bruker logg-inn`

As stated above, you must also re-authenticate yourself whenever you're changing environment since the same configuration file is used.

# Configuration
Configuration is stored at `~/.floq/session.toml`
