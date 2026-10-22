---
layout: ../../layouts/Docs.astro
title: "Módulos nativos"
description: "Referencia de la biblioteca estándar de Vaca: json, bd, mate, tiempo, sistema, archivo, cadenas, datos y concurrencia."
---

# Módulos nativos

La biblioteca estándar se importa por nombre. El motor usa Rust estándar y
`rustls` para TLS; no hay frameworks ni
un gestor de paquetes: todo lo que aparece aquí viene con el binario.

```vaca
importar json
importar mate como mat
importar archivo
```

| Módulo | Alias | Contenido |
| :--- | :--- | :--- |
| `json` | — | Parseo, serialización y validación |
| `bd` | `db`, `base_datos` | Conexiones SQLite |
| `mate` | `matematica`, `matemática` | Matemáticas y azar |
| `tiempo` | — | Reloj, fechas y esperas |
| `sistema` | — | Entorno, argumentos y salida |
| `archivo` | `fs` | Lectura y escritura de archivos |
| `cadenas` | `texto` | Utilidades de texto |
| `datos` | — | CSV y DataFrames |
| `concurrencia` | `concurrente` | Tareas, hilos y contadores atómicos |
| `http` | — | Servidor y cliente HTTP |

## `json`

```vaca
importar json

datos = json.parse("{\"nombre\": \"Ada\", \"edad\": 36}")
imprimir(datos.nombre)                       # Ada

texto = json.stringify(datos)                 # compacto
bonito = json.stringify(datos, verdadero)     # con indentación
valido = json.valido(texto)                   # verdadero
```

| Función | Firma | Devuelve |
| :--- | :--- | :--- |
| `parse` | `json.parse(texto)` | Lista, diccionario, texto, número, lógico o `nulo` |
| `stringify` | `json.stringify(valor, indentar = falso)` | Texto JSON |
| `valido` | `json.valido(texto)` | Lógico |

`json.parse` lanza un error de ejecución si el texto no es JSON válido o si el
anidamiento supera los 300 niveles. `json.stringify` serializa listas,
diccionarios e instancias (por sus campos) y limita la profundidad a 128; las
funciones, clases y tareas no son serializables.

## `bd`

Conexiones **SQLite** con consultas parametrizadas. Los valores nunca se
interpolan en el SQL: se envían como parámetros enlazados.

```vaca
importar bd

db = bd.conectar("sqlite:datos.db")
memoria = bd.conectar("sqlite::memory:")
```

### DSN

| DSN | Descripción |
| :--- | :--- |
| `sqlite:datos.db` | Archivo SQLite (relativo al programa) |
| `sqlite:ruta/absoluta.db` | Archivo SQLite |
| `sqlite::memory:` | Base SQLite en memoria |

`bd.sqlite(ruta)` es un atajo de `bd.conectar("sqlite:" + ruta)`.

### Modo de apertura

Por defecto, si el archivo no existe, SQLite **lo crea vacío**. Puedes cambiar
ese comportamiento:

```vaca
# Falla si el archivo no existe (no lo crea).
db = bd.conectar("sqlite:datos.db", {"crear": falso})

# Solo lectura: falla si no existe y rechaza escrituras.
db = bd.abrir_solo_lectura("datos.db")
```

| Opción | Defecto | Efecto |
| :--- | :--- | :--- |
| `crear` | `verdadero` | Si es `falso`, falla cuando el archivo no existe |
| `solo_lectura` | `falso` | Abre en solo lectura; implica `crear: falso` |
| `tiempo_espera` | `5000` | Milisegundos de espera ante un bloqueo |

```vaca
db = bd.conectar("sqlite:datos.db", {
    "solo_lectura": verdadero,
    "tiempo_espera": 2000
})
```

- La carpeta padre debe existir; Vaca no crea directorios.
- `sqlite::memory:` no admite `solo_lectura`.
- Puedes comprobar el modo con `db.solo_lectura()`.

