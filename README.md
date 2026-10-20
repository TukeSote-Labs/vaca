# Vaca

**Vaca** es un lenguaje de programación completo, con sintaxis en español y
pensado para servidores, backends y procesamiento de datos. Está implementado
en Rust usando únicamente la biblioteca estándar, sin dependencias externas.

- Sintaxis con **llaves** `{ }`: no depende de la indentación.
- Tipos dinámicos con anotaciones opcionales.
- Funciones, cierres (closures), callbacks y recursividad.
- Listas, diccionarios y POO con clases, herencia y `este`.
- Módulos nativos para **HTTP**, **JSON**, **DataFrames/CSV**, archivos,
  matemáticas, texto, tiempo, sistema y **concurrencia**.
- **Concurrencia real con hilos**: `async fn`, `esperar`, contadores atómicos y
  servidor HTTP que atiende conexiones en paralelo.
- CLI con ejecución de archivos, REPL, volcado de tokens y AST.
- Extensión de Zed en `editor/` con gramática Tree-sitter propia.
- Kit de empaquetado en `packaging/`: tarball, Docker, systemd y binario estático.
- 57 pruebas automatizadas y ejemplos ejecutables.

> **Estado:** intérprete de árbol funcional con análisis semántico previo de
> variables locales no usadas y concurrencia real basada en hilos. No usa
> bytecode; consulta [Estado de las fases](#estado-de-las-fases) y
> [Limitaciones](#limitaciones).

## Documentación

| Tema | Dónde |
| :--- | :--- |
| Instalación, CLI y lenguaje | Este `README.md` |
| Ejemplos ejecutables | [`ejemplos/README.md`](ejemplos/README.md) |
| Extensión para Zed | [`editor/README.md`](editor/README.md) |
| Empaquetado, Docker y systemd | [`packaging/README.md`](packaging/README.md) |
| Gramática Tree-sitter | [`editor/tree-sitter-vaca/README.md`](editor/tree-sitter-vaca/README.md) |
| Landing del lenguaje | [`landing/README.md`](landing/README.md) |
| Documentación web completa | [`landing/src/pages/docs`](landing/src/pages/docs) |

---

## Instalación y uso

### Requisitos

- Rust estable (`rustc` y `cargo`).
- No hay dependencias externas: el proyecto compila sin conexión a red.

### Compilar

Desde la raíz del proyecto:

```bash
cd /home/keso/Escritorio/aguila
cargo build --release
```

El ejecutable queda en `target/release/vaca`. También puedes compilar en modo
depuración con `cargo build` (el binario estará en `target/debug/vaca`).

### Ejecutar un programa

Crea un archivo con extensión `.vaca`:

```vaca
# hola.vaca
imprimir("Hola desde Vaca")
```

Ejecútalo con el binario compilado:

```bash
./target/release/vaca hola.vaca
```

O sin compilar manualmente:

```bash
cargo run -- hola.vaca
```

La extensión `.vaca` no es obligatoria para el intérprete, pero es la
convención del lenguaje.

### Ejemplos incluidos

```bash
./target/release/vaca ejemplos/hola.vaca
./target/release/vaca ejemplos/colecciones.vaca
./target/release/vaca ejemplos/texto.vaca
./target/release/vaca ejemplos/closures.vaca
./target/release/vaca ejemplos/errores.vaca
./target/release/vaca ejemplos/algoritmos.vaca
./target/release/vaca ejemplos/grafos.vaca
./target/release/vaca ejemplos/poo.vaca
./target/release/vaca ejemplos/poo_avanzada.vaca
./target/release/vaca ejemplos/patrones.vaca
./target/release/vaca ejemplos/async.vaca
./target/release/vaca ejemplos/concurrencia.vaca
./target/release/vaca ejemplos/archivos.vaca
./target/release/vaca ejemplos/importar.vaca
./target/release/vaca ejemplos/datos.vaca
```

Consulta [`ejemplos/README.md`](ejemplos/README.md) para ver el índice completo,
incluidos los tres ejemplos de servidor HTTP.

### REPL interactivo

```bash
./target/release/vaca repl
```

Dentro del REPL puedes escribir directamente:

```vaca
imprimir(2 + 2)
fn saludar(nombre) { retornar a"Hola, {nombre}" }
saludar("Vaca")
salir
```

El REPL admite bloques de varias líneas: espera hasta que las llaves `{ }`
estén balanceadas. Los `;` son opcionales en todo el lenguaje.

### Servidor HTTP

```bash
./target/release/vaca ejemplos/servidor.vaca
```

En otra terminal:

```bash
curl http://localhost:8080/salud
curl "http://localhost:8080/usuarios?nombre=Ana"
curl -X POST -d '{"hola":"mundo"}' http://localhost:8080/eco
```

Detén el servidor con `Ctrl+C`. Cada conexión se atiende en su propio hilo.

Para una prueba de carga con decenas de conexiones simultáneas:

```bash
./target/release/vaca ejemplos/servidor_concurrente.vaca
python3 scripts/probar_concurrencia.py
```

### Instalar el comando `vaca` globalmente

Para no escribir la ruta completa cada vez:

```bash
cd /home/keso/Escritorio/aguila
cargo install --path .
```

Esto instala `vaca` en `~/.cargo/bin`, que normalmente ya está en el `PATH`.
Después puedes ejecutarlo desde cualquier directorio:

```bash
vaca hola.vaca
vaca repl
vaca ejemplos/hola.vaca
```

### Empaquetar y distribuir

El usuario final no necesita Rust ni Cargo. El script de empaquetado compila y
genera un tarball con el binario, un instalador, la licencia y los ejemplos:

```bash
packaging/empaquetar.sh
# dist/vaca-0.1.0-x86_64-unknown-linux-gnu.tar.gz
```

Instalación desde el paquete:

```bash
tar -xzf dist/vaca-0.1.0-x86_64-unknown-linux-gnu.tar.gz
cd vaca-0.1.0-x86_64-unknown-linux-gnu
./install.sh
```

También hay:

```bash
# Binario estático para Linux
rustup target add x86_64-unknown-linux-musl
packaging/empaquetar.sh --target x86_64-unknown-linux-musl

# Imagen Docker
docker build -f packaging/Dockerfile -t vaca:0.1.0 .
docker run --rm -p 8080:8080 vaca:0.1.0
```

El archivo `packaging/vaca.service` es una unidad systemd de ejemplo lista
para adaptar a tu aplicación. Consulta [`packaging/README.md`](packaging/README.md)
para el despliegue completo, cross-compilación y la estructura recomendada de
una aplicación.

### Comandos disponibles

```bash
vaca <archivo.vaca>        # ejecuta un programa
vaca run <archivo.vaca>    # igual que el anterior
vaca repl                  # consola interactiva (multilínea con llaves)
vaca tokens <archivo>      # muestra los tokens
vaca ast <archivo>         # muestra el AST
vaca --version
vaca --ayuda
```

### Comprobar un programa antes de ejecutarlo

Vaca analiza el programa antes de ejecutar la primera instrucción. Si hay una
variable local declarada y nunca usada, verás un error como este:

```text
error de compilación: variables declaradas pero nunca usadas: variable 'x' (función 'f')
```

El programa no se ejecuta en ese caso. Consulta
[Comprobación de variables no usadas](#comprobación-de-variables-no-usadas).

### Pruebas

```bash
cargo test                              # incluye la ejecución de todos los ejemplos
python3 scripts/probar_servidor.py      # servidor HTTP básico (puerto 8080)
python3 scripts/probar_api.py           # API JSON con validación (puerto 8081)
python3 scripts/probar_concurrencia.py  # carga concurrente (puerto 8090)
```

### Soporte en Zed

La carpeta `editor/` contiene una extensión para Zed con:

- Resaltado de sintaxis.
- Emparejamiento de llaves.
- Indentación y plegado.
- Outline de funciones, clases y constantes.
- Text objects para funciones y clases.
- Comentarios `#`, `//` y `/* ... */`.

Instalación:

1. Abre la paleta de comandos de Zed.
2. Ejecuta **`zed: install dev extension`**.
3. Selecciona la carpeta `editor/`.
4. Abre un archivo `.vaca` y elige el lenguaje **Vaca** si no se detecta solo.

La gramática Tree-sitter vive en `editor/tree-sitter-vaca/`. Solo se necesita
**Bun** si quieres regenerarla:

```bash
cd editor/tree-sitter-vaca
bun install
bun run generate
bun run test
```

Después de modificarla, ejecuta `editor/actualizar.sh` para actualizar la
revisión que usa la extensión de Zed. Consulta
[`editor/README.md`](editor/README.md) para más detalles.

---

## Guía rápida

```vaca
# Comentario de una línea
// También es un comentario

nombre = "Mundo"          # variable dinámica
constante LIMITE = 10     # inmutable; nombre en MAYÚSCULAS
edad: Entero = 3          # anotación de tipo opcional

imprimir("Hola,", nombre)
imprimir(a"{nombre} tiene {edad} años y 2+2={2 + 2}")
```

El punto y coma `;` es **opcional**: puedes usarlo si quieres separar
sentencias, pero nunca es obligatorio.

### Nombres de constantes

Las constantes se declaran con `constante` y son inmutables. Su nombre debe ir
en **MAYÚSCULAS**; el analizador rechaza cualquier nombre con minúsculas:

```vaca
constante LIMITE = 10
constante AÑO = 2026

# error de compilación:
# constante Limite = 10
```

### Comprobación de variables no usadas

Antes de ejecutar, Vaca analiza el AST y rechaza las variables locales que se
declaran pero nunca se leen, al estilo de Go:

```vaca
fn calcular() {
    resultado = 42   # error de compilación: nunca se usa
    retornar 0
}
```

Reglas del análisis:

- Se revisan variables locales de funciones, bloques, bucles y `atrapar`.
- Los **parámetros** y las **variables globales** están exentos, igual que en Go.
- `_` y los nombres que empiezan por `_` desactivan la comprobación para esa
  variable.
- Los imports no se reportan como no usados, porque importar un archivo puede
  ejecutar efectos secundarios.
- Una variable que solo se asigna y nunca se lee también cuenta como no usada.
- Si hay varias, el error las lista todas antes de ejecutar nada del programa.

### Texto e interpolación

El prefijo `a` activa la interpolación. Las expresiones van entre `{ }`.

```vaca
nombre = "Ana"
imprimir(a"Hola, {nombre}")
imprimir(a"Mayúsculas: {nombre.mayusculas()}")
```

Si dentro de la interpolación necesitas un texto literal, usa comillas
simples o escapa las dobles:

```vaca
datos = {"clave": "valor"}
imprimir(a"Resultado: {datos.get('clave')}")
imprimir(a"Resultado: {datos.get(\"clave\")}")
```

### Tipos y conversión

Valores: `Entero`, `Decimal`, `Texto`, `Logico`, `Nulo`, `Lista`,
`Diccionario`, `Funcion`, `Clase`, `Instancia`, `Modulo`.

```vaca
entero("42")       # 42
decimal("3.5")     # 3.5
texto(42)          # "42"
logico(0)          # falso
longitud("Vaca")   # 4
tipo(3.14)         # "Decimal"
```

`10 == 10.0` es `verdadero`. La división entre enteros es entera:
`5 / 2` da `2`; usa `decimal(5) / 2` para obtener `2.5`.

Valores considerados falsos: `falso`, `nulo`, `0`, `0.0`, `""`, listas y
diccionarios vacíos.

### Operadores

| Categoría | Operadores |
| :--- | :--- |
| Aritmética | `+ - * / %` |
| Comparación | `== != < > <= >=` |
| Lógica | `y`, `o`, `no` |
| Asignación | `= += -= *= /=` |
| Acceso | `.`, `[ ]`, `( )` |

Precedencia (de menor a mayor): `o` < `y` < `no` < comparación < suma/resta <
multiplicación/división/módulo < unario `- +` < llamada/acceso.

```vaca
si edad >= 18 y edad <= 65 { ... }
si no activo { ... }
```

### Condicionales

```vaca
si nota >= 9 {
    imprimir("Sobresaliente")
} sino si nota >= 7 {
    imprimir("Notable")
} sino {
    imprimir("Aprobado o suspenso")
}
```

### Bucles

```vaca
para i = 1 hasta 5 {        # 1, 2, 3, 4, 5 (fin inclusivo)
    imprimir(i)
}

para i = 10 hasta 0 paso -2 {
    imprimir(i)
}

para fruta en ["pera", "uva"] {
    imprimir(fruta)
}

mientras activo {
    si cancelado { romper }
    si pausa { continuar }
}
```

`romper` sale del bucle y `continuar` pasa a la siguiente iteración.

### Funciones

```vaca
fn saludar(nombre, idioma = "ES") {
    si idioma == "ES" { retornar a"Hola, {nombre}" }
    retornar a"Hello, {nombre}"
}

fn ejecutar(accion) { accion() }

ejecutar(fn() { imprimir("callback") })

fn factorial(n) {
    si n <= 1 { retornar 1 }
    retornar n * factorial(n - 1)
}
```

Las funciones se pueden declarar después de usarlas dentro del mismo bloque
(se iza la declaración). Los cierres capturan el entorno donde se crean y
pueden modificar variables de ese entorno.

### Listas

```vaca
l = [3, 1, 2]
l.agregar(4)
l.insertar(0, 0)
l.ordenar()
imprimir(l)                 # [0, 1, 2, 3, 4]
imprimir(l.eliminar(0))     # 0
imprimir(l.pop())           # 4

l.invertir()
imprimir(l.sublista(0, 2))
imprimir(l.contiene(2))
imprimir(l.indice_de(2))

dobles = l.mapear(fn(x) { retornar x * 2 })
pares = dobles.filtrar(fn(x) { retornar x % 4 == 0 })
total = dobles.reducir(fn(acc, x) { retornar acc + x }, 0)

imprimir([1, 2].unir([3, 4]))
imprimir([1, 2, 3].unir_texto("-"))
imprimir([1, 2, 3].suma())
imprimir([1, 2, 3].promedio())
```

Métodos: `agregar`, `insertar`, `eliminar`, `pop`, `limpiar`, `longitud`,
`vacia`, `unir`, `sublista`, `invertir`, `ordenar` (con comparador
opcional), `contiene`, `indice_de`, `copiar`, `mapear`, `filtrar`, `reducir`,
`para_cada`, `buscar`, `unir_texto`, `suma`, `promedio`.

Los índices empiezan en 0 y admiten negativos (`-1` es el último).

### Diccionarios

```vaca
d = {"nombre": "Vaca", "edad": 2}
d.set("pais", "ES")
d["ciudad"] = "Madrid"

imprimir(d.get("nombre"))
imprimir(d.get("falta", "valor por defecto"))
imprimir(d.existe("edad"))
imprimir(d.llaves())
imprimir(d.valores())
imprimir(d.items())
imprimir(d.eliminar("edad"))
```

Acceder a una clave inexistente con `d["clave"]` o `d.clave` devuelve `nulo`;
`get` permite un valor por defecto. Las claves se convierten a texto.

### Clases, herencia y `este`

```vaca
clase Animal {
    fn iniciar(nombre) {
        este.nombre = nombre
    }

    fn hablar() {
        lanzar a"{este.nombre} debe implementar hablar()"
    }

    fn describir() {
        retornar a"{este.nombre}: {este.hablar()}"
    }
}

clase Perro(Animal) {
    fn iniciar(nombre, energia = 100) {
        este.nombre = nombre
        este.energia = energia
    }

    fn hablar() { retornar "Guau" }
}

p = nuevo Perro("Firulais")
imprimir(p.describir())   # Firulais: Guau
```

- `iniciar` es el constructor y se llama automáticamente con `nuevo`.
- Una clase hija hereda campos y métodos del padre.
- Los métodos se resuelven dinámicamente: si la hija sobrescribe `hablar`,
  las llamadas desde métodos del padre usan la versión de la hija.
- `lanzar` permite implementar métodos abstractos/contratos.

### Excepciones

```vaca
fn dividir(a, b) {
    si b == 0 { lanzar "división entre cero" }
    retornar a / b
}

intentar {
    imprimir(dividir(10, 2))
    dividir(1, 0)
} atrapar error {
    imprimir(a"Error: {error}")
}
```

`intentar` captura valores lanzados con `lanzar`, la función `error(...)`, y
también errores de ejecución como índices fuera de rango o variables no
definidas. `afirmar(condicion, mensaje)` lanza si la condición es falsa.

### Importaciones

```vaca
importar json
importar mate como mat
importar "util.vaca" como util
```

- Los módulos nativos se importan por nombre.
- Los archivos `.vaca` se importan entre comillas (o por ruta sin extensión)
  y se exponen como un módulo con sus funciones, constantes y variables.
- Las importaciones se cachean y se detectan los ciclos.

### Concurrencia

Cada llamada a una función `async` arranca un hilo del sistema y devuelve una
**tarea**. `esperar` bloquea hasta que la tarea termina y devuelve su
resultado; dos tareas lanzadas seguidas se ejecutan a la vez:

```vaca
importar tiempo

async fn consultar(nombre, ms) {
    tiempo.dormir(ms)
    retornar a"{nombre} lista"
}

a = consultar("A", 200)      # arranca en un hilo
b = consultar("B", 200)      # arranca en otro hilo
resultado_a = esperar a      # espera ~200 ms en total, no 400
resultado_b = esperar b
imprimir(resultado_a, resultado_b)
```

`esperar` también acepta valores que no son tareas y los devuelve tal cual.
Esperar dos veces la misma tarea reutiliza el resultado ya calculado. Si la
tarea lanza un error, `esperar` lo propaga y puede capturarse con
`intentar`/`atrapar`.

El módulo `concurrencia` añade utilidades para paralelismo y estado
compartido:

```vaca
importar concurrencia

# Lanza varias funciones a la vez y espera a todas.
resultados = concurrencia.paralelo([procesar_a, procesar_b])

# Lanza una función con argumentos en un hilo aparte.
tarea = concurrencia.tarea(procesar, dato, 100)

# Contador atómico: no pierde incrementos entre hilos.
contador = concurrencia.contador(0)
contador.incrementar()          # devuelve el nuevo valor
contador.incrementar(10)
contador.valor()
contador.reiniciar()

concurrencia.esperar_todas([tarea])
concurrencia.hilos()            # hilos de CPU disponibles
concurrencia.dormir(ms)
```

Los diccionarios también ofrecen `diccionario.incrementar(clave, cantidad)`
como operación atómica, útil para métricas compartidas:

```vaca
visitas = {}
visitas.incrementar("total")
imprimir(visitas.get("total", 0))
```

---

## Módulos nativos

### `json`

```vaca
importar json
valor = json.parse("{\"a\": [1, 2]}")
texto = json.stringify(valor)         # compacto
bonito = json.stringify(valor, verdadero)
valido = json.valido("{mal")
```

### `mate`

`PI`, `E`, `promedio`, `raiz`, `potencia`, `redondear`, `piso`, `techo`,
`absoluto`, `minimo`, `maximo`, `sumar`, `aleatorio`, `aleatorio_entero`,
`logaritmo`, `seno`, `coseno`, `tangente`.

`promedio`, `minimo`, `maximo` y `sumar` aceptan una lista o argumentos
variádicos.

### `tiempo`

`ahora()` (segundos Unix), `millis()`, `dormir(ms)`, `fecha()` y
`fecha_iso()` en UTC.

### `sistema`

`sistema.env(nombre, defecto)`, `sistema.argumentos()`, `sistema.salir(codigo)`,
`sistema.directorio()`, `sistema.plataforma()`.

### `concurrencia`

`tarea(funcion, ...args)`, `paralelo([funciones])`, `esperar_todas(lista)`,
`contador(inicial)`, `incrementar(contador, cantidad)`, `valor(contador)`,
`reiniciar(contador, valor)`, `dormir(ms)`, `hilos()`.

Los contadores exponen los mismos métodos: `contador.incrementar(cantidad)`,
`contador.valor()` y `contador.reiniciar(valor)`.

### `archivo`

`leer`, `escribir`, `anexar`, `existe`, `eliminar`, `listar`, `crear_dir`,
`tamano`, `ruta_actual`. Las rutas relativas se resuelven desde el archivo
principal (o desde el módulo mientras se ejecuta su nivel superior).

### `cadenas`

`unir`, `mayusculas`, `minusculas`, `recortar`, `reemplazar`, `dividir`,
`contiene`, `empieza_con`, `termina_con`, `repetir`, `rellenar_izquierda`,
`rellenar_derecha`, `longitud`.

### `datos` (DataFrames)

Un DataFrame es un diccionario con `"columnas"` (lista de textos) y `"filas"`
(lista de listas). Funciones:

```vaca
importar datos

df = datos.leer_csv("ventas.csv")
datos.ver(df)

datos.dimensiones(df)                  # [filas, columnas]
datos.nombres(df)                      # nombres de columnas
datos.columna(df, "monto")             # lista de valores

filtrado = datos.filtrar(df, "monto", ">=", 100)
ordenado = datos.ordenar(df, "monto", falso)
con_iva = datos.mutar(df, "monto_iva", fn(fila) { retornar fila.monto * 1.21 })
resumen = datos.agrupar_resumir(df, "ciudad", "monto", "suma")
seleccion = datos.seleccionar(df, ["ciudad", "monto"])
sin_columna = datos.eliminar_columna(df, "id")
combinado = datos.unir(df1, df2)

json_df = datos.a_json(df)
df2 = datos.desde_json(json_df)
datos.guardar_csv(df, "salida.csv")
```

Operadores de filtro: `==`, `!=`, `>`, `<`, `>=`, `<=`, `contiene`.
Operaciones de resumen: `suma`, `promedio`, `conteo`, `min`, `max`,
`primero`, `ultimo`, `unir`.

### `http`

#### Servidor

```vaca
importar http
importar json

fn manejador(peticion) {
    si peticion.metodo == "GET" y peticion.ruta == "/salud" {
        retornar {"cuerpo": {"estado": "ok"}}
    }

    si peticion.metodo == "POST" y peticion.ruta == "/eco" {
        cuerpo = json.parse(peticion.cuerpo)
        retornar {"estado": 201, "cuerpo": {"recibido": cuerpo}}
    }

    retornar {"estado": 404, "cuerpo": {"error": "no encontrado"}}
}

http.servidor(8080, manejador)
```

Firma: `http.servidor(puerto, manejador)`,
`http.servidor(host, puerto, manejador)`,
`http.servidor(puerto, manejador, limite)` o
`http.servidor(host, puerto, manejador, limite)`.

Cada conexión se atiende en su propio hilo, de modo que una petición lenta no
bloquea a las demás. `limite` es el número máximo de conexiones simultáneas
(`0` o negativo significa ilimitado); al superarlo se responde `503` con
`Retry-After: 1`. Si el manejador es `async fn`, el servidor espera la tarea
antes de construir la respuesta.

Como las peticiones se ejecutan a la vez, el estado global compartido debe
usar primitivas atómicas (`concurrencia.contador` o
`diccionario.incrementar`) para evitar carreras en contadores.

La petición que recibe el manejador es un diccionario:

| Campo | Tipo | Descripción |
| :--- | :--- | :--- |
| `metodo` | Texto | `GET`, `POST`, etc. |
| `ruta` | Texto | Ruta decodificada |
| `consulta` | Diccionario | Parámetros de la query |
| `cabeceras` | Diccionario | Cabeceras con claves en minúsculas |
| `cuerpo` | Texto | Cuerpo de la petición |
| `version` | Texto | Versión HTTP |
| `ip` | Texto | IP del cliente |

La respuesta puede ser:

- Un `Texto`: se envía como `200 text/plain`.
- `nulo`: `204 No Content`.
- Un diccionario con `estado`, `cuerpo`, `tipo` y `cabeceras` opcionales.
  Si `cuerpo` es una lista o diccionario se serializa a JSON
  automáticamente.

#### Cliente

```vaca
importar http
respuesta = http.peticion("http://127.0.0.1:8080/salud")
imprimir(respuesta.estado)
imprimir(respuesta.cuerpo)

respuesta = http.peticion("http://api.local/items", "POST", "{\"x\":1}", {
    "Authorization": "Bearer token"
})
```

Firma: `http.peticion(url, metodo = "GET", cuerpo = "", cabeceras = {})`.
Devuelve un diccionario con `estado`, `razon`, `cabeceras` y `cuerpo`.
Solo se admite `http://`; para HTTPS usa un proxy inverso con TLS.

También hay `http.codificar_url`, `http.decodificar_url` y
`http.estado(codigo)`.

#### Laboratorio de estrés

`ejemplos/servidor_concurrente.vaca` está pensado para pruebas de carga:

```bash
cargo build --release
./target/release/vaca ejemplos/servidor_concurrente.vaca
python3 scripts/probar_concurrencia.py
```

Rutas disponibles: `/salud`, `/metricas`, `/lento?ms=...` y `POST /eco`. El
script lanza 50 peticiones de 200 ms a la vez y verifica que terminan en
~0,2 s, además de comprobar que los contadores atómicos no pierden
incrementos y que las peticiones concurrentes reciben identificadores únicos.

Cada conexión reserva 8 MiB de pila virtual, así que limita las conexiones
simultáneas para proteger el proceso frente a slowloris o picos de carga:

```vaca
http.servidor("0.0.0.0", 8090, manejador, 1000)
```

---

## Estructura del proyecto

```text
src/
  main.rs           CLI y REPL
  lib.rs            API pública
  token.rs          Tokens
  lexer.rs          Analizador léxico
  ast.rs            Árbol de sintaxis
  parser.rs         Parser recursivo descendente
  error.rs          Errores léxicos, sintácticos y de ejecución
  value.rs          Valores en tiempo de ejecución
  interprete.rs     Intérprete, entornos y métodos nativos
  modulos/
    mod.rs          Registro de módulos
    json.rs         JSON
    mate.rs         Matemáticas
    http.rs         Servidor y cliente HTTP
    datos.rs        DataFrames y CSV
    archivo.rs      Archivos
    cadenas.rs      Texto
    tiempo.rs       Reloj y fechas
    sistema.rs      Entorno y proceso
    concurrencia.rs Tareas, contadores atómicos e hilos
ejemplos/           Programas de ejemplo
tests/              Pruebas de lenguaje e integración
scripts/            Prueba de humo de los servidores HTTP
packaging/          Empaquetado, Docker y systemd
  empaquetar.sh     Genera el tarball distribuible
  Dockerfile        Imagen de ejecución
  vaca.service      Unidad systemd de ejemplo
editor/             Extensión de Zed
  extension.toml    Manifiesto de la extensión
  languages/vaca/   Configuración y consultas Tree-sitter
  tree-sitter-vaca/ Gramática Tree-sitter (repo Git local)
  actualizar.sh     Actualiza la revisión de la gramática
landing/            Landing del lenguaje (Astro + Bun)
```

---

## Estado de las fases

| Fase | Contenido | Estado |
| :--- | :--- | :--- |
| 0 | Andamiaje y tipos base | Completa |
| 1 | Lexer en español | Completa |
| 2 | Parser y AST con llaves | Completa |
| 3 | Intérprete de árbol | Completa |
| 4 | Compilador a bytecode y VM | **No implementada**: se optó por intérprete de árbol para mantener el proyecto simple |
| 5 | Funciones, cierres, defaults, recursividad | Completa |
| 6 | Listas, diccionarios y métodos nativos | Completa |
| 7 | POO: clases, herencia, `este`, `iniciar`, abstracción | Completa |
| 8 | Módulos JSON, HTTP, mate, tiempo, sistema, archivos | Completa |
| 9 | DataFrames y CSV | Completa |
| 10 | `async`/`await` y concurrencia real | Completa |
| 11 | CLI, REPL, tokens y AST | **Parcial**: extensión de Zed y kit de empaquetado completos; faltan formateador y gestor de paquetes |
| 12 | Type checker, optimizaciones, GC | **Parcial**: análisis semántico de variables no usadas; faltan comprobación de tipos, optimizaciones y GC |

---

## Limitaciones

- **Intérprete de árbol, no bytecode.** Es más simple y suficiente para
  scripts y APIs pequeñas, pero más lento que una VM.
- **Concurrencia basada en hilos del sistema.** Cada `async fn` y cada
  conexión HTTP crea un hilo (8 MiB de pila reservada). Es concurrencia real y
  suficiente para un laboratorio de estrés o una API moderada, pero no es un
  event loop: miles de tareas simultáneas consumen memoria. Para cargas
  masivas, usa `http.servidor(..., limite)` o reparte la carga entre varias
  instancias. No se usan `tokio` ni dependencias externas.
- **Sin recolector de basura.** Listas, diccionarios e instancias usan
  `Arc`/`Mutex`; los ciclos de referencias no se liberan automáticamente.
- **No esperes una tarea desde dentro de sí misma.** `esperar` sobre la tarea
  que se está ejecutando en el hilo actual bloquearía ese hilo para siempre.
  Lanza la tarea desde otro hilo o no la esperes dentro de su propio cuerpo.
- **Anotaciones de tipo informativas.** `x: Entero = 0` se acepta, pero el
  intérprete no realiza comprobación estática de tipos; el tipado es dinámico.
  El análisis semántico actual solo comprueba variables locales no usadas.
- **HTTP sin TLS.** El cliente solo habla `http://`. El servidor tampoco
  termina TLS.
- **Sin `super`.** La herencia funciona por búsqueda dinámica de métodos, pero
  no hay una forma explícita de llamar al método del padre.
- **Límite de recursión.** El intérprete embebido limita la profundidad a 64
  niveles por defecto para no desbordar pilas pequeñas. La CLI usa 256 y los
  helpers `ejecutar_fuente`/`ejecutar_archivo` usan 512 en un hilo de 32 MiB.
  Puedes ajustar `interprete.max_profundidad` si controlas el tamaño de pila.
- **Error de ejecución sin línea.** Los errores léxicos y sintácticos indican
  línea y columna; los de ejecución no incluyen la posición.
- **Rutas de módulos.** Las funciones de un módulo importado resuelven rutas
  relativas contra el archivo principal cuando se ejecutan después del
  import; durante el nivel superior del módulo se resuelven contra el módulo.

## Próximos pasos razonables

1. Compilar el AST a bytecode y añadir una VM con pila.
2. Pool de hilos acotado o event loop para reducir el coste por tarea.
3. Analizador semántico opcional que valide las anotaciones de tipo.
4. Formateador (`vaca fmt`) y gestor de paquetes.
5. Extensiones de editor para resaltado y autocompletado.

---

## Licencia

MIT.
