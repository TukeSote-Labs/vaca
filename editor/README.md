# Extensión de Vaca para Zed

Soporte de lenguaje para archivos `.vaca`:

- Resaltado de sintaxis.
- Emparejamiento de llaves y paréntesis.
- Indentación automática.
- Plegado de bloques.
- Outline de funciones, clases y constantes.
- Text objects para funciones y clases (modo Vim).
- Comentarios de línea (`#`, `//`) y de bloque (`/* ... */`).
- Detección automática de archivos con extensión `.vaca`.

La extensión es declarativa: no necesita compilar Rust ni ejecutar un servidor
de lenguaje. Usa una gramática Tree-sitter propia incluida en
`tree-sitter-vaca/`.

## Instalar en Zed

1. Abre la paleta de comandos de Zed.
2. Ejecuta **`zed: install dev extension`**.
3. Selecciona esta carpeta: `editor/` (la que contiene `extension.toml`).
4. Abre cualquier archivo `.vaca`.

Si el archivo no se detecta automáticamente, usa el selector de lenguaje y
elige **Vaca**.

La gramática se carga con una URL `file://` y una revisión de Git local. Por
eso `tree-sitter-vaca/` es un repositorio Git independiente. Si mueves el
proyecto de carpeta, vuelve a ejecutar:

```bash
editor/actualizar.sh
```

y reinstala la extensión de desarrollo en Zed.

## Estructura

```text
editor/
  extension.toml              Manifiesto de la extensión de Zed
  languages/vaca/
    config.toml               Configuración del lenguaje
    highlights.scm            Resaltado
    brackets.scm              Emparejamiento de llaves
    indents.scm               Indentación
    folds.scm                 Plegado
    outline.scm               Outline
    textobjects.scm           Text objects
    overrides.scm             Scopes de string/comment
  tree-sitter-vaca/           Gramática Tree-sitter (repo Git local)
    grammar.js
    src/parser.c              Parser generado
    queries/                  Consultas de referencia
    test/corpus/              Pruebas de la gramática
  actualizar.sh               Recalcula ruta y revisión de la gramática
```

## Modificar la gramática

Bun es la única herramienta JavaScript necesaria, y solo para regenerar el
parser. El lenguaje en sí y Zed no usan Bun ni Node en tiempo de ejecución.

```bash
cd editor/tree-sitter-vaca
bun install
bun run generate
bun run test
```

Después de cambiar `grammar.js` o las consultas:

```bash
editor/actualizar.sh
```

Esto crea un commit en `tree-sitter-vaca/`, obtiene el nuevo hash y actualiza
`repository` y `rev` en `extension.toml`.

## Limitaciones

- El resaltado de la interpolación `a"...{expr}..."` se aplica al texto
  completo; el interior no se analiza como expresión.
- La extensión no incluye autocompletado, diagnóstico ni LSP.
- La gramática se genera con ABI 14 de Tree-sitter para máxima compatibilidad.
