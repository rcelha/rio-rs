import? 'local.just'

# Installs the pre-commit hook as soon as one runs `just`
_ := `ln -sf ${PWD}/pre-commit ${PWD}/.git/hooks/`

# Prints this message
help:
    @just --list

# Prints the TODOs found throughout the repository
todo:
    @ag --rust -Q 'expect("TODO'

# Prints missing docs
todo-docs:
    @ag --rust '/// TODO'

fmt:
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo fix --allow-dirty --allow-staged
    cargo fmt
    cargo check --all-features

# Runs tests for the main project
test:
    cargo nextest run --no-fail-fast
    cargo test --doc

# Do something
bump-type:
    #!/usr/bin/env bash
    changelog=$(git changelog -- --reverse --since "2 years ago" )
    if [[ $changelog =~ "BREAKING" ]]; then
        echo MAJOR
    elif [[ $changelog =~ "feat:" ]]; then
        echo MINOR
    else
        echo PATCH
    fi

# Install development tools
install-tools:
    cargo install cargo-watch
    cargo install cargo-nextest
    cargo install cargo-readme
    cargo install simple-http-server
    cargo install cargo-release
    cargo install git-cliff

# Release a new version
release:
    @echo just run "cargo release"

# Generates the README.md file
readme:
    cargo readme -t ../README.tpl.md -r rio-rs > README.md

# Generate changelog
changelog:
    git cliff -o CHANGELOG.md

# Generate docs and run a server for preview
YELLOW:='\033[0;33m'
NC:='\033[0m'
serve-docs:
    cargo doc -p rio-rs -p rio-macros
    @echo -e "you can open your docs at {{YELLOW}}http://$(hostname):8000/rio_rs/{{NC}}"
    simple-http-server --nocache -i ./target/doc

# Run serve-docs, and refresh it when the code changes
serve-docs-watch:
    cargo watch -s 'just serve-docs'
