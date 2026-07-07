#!/usr/bin/env bash
# Emit VERSION and DEB_VERSION for release packaging; validate tag vs Cargo.toml.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TAG="${GITHUB_REF_NAME:-${1:-}}"

if [[ -z "$TAG" ]]; then
  echo "usage: $0 vX.Y.Z" >&2
  exit 1
fi

if [[ "$TAG" != v* ]]; then
  echo "tag must start with v (got $TAG)" >&2
  exit 1
fi

VERSION="${TAG#v}"
CARGO_VERSION="$(
  grep -E '^version\s*=' "$ROOT/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
)"

if [[ "$VERSION" != "$CARGO_VERSION" ]]; then
  echo "tag version $VERSION does not match Cargo.toml version $CARGO_VERSION" >&2
  exit 1
fi

echo "VERSION=$VERSION"
echo "DEB_VERSION=${VERSION}-1"
