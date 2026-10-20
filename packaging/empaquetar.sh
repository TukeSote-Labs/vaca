#!/usr/bin/env bash
# Construye un paquete distribuible de Vaca.
#
# Uso:
#   packaging/empaquetar.sh
#   packaging/empaquetar.sh --target x86_64-unknown-linux-musl
#
# El resultado queda en dist/:
#   vaca-<version>-<target>.tar.gz
#   vaca-<version>-<target>.tar.gz.sha256
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

TARGET=""
if [ "${1:-}" = "--target" ]; then
    TARGET="${2:?falta el nombre del target}"
fi

VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
if [ -z "$VERSION" ]; then
    echo "error: no se pudo leer la versión de Cargo.toml" >&2
    exit 1
fi

HOST=$(rustc -vV | sed -n 's/^host: //p')
TARGET=${TARGET:-$HOST}

if [ "$TARGET" = "$HOST" ]; then
    cargo build --release --locked
    BINARIO="target/release/vaca"
else
    cargo build --release --locked --target "$TARGET"
    BINARIO="target/$TARGET/release/vaca"
fi

if [ ! -f "$BINARIO" ]; then
    echo "error: no se encontró el binario $BINARIO" >&2
    exit 1
fi

NOMBRE="vaca-$VERSION-$TARGET"
DESTINO="dist/$NOMBRE"

rm -rf "$DESTINO"
mkdir -p "$DESTINO/bin" "$DESTINO/share/vaca/examples"

cp "$BINARIO" "$DESTINO/bin/vaca"
if command -v strip >/dev/null 2>&1; then
    strip "$DESTINO/bin/vaca" 2>/dev/null || true
fi

cp README.md LICENSE Cargo.toml "$DESTINO/"
cp -R ejemplos/. "$DESTINO/share/vaca/examples/"
printf '%s\n' "$VERSION" > "$DESTINO/VERSION"

cat > "$DESTINO/install.sh" <<'INSTALADOR'
#!/bin/sh
# Instala Vaca en $PREFIX/bin (por defecto ~/.local).
set -eu

BASE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
PREFIX="${PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
SHAREDIR="$PREFIX/share/vaca"

mkdir -p "$BINDIR" "$SHAREDIR/examples"
cp "$BASE/bin/vaca" "$BINDIR/vaca"
chmod +x "$BINDIR/vaca"
cp -R "$BASE/share/vaca/examples/." "$SHAREDIR/examples/"

printf 'Vaca instalado en %s\n' "$BINDIR/vaca"
printf 'Ejemplos instalados en %s\n' "$SHAREDIR/examples"
printf 'Asegúrate de que %s está en el PATH.\n' "$BINDIR"
INSTALADOR

chmod +x "$DESTINO/install.sh"

rm -f "dist/$NOMBRE.tar.gz" "dist/$NOMBRE.tar.gz.sha256"
( cd dist && tar -czf "$NOMBRE.tar.gz" "$NOMBRE" )

if command -v sha256sum >/dev/null 2>&1; then
    ( cd dist && sha256sum "$NOMBRE.tar.gz" > "$NOMBRE.tar.gz.sha256" )
elif command -v shasum >/dev/null 2>&1; then
    ( cd dist && shasum -a 256 "$NOMBRE.tar.gz" > "$NOMBRE.tar.gz.sha256" )
fi

echo "Paquete creado: dist/$NOMBRE.tar.gz"
echo "Directorio:     $DESTINO"
echo "Versión:        $VERSION"
echo "Target:         $TARGET"