### Crear y consultar

```vaca
importar bd

db = bd.conectar("sqlite:tienda.db")

db.ejecutar("CREATE TABLE IF NOT EXISTS productos (id INTEGER PRIMARY KEY AUTOINCREMENT, nombre TEXT, precio REAL)")

resultado = db.ejecutar(
    "INSERT INTO productos (nombre, precio) VALUES (?, ?)",
    ["Café", 8.5]
)
imprimir(resultado.filas)       # 1
imprimir(resultado.ultimo_id)   # id generado

filas = db.consultar("SELECT id, nombre, precio FROM productos WHERE precio > ?", [5])
para fila en filas {
    imprimir(a"{fila.nombre}: {fila.precio}")
}

uno = db.una("SELECT * FROM productos ORDER BY precio DESC")
nombre = db.valor("SELECT nombre FROM productos WHERE id = ?", [1])
```

### Métodos

| Método | Devuelve |
| :--- | :--- |
| `consultar(sql, params = [])` | Lista de diccionarios (una entrada por fila) |
| `ejecutar(sql, params = [])` | `{filas, ultimo_id}` |
| `una(sql, params = [])` | Primer diccionario o `nulo` |
| `valor(sql, params = [])` | Primera columna de la primera fila o `nulo` |
| `lote(sql)` | Ejecuta varias sentencias sin parámetros |
| `iniciar()` / `confirmar()` / `revertir()` | Transacciones |
| `cerrar()` | Cierra la conexión |
| `abierta()` | Lógico |
| `solo_lectura()` | Lógico |
| `motor()` | `sqlite` |
| `ruta()` | Archivo o `:memory:` |
| `version()` | Versión de SQLite |

`db.tipo` devuelve `conexion`.

### Transacciones

```vaca
db.iniciar()
intentar {
    db.ejecutar("INSERT INTO productos (nombre, precio) VALUES (?, ?)", ["Té", 4.0])
    db.ejecutar("UPDATE productos SET precio = ? WHERE nombre = ?", [4.5, "Té"])
    db.confirmar()
} atrapar error {
    db.revertir()
    lanzar error
}
```

### Tipos

| SQLite | Vaca |
| :--- | :--- |
| `NULL` | `nulo` |
| `INTEGER`, `INT` | `Entero` |
| `REAL`, `DOUBLE`, `FLOAT` | `Decimal` |
| `TEXT`, `VARCHAR`, `CHAR`, `CLOB` | `Texto` |
| `BLOB` | `Texto` (UTF-8 con reemplazo) |
| `BOOLEAN` (guardado como entero) | `0` / `1` |

SQLite no tiene fecha nativa: guarda fechas como `TEXT` ISO o como epoch y
conviértelas en Vaca. Las columnas JSON también son `TEXT`; usa `json.parse`.

### Notas

- Cada `conectar` abre **una** conexión. Los métodos se serializan con un
  mutex; para muchas peticiones concurrentes, abre una conexión por tarea.
- `ultimo_id` solo se devuelve tras un `INSERT`.
- `ejecutar` no devuelve filas: para `INSERT ... RETURNING` usa `consultar`.
- Al abrir en solo lectura, cualquier `INSERT`, `UPDATE` o `DELETE` falla.
- Los parámetros admiten enteros, decimales, textos, lógicos y `nulo`.
- Si un `SELECT` repite el mismo nombre de columna, usa alias (`AS`) para no perder valores en el diccionario.
- No hay agrupación de conexiones ni migraciones automáticas.
- Vaca solo incluye SQLite de forma nativa; para otro motor necesitas otro
  binario o un módulo externo.

## `mate`

```vaca
importar mate

imprimir(mate.PI)
imprimir(mate.raiz(16))              # 4
imprimir(mate.potencia(2, 10))       # 1024
imprimir(mate.redondear(3.14159, 2)) # 3.14
imprimir(mate.aleatorio_entero(1, 6))
```

