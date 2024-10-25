#!/bin/bash
set -euf -o pipefail

VERSION="$1"

if ! git diff --quiet; then
  echo "Error: There are unstaged changes in the repository."
  exit 1
fi

cargo install cargo-edit
cargo set-version "$VERSION"

cd fframes-editor
npm version "$VERSION"
yarn bundle:prod
yarn publish

cd webvtt-parser && cargo publish
cd ../svgr-macro && cargo publish
cd ../fframes-media-loaders && cargo publish
cd ../media-dir-macro && cargo publish
cd ../fframes && cargo publish
cd ../fframes-ediotr-controller && cargo publish
cd ../fframes-renderer && cargo publish

