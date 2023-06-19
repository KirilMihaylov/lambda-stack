set windows-shell := ["cmd", "/c"]

alias b := build
alias r := release

update:
    cargo update

fix:
    cargo fix --allow-dirty --allow-staged
    cargo fmt

fmt:
    cargo fmt

lib: update
    cargo build --release -p lambda-lib --target wasm32-unknown-unknown

build: update
    cargo build

release: update
    cargo build --release
