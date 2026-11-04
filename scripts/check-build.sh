#!/bin/sh
# Run the same locked native, container and package checks used by build.yml.
set -eu
cd "$(dirname "$0")/.."
revision=$(cat .github/vcw-revision)
case "$revision" in
    *[!0-9a-f]*|'') echo 'Invalid pinned VCW revision' >&2; exit 1 ;;
esac
[ "${#revision}" -eq 40 ] || { echo 'VCW revision must be a full commit SHA' >&2; exit 1; }
actual=$(git -C ../vcw rev-parse HEAD)
[ "$actual" = "$revision" ] || {
    echo "VCW checkout differs from .github/vcw-revision: expected $revision, got $actual" >&2
    exit 1
}
git -C ../vcw diff --quiet HEAD -- Cargo.toml crates/metadata crates/types || {
    echo 'VCW dependency sources differ from the pinned commit' >&2
    exit 1
}
docker_sudo=0
if docker info >/dev/null 2>&1; then
    :
elif command -v sudo >/dev/null 2>&1 && sudo -n docker info >/dev/null 2>&1; then
    docker_sudo=1
else
    echo 'Docker daemon unavailable. If Docker requires sudo, run sudo -v, then rerun this script as your normal user.' >&2
    exit 1
fi
docker_cmd() {
    if [ "$docker_sudo" -eq 1 ]; then
        sudo -n docker "$@"
    else
        docker "$@"
    fi
}
docker_cmd buildx version
cargo +stable fmt --check
cargo +stable clippy --locked --all-targets -- -D warnings
cargo +stable build --locked --release
cargo +stable test --locked
./target/release/bcfunl --version
docker_cmd buildx build --load --build-context vcw=../vcw --tag bcfunl:ci .
docker_cmd run --rm bcfunl:ci --version
docker_cmd run --rm --entrypoint sh bcfunl:ci -ec 'for tool in aria2c flock sqlite3; do command -v "$tool"; done'
mkdir -p target/package
package=$(mktemp -d "$PWD/target/package/work.XXXXXXXX")
trap 'rm -rf "$package"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
mkdir -p "$package/scripts" "$package/docs"
cp target/release/bcfunl "$package/"
cp -R data assets skills "$package/"
cp scripts/maintenance.sh scripts/migrate-nas-paths.sql "$package/scripts/"
cp README.md LICENSE "$package/"
cp ../vcw/LICENSE "$package/VCW-LICENSE"
cp docs/REFERENCE.md docs/TRUENAS.md docs/CONTAINERS.md "$package/docs/"
tar -czf target/package/bcfunl-linux-x64.tar.gz -C "$package" .
echo 'Build workflow checks passed: locked native build, tests, container and package.'
