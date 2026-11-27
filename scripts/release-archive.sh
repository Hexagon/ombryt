#!/bin/sh
# One release archive, into dist/:
#
#   ombryt-<target>.tar.gz   (.zip for Windows)   ombryt-<target>/{ombryt, LICENSE-MIT, LICENSE-APACHE, README.md,
#                                                 CHANGELOG.md}
#
# The names carry no version, so that .../releases/latest/download/<name> always gets the newest release.
# Linux targets (*-linux-musl) are built static with cargo-zigbuild (pip install ziglang; cargo install
# cargo-zigbuild); others with plain cargo, on a runner of that OS. Archives have fixed names, order, owners and
# times, so the same binary gives the same archive.
#
#   scripts/release-archive.sh x86_64-unknown-linux-musl
set -eu
cd "$(dirname "$0")/.."
target=${1:?usage: release-archive.sh <target>}
name=ombryt-$target

case $target in
  *-linux-musl) cargo zigbuild --release --locked --target "$target" ;;
  *) cargo build --release --locked --target "$target" ;;
esac

exe=ombryt
case $target in *-windows-*) exe=ombryt.exe ;; esac

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/$name" dist
cp "target/$target/release/$exe" LICENSE-MIT LICENSE-APACHE README.md CHANGELOG.md "$stage/$name/"

case $target in
  *-windows-*)
    out=$(pwd)/dist/$name.zip
    rm -f "$out"
    (cd "$stage" && 7z a -tzip -bd "$out" "$name" > /dev/null)
    ;;
  *)
    tar=$(command -v gtar || command -v tar) # GNU tar; macOS has it as gtar
    "$tar" --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner --format=gnu \
      --mode='u+rwX,go+rX,go-w' -C "$stage" -cf - "$name" | gzip -n -9 > "dist/$name.tar.gz"
    ;;
esac
ls -l dist/