| Función | Descripción |
| :--- | :--- |
| `PI`, `E` | Constantes |
| `promedio(...)` | Media de una lista o de argumentos variádicos |
| `raiz(x)` | Raíz cuadrada |
| `potencia(base, exponente)` | Potencia |
| `redondear(x, decimales = 0)` | Redondeo |
| `piso(x)` / `techo(x)` | Suelo y techo |
| `absoluto(x)` | Valor absoluto |
| `minimo(...)` / `maximo(...)` | Mínimo y máximo de una lista o argumentos |
| `sumar(...)` | Suma de una lista o argumentos |
| `aleatorio()` | Decimal en `[0, 1)` |
| `aleatorio_entero(min, max)` | Entero en `[min, max]`, ambos inclusive |
| `logaritmo(x, base = e)` | Logaritmo |
| `seno`, `coseno`, `tangente` | Trigonometría |

`promedio`, `minimo`, `maximo` y `sumar` aceptan una lista o argumentos
separados:

```vaca
imprimir(mate.promedio([10, 20, 30]))   # 20
imprimir(mate.promedio(10, 20, 30))     # 20
```

## `tiempo`

```vaca
importar tiempo

imprimir(tiempo.ahora())        # segundos Unix
imprimir(tiempo.millis())       # milisegundos Unix
imprimir(tiempo.fecha_iso())    # 2026-10-20T12:00:00Z
tiempo.dormir(250)              # bloquea 250 ms
```

| Función | Descripción |
| :--- | :--- |
| `ahora()` | Segundos desde la época Unix |
| `millis()` | Milisegundos desde la época Unix |
| `dormir(ms)` | Bloquea el hilo actual |
| `fecha(epoch = ahora)` | Fecha legible en UTC |
| `fecha_iso(epoch = ahora)` | Fecha ISO 8601 en UTC |
| `iso(epoch = ahora)` | Alias de `fecha_iso` |

> `tiempo.dormir` bloquea el hilo. Para esperar sin bloquear el hilo principal,
> lánzalo dentro de una tarea `async`.

## `sistema`

```vaca
importar sistema

puerto = sistema.env("PUERTO", "8080")
imprimir(sistema.directorio())
imprimir(sistema.plataforma())     # linux, macos, windows...
imprimir(sistema.argumentos())     # argumentos extra del programa
sistema.salir(0)
```

| Función | Descripción |
| :--- | :--- |
| `env(nombre, defecto)` | Variable de entorno o defecto |
| `cargar_env(ruta = ".env", obligatorio = falso)` | Lee un `.env` y devuelve un diccionario |
| `argumentos()` | Lista de argumentos pasados tras el archivo |
| `salir(codigo = 0)` | Termina el proceso con un código |
| `directorio()` | Directorio de trabajo actual |
| `plataforma()` | Sistema operativo |

`salir` no se puede capturar con `intentar`/`atrapar`.

### Archivos `.env`

`sistema.cargar_env` lee un archivo `.env` y devuelve un **diccionario**. No
modifica las variables de entorno del proceso:

```vaca
importar sistema

config = sistema.cargar_env(".env")
puerto = sistema.env("PUERTO", config.obtener("PUERTO", "8080"))
imprimir(puerto)
```

- Sin argumentos lee `.env` desde el directorio del programa.
- Si el archivo no existe y `obligatorio` es `falso`, devuelve `{}`.
- Con `obligatorio = verdadero`, lanza un error.
- Soporta `export`, comillas simples y dobles, escapes en comillas dobles y
  comentarios `#` al principio o al final de la línea.
- En valores sin comillas, `#` solo inicia un comentario si va precedido de
  espacio; `abc#def` conserva el `#`.
- No expande `${VARIABLES}` ni valores multilínea.

Ejemplo de `.env`:

```text
# Configuración
PUERTO=8080
NOMBRE="Vaca servidor" # comentario al final
export DEBUG=true
URL=http://localhost:8080/api?x=1
CLAVE='valor # literal'
```

