# Requirements
The Rust build tool and package manager, Cargo, is required.

Installation instructions can be found here: https://www.rust-lang.org/learn/get-started

# Compiling and running from source
Compilation requires two env variables: `FLOQ_DOMAIN` and `FLOQ_API_DOMAIN`.
These are used to select what environment you would like to connect to.

Blank (prod)
```
export FLOQ_DOMAIN=https://inni.blank.no
export FLOQ_DOMAIN=https://api-blank.floq.no
```

Blank (test)
```
export FLOQ_DOMAIN=https://blank-test.floq.no
export FLOQ_DOMAIN=https://api-blank-test.floq.no
```

FOLQ (prod)
```
export FLOQ_DOMAIN=https://folq.floq.no
export FLOQ_DOMAIN=https://api-folq.floq.no
```

## Compiling
`cargo build`

## Running
`cargo run -- SUBCOMMAND [args]`

# First time connecting to an environment
If it's your first time using this tool, or you have changed the environment then this command must be run in order to authenticate yourself:

`cargo run -- bruker logg-inn`

As stated above, you must also re-authenticate yourself whenever you're changing environment since the same configuration file is used.

# Configuration
Configuration is stored at `~/.floq/user.config.toml`
