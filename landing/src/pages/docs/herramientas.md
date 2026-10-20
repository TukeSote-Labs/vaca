---
layout: ../../layouts/Docs.astro
title: "Herramientas"
description: "CLI, REPL, comprobación de variables, editor de Zed, empaquetado, pruebas y diagnóstico de errores."
---

# Herramientas

## CLI

```bash
vaca <archivo.vaca>        # ejecuta un programa
vaca run <archivo.vaca>    # igual, con argumentos extra
vaca repl                  # consola interactiva
vaca tokens <archivo>      # volcado de tokens
vaca ast <archivo>         # volcado del AST
vaca --version             # versión
vaca --ayuda               # ayuda + banner cowsay
```

La versión incluye metadatos del último commit: `0.1.0+4.0b78e95` significa
versión semántica `0.1.0`, 4 commits y el hash corto `0b78e95`. Si el binario
se compila sin `.git`, muestra solo la versión de `Cargo.toml`.

Los argumentos que van después del archivo quedan disponibles en
`sistema.argumentos()`:

```bash
vaca app.vaca --puerto 8080 --debug
```

```vaca
importar sistema
imprimir(sistema.argumentos())   # ["--puerto", "8080", "--debug"]
```

## REPL

```bash
vaca repl
```

El REPL acepta bloques de varias líneas y espera hasta que las llaves estén
balanceadas:

```text
vaca> fn doble(n) { retornar n * 2 }
vaca> doble(21)
42
vaca> frutas = ["pera", "uva"]
vaca> frutas.agregar("higo")
vaca> frutas
["pera", "uva", "higo"]
vaca> salir
```

Las variables y funciones definidas en el REPL persisten entre líneas.

## Variables no usadas

Antes de ejecutar, Vaca analiza el programa. Una variable **local** declarada y
nunca leída es un error de compilación:

```vaca
fn calcular() {
    resultado = 42
    retornar 0
}
```

```text
error de compilación: variables declaradas pero nunca usadas: variable 'resultado' (función 'calcular')
```

Reglas:

- Se revisan las variables locales de funciones, bloques, bucles y `atrapar`.
- Los parámetros están exentos.
- Las variables globales están exentas.
- Una variable solo escrita (nunca leída) cuenta como no usada.
- Los nombres que empiezan por `_` están exentos.
- El nombre `_` se usa para descartar valores.

```vaca
para _ = 1 hasta 3 {
    imprimir("tres veces")
}

fn procesar(_ignorado, dato) {
    retornar dato * 2
}
```

## Tokens y AST

```bash
vaca tokens ejemplos/hola.vaca
vaca ast ejemplos/hola.vaca
```

`tokens` imprime línea, columna y tipo de cada token. `ast` imprime el árbol de
sintaxis con `Debug`. Son útiles para depurar el parser o entender cómo se
interpreta una construcción.

## Editor de Zed

La extensión vive en `editor/` e incluye:

- Resaltado de sintaxis con una gramática Tree-sitter propia.
- Emparejamiento de llaves.
- Indentación y plegado.
- Outline de funciones, clases y constantes.
- Text objects para funciones y clases.
- Comentarios `#`, `//` y `/* ... */`.

Instalación:

1. Abre la paleta de comandos de Zed.
2. Ejecuta **`zed: install dev extension`**.
3. Selecciona la carpeta `editor/`.
4. Abre un archivo `.vaca` y elige el lenguaje **Vaca**.

La gramática está en `editor/tree-sitter-vaca/`. Solo necesitas **Bun** si
quieres regenerarla:

```bash
cd editor/tree-sitter-vaca
bun install
bun run generate
bun run test
```

Después de modificarla, ejecuta `editor/actualizar.sh`.

## Empaquetado

### Tarball con instalador

```bash
packaging/empaquetar.sh
# dist/vaca-0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

El paquete incluye el binario, `install.sh`, la licencia y los ejemplos.

### Binario estático

```bash
rustup target add x86_64-unknown-linux-musl
packaging/empaquetar.sh --target x86_64-unknown-linux-musl
```

### Docker

```bash
docker build -f packaging/Dockerfile -t vaca:0.1.0 .
docker run --rm -p 8080:8080 vaca:0.1.0
```

### systemd

`packaging/vaca.service` es una unidad de ejemplo. Ajusta `ExecStart`,
`WorkingDirectory` y `User`.

### Instalación global

```bash
cargo install --path .
vaca --version
```

## Pruebas

```bash
cargo test
python3 scripts/probar_servidor.py
python3 scripts/probar_api.py
python3 scripts/probar_concurrencia.py
```

- `cargo test` ejecuta las pruebas del lenguaje, la integración y todos los
  ejemplos que no son servidores.
- Los scripts de Python levantan servidores reales, hacen peticiones y
  verifican las respuestas.
- `probar_concurrencia.py` lanza 50 peticiones lentas simultáneas y comprueba
  el paralelismo y los contadores atómicos.

## Diagnóstico de errores

### Errores de compilación

| Mensaje | Causa | Solución |
| :--- | :--- | :--- |
| `variables declaradas pero nunca usadas` | Variable local sin leer | Úsala, elimínala o renómbrala con `_` |
| `las constantes ... MAYÚSCULAS` | Nombre de constante con minúsculas | Escríbelo todo en MAYÚSCULAS |
| `error de sintaxis en la línea N, columna M` | Llave, paréntesis o token inesperado | Revisa la línea indicada |
| `error léxico en la línea N, columna M` | Texto sin cerrar, carácter inválido | Revisa comillas y escapes |

### Errores de ejecución

| Mensaje | Causa |
| :--- | :--- |
| `la variable 'x' no está definida` | Nombre mal escrito o fuera de ámbito |
| `índice 10 fuera de rango (longitud 3)` | Índice inválido en lista o texto |
| `la función 'f' espera 2 argumento(s), pero recibió 1` | Aridad incorrecta |
| `división entre cero` / `módulo entre cero` | Operación inválida |
| `desbordamiento de pila: demasiada recursión` | Falta un caso base |
| `'romper' fuera de un bucle` | `romper` mal ubicado |
| `el constructor 'iniciar' no puede ser async` | `iniciar` declarado `async` |
| `no se pudo iniciar el servidor en '...'` | Puerto ocupado o permisos |
| `https no está soportado sin TLS` | Usa `http://` o un proxy |

Los errores léxicos y sintácticos indican línea y columna. Los de ejecución no
incluyen la posición: usa `imprimir` para rastrear el estado.

### Depurar un servidor

```vaca
fn manejador(peticion) {
    imprimir(a"{peticion.metodo} {peticion.ruta} desde {peticion.ip}")

    intentar {
        # lógica
    } atrapar error {
        imprimir(a"Error en {peticion.ruta}: {error}")
        retornar {"estado": 500, "cuerpo": {"error": "interno"}}
    }
}
```

No imprimas secretos ni cuerpos completos en producción. Para carga, usa
`/metricas` con contadores atómicos en lugar de logs por petición.

## Límites conocidos

- No hay compilador a bytecode: es un intérprete de árbol.
- No hay comprobación estática de tipos ni recolector de basura.
- No hay formateador (`vaca fmt`), gestor de paquetes, LSP ni `vaca test`.
- No hay TLS nativo.
- No existe `super` para llamar al método del padre.
- La concurrencia usa hilos del sistema, no un event loop.
- Los ciclos de referencias con `Arc` no se liberan.

Consulta la [referencia rápida](/docs/referencia/#limitaciones) para la lista
completa.
