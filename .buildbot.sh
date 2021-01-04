#! /bin/sh

set -e

export CARGO_HOME="`pwd`/.cargo"
export RUSTUP_HOME="`pwd`/.rustup"

if [ -f "$CARGO_HOME" ]; then
    echo "$CARGO_HOME exists."
    exit 1
fi

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh
sh rustup.sh --default-host x86_64-unknown-linux-gnu \
    --default-toolchain nightly \
    --no-modify-path \
    --profile minimal \
    -y
export PATH=`pwd`/.cargo/bin/:$PATH

rm -rf target/
cargo test
rm -rf target/
cargo test --release

rustup toolchain install nightly --allow-downgrade --component rustfmt
cargo +nightly fmt --all -- --check
