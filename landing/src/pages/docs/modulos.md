---
layout: ../../layouts/Docs.astro
title: "Módulos nativos"
description: "Referencia de la biblioteca estándar de Vaca: json, mate, tiempo, sistema, archivo, cadenas, datos y concurrencia."
---

# Módulos nativos

La biblioteca estándar se importa por nombre. No hay dependencias externas ni
un gestor de paquetes: todo lo que aparece aquí viene con el binario.

```vaca
importar json
importar mate como mat
importar archivo
```

| Módulo | Alias | Contenido |
| :--- | :--- | :--- |
| `json` | — | Parseo, serialización y validación |
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
| `argumentos()` | Lista de argumentos pasados tras el archivo |
| `salir(codigo = 0)` | Termina el proceso con un código |
| `directorio()` | Directorio de trabajo actual |
| `plataforma()` | Sistema operativo |

`salir` no se puede capturar con `intentar`/`atrapar`.

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

respuesta = http.peticion("http://127.0.0.1:8080/salud")
```

| Función | Descripción |
| :--- | :--- |
| `servidor(...)` | Levanta un servidor HTTP concurrente |
| `peticion(url, metodo = "GET", cuerpo = "", cabeceras = {})` | Cliente HTTP |
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