## `archivo`

Las rutas relativas se resuelven desde el archivo principal.

```vaca
importar archivo

archivo.escribir("datos.txt", "hola\n")
archivo.anexar("datos.txt", "mundo\n")
contenido = archivo.leer("datos.txt")
imprimir(contenido)

si archivo.existe("datos.txt") {
    imprimir(archivo.tamano("datos.txt"))
}

archivo.crear_dir("salida")
imprimir(archivo.listar("."))
archivo.eliminar("datos.txt")
```

| Función | Descripción |
| :--- | :--- |
| `leer(ruta)` | Devuelve el contenido como texto |
| `escribir(ruta, texto)` | Crea o sobrescribe |
| `anexar(ruta, texto)` | Añade al final |
| `existe(ruta)` | Lógico |
| `eliminar(ruta)` | Borra un archivo |
| `listar(ruta = ".")` | Lista entradas de un directorio |
| `crear_dir(ruta)` | Crea un directorio |
| `tamano(ruta)` | Tamaño en bytes |
| `ruta_actual()` | Directorio base de resolución |

## `cadenas`

```vaca
importar cadenas

imprimir(cadenas.mayusculas("vaca"))            # VACA
imprimir(cadenas.unir(["a", "b"], "-"))         # a-b
imprimir(cadenas.rellenar_izquierda("7", 3, "0")) # 007
```

| Función | Descripción |
| :--- | :--- |
| `unir(lista, separador = "")` | Une una lista de textos |
| `mayusculas(texto)` / `minusculas(texto)` | Cambia el caso |
| `recortar(texto)` | Quita espacios de los extremos |
| `reemplazar(texto, viejo, nuevo)` | Reemplaza todas las apariciones |
| `dividir(texto, separador)` | Divide en una lista |

`dividir` también está disponible como función global: `dividir("a,b", ",")`.
Con un separador vacío divide en caracteres. Conserva las partes vacías.
| `contiene(texto, buscado)` | Lógico |
| `empieza_con(texto, prefijo)` | Lógico |
| `termina_con(texto, sufijo)` | Lógico |
| `repetir(texto, veces)` | Repite |
| `rellenar_izquierda(texto, ancho, relleno = " ")` | Rellena a la izquierda |
| `rellenar_derecha(texto, ancho, relleno = " ")` | Rellena a la derecha |
| `longitud(texto)` | Número de caracteres |

