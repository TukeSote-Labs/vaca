#!/usr/bin/env sh
# Actualiza la revisión y la ruta absoluta de la gramática que usa la
# extensión de Zed. La gramática vive en un repositorio Git local para que
# Zed pueda clonarla mediante una URL file://.
set -eu

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
GRAMMAR="$DIR/tree-sitter-vaca"

if [ ! -d "$GRAMMAR/.git" ]; then
    git -C "$GRAMMAR" init -q
fi

git -C "$GRAMMAR" add -A
if ! git -C "$GRAMMAR" rev-parse --verify HEAD >/dev/null 2>&1; then
    git -C "$GRAMMAR" -c user.name="Vaca" -c user.email="vaca@localhost" \
        commit -q -m "Gramática inicial de Vaca"
elif ! git -C "$GRAMMAR" diff --cached --quiet; then
    git -C "$GRAMMAR" -c user.name="Vaca" -c user.email="vaca@localhost" \
        commit -q -m "Actualizar gramática de Vaca"
fi

REV=$(git -C "$GRAMMAR" rev-parse HEAD)

sed -i "s|^repository = \"file://.*\"|repository = \"file://$GRAMMAR\"|" "$DIR/extension.toml"
sed -i "s|^rev = \".*\"|rev = \"$REV\"|" "$DIR/extension.toml"

printf 'Extensión actualizada.\n  gramática: %s\n  revisión:  %s\n' "$GRAMMAR" "$REV"
