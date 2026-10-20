# Ejemplos de Vaca

Ejecuta cualquier ejemplo desde la raíz del proyecto:

```bash
./target/release/vaca ejemplos/<archivo>.vaca
```

O con Cargo:

```bash
cargo run -- ejemplos/<archivo>.vaca
```

## Índice

| Archivo | Tema |
| :--- | :--- |
| `hola.vaca` | Variables, interpolación, listas y diccionarios. |
| `algoritmos.vaca` | Two Sum, búsqueda binaria, Fibonacci, paréntesis y merge sort. |
| `grafos.vaca` | BFS, DFS, camino mínimo sin pesos y orden topológico. |
| `colecciones.vaca` | `mapear`, `filtrar`, `reducir`, orden, búsqueda y diccionarios. |
| `texto.vaca` | Texto, Unicode, divisiones, reemplazos y conversiones. |
| `closures.vaca` | Closures, callbacks, composición y memoización. |
| `errores.vaca` | `lanzar`, `intentar`/`atrapar`, `afirmar` y errores de ejecución. |
| `poo.vaca` | Clases, herencia, constructores y abstracción. |
| `poo_avanzada.vaca` | Observer, Strategy, contratos e inyección de dependencias. |
| `patrones.vaca` | Singleton, middleware, cola FIFO, caché y rate limiter. |
| `async.vaca` | Sintaxis `async`/`esperar`. La ejecución actual es síncrona. |
| `archivos.vaca` | Lectura, escritura, anexado, directorios y borrado. |
| `importar.vaca` + `util.vaca` | Importación de módulos escritos en Vaca. |
| `datos.vaca` + `ventas.csv` | DataFrames, filtros, orden, mutación y agrupación. |
| `servidor.vaca` | Servidor HTTP básico con rutas y JSON. |
| `servidor_api.vaca` | API JSON con validación, rate limiting y estado en memoria. |

## Pruebas de los servidores

```bash
python3 scripts/probar_servidor.py   # ejemplos/servidor.vaca (puerto 8080)
python3 scripts/probar_api.py        # ejemplos/servidor_api.vaca (puerto 8081)
```

Los scripts levantan el servidor, hacen peticiones reales, verifican las
respuestas y detienen el proceso.