Los métodos de texto (`texto.mayusculas()`, `texto.dividir(...)`, etc.) hacen
lo mismo sin importar el módulo. Consulta
[El lenguaje](/docs/lenguaje/#metodos-de-texto) para la lista completa.

## `datos`

Un **DataFrame** es un diccionario con dos claves:

```vaca
df = {
    "columnas": ["ciudad", "monto"],
    "filas": [["Madrid", 100], ["Lima", 50]]
}
```

### Leer, ver y guardar

```vaca
importar datos

df = datos.leer_csv("ventas.csv")
datos.ver(df)
datos.guardar_csv(df, "copia.csv")
```

### Consultar

```vaca
datos.dimensiones(df)          # [filas, columnas]
datos.nombres(df)              # ["ciudad", "monto"]
datos.columna(df, "monto")     # [100, 50, 30]
```

### Filtrar y ordenar

```vaca
grandes = datos.filtrar(df, "monto", ">=", 100)
ordenado = datos.ordenar(grandes, "monto", falso)
```

Operadores de filtro: `==`, `=`, `!=`, `<>`, `>`, `<`, `>=`, `<=` y
`contiene`. El tercer argumento de `ordenar` es `ascendente` y vale
`verdadero` por defecto.

### Mutar y agrupar

```vaca
con_iva = datos.mutar(df, "monto_iva", fn(fila) {
    retornar fila.monto * 1.21
})

resumen = datos.agrupar_resumir(df, "ciudad", "monto", "suma")
```

Operaciones de resumen: `suma`, `promedio`, `conteo`, `min`, `max`, `primero`,
`ultimo` y `unir`.

### Otras operaciones

```vaca
seleccion = datos.seleccionar(df, ["ciudad", "monto"])
sin_columna = datos.eliminar_columna(df, "id")
combinado = datos.unir(df1, df2)

json_df = datos.a_json(df)
df2 = datos.desde_json(json_df)
```

| Función | Descripción |
| :--- | :--- |
| `leer_csv(ruta)` | Lee un CSV con cabecera |
| `guardar_csv(df, ruta)` | Escribe un CSV |
| `dimensiones(df)` | `[filas, columnas]` |
| `nombres(df)` | Nombres de columnas |
| `columna(df, nombre)` | Lista de valores |
| `filtrar(df, columna, operador, valor)` | Filtra filas |
| `ordenar(df, columna, ascendente = verdadero)` | Ordena |
| `mutar(df, nueva_columna, funcion_o_valor)` | Añade una columna |
| `agrupar_resumir(df, grupo, columna, operacion)` | Agrupa y resume |
| `ver(df)` | Imprime una tabla |
| `a_json(df)` / `desde_json(valor)` | Conversión |
| `unir(df1, df2)` | Concatena filas |
| `seleccionar(df, columnas)` | Proyecta columnas |
| `eliminar_columna(df, nombre)` | Elimina una columna |

> Los DataFrames son estructuras en memoria. Para datasets que no caben en
> RAM, procesa por lotes o usa una base de datos externa.

## `concurrencia`

```vaca
importar concurrencia

t = concurrencia.tarea(procesar, dato)
resultado = concurrencia.esperar_todas([t])

contador = concurrencia.contador(0)
contador.incrementar()
imprimir(contador.valor())
```

| Función | Descripción |
| :--- | :--- |
| `tarea(funcion, ...args)` | Ejecuta una función en un hilo y devuelve una tarea |
| `paralelo([funciones])` | Ejecuta todas y devuelve sus resultados |
| `esperar_todas(lista)` | Espera todas las tareas y devuelve resultados |
| `contador(inicial = 0)` | Contador atómico |
| `incrementar(contador, cantidad = 1)` | Incrementa y devuelve el nuevo valor |
| `valor(contador)` | Valor actual |
| `reiniciar(contador, valor = 0)` | Reinicia |
| `dormir(ms)` | Duerme el hilo actual |
| `hilos()` | Hilos de CPU disponibles |

Los contadores exponen los mismos métodos:

```vaca
contador.incrementar(10)
contador.valor()
contador.reiniciar()
```

Consulta [HTTP y concurrencia](/docs/servidor/#concurrencia) para el modelo de
hilos, las tareas `async` y los límites.

## `http`

El módulo se documenta en detalle en
[HTTP y concurrencia](/docs/servidor/). Resumen:

```vaca
importar http

http.servidor(8080, manejador)
http.servidor("0.0.0.0", 8080, manejador, 1000)
http.servidor_tls(8443, "servidor.pem", "servidor-clave.pem", manejador, 1000)

respuesta = http.peticion("https://127.0.0.1:8443/salud")
```

| Función | Descripción |
| :--- | :--- |
| `servidor(...)` | Levanta un servidor HTTP concurrente |
| `servidor_tls(...)` | Servidor HTTPS con TLS 1.2/1.3 (rustls) |
| `peticion(url, metodo = "GET", cuerpo = "", cabeceras = {})` | Cliente HTTP/HTTPS |
| `codificar_url(texto)` / `decodificar_url(texto)` | Percent-encoding |
| `estado(codigo)` | Texto de la razón HTTP |

## Módulos de archivo

Cualquier archivo `.vaca` puede importarse como módulo:

```vaca
importar "util.vaca" como util
importar util
```

El módulo expone sus funciones, constantes y variables. Las importaciones se
cachean y los ciclos se detectan con un error.
