---
layout: ../../layouts/Docs.astro
title: "El lenguaje"
description: "Sintaxis, variables, tipos, operadores, control de flujo, funciones, colecciones, POO, errores e importaciones de Vaca."
---

# El lenguaje

Vaca usa llaves `{ }` y palabras clave en español. No depende de la
indentación y el punto y coma es opcional en todo el lenguaje.

```vaca
# Un programa completo y pequeño
importar tiempo

fn saludar(nombre) {
    retornar a"Hola, {nombre}"
}

imprimir(saludar("Mundo"))
imprimir(a"Hoy es {tiempo.fecha_iso()}")
```

## Variables y constantes

Una asignación declara la variable si no existe en ningún ámbito visible.

```vaca
nombre = "Ada"
edad = 36
activo = verdadero
```

Las **anotaciones de tipo** son opcionales y puramente informativas. El motor
no comprueba tipos:

```vaca
contador: Entero = 0
precio: Decimal = 19.99
mensaje: Texto = "hola"
```

Una variable anotada sin `=` se inicializa en `nulo`:

```vaca
pendiente: Texto
imprimir(pendiente)   # nulo
```

Las **constantes** se declaran con `constante` y no se pueden reasignar:

```vaca
constante LIMITE = 100

# Esto falla:
# LIMITE = 200
```

## Tipos

| Tipo | Ejemplos | Notas |
| :--- | :--- | :--- |
| `Entero` | `42`, `-7` | 64 bits con signo |
| `Decimal` | `3.14`, `-0.5` | coma flotante de 64 bits |
| `Texto` | `"hola"`, `a"Hola {nombre}"` | UTF-8 |
| `Logico` | `verdadero`, `falso` | booleanos |
| `Nulo` | `nulo` | ausencia de valor |
| `Lista` | `[1, 2, 3]` | secuencia mutable |
| `Diccionario` | `{"clave": "valor"}` | mapa texto → valor |
| `Funcion` | `fn(x) { ... }` | función o cierre |
| `Clase` / `Instancia` | `nuevo Perro()` | objetos |
| `Tarea` | resultado de `async fn` | se espera con `esperar` |

`tipo(valor)` devuelve el nombre del tipo en tiempo de ejecución:

```vaca
imprimir(tipo(42))          # Entero
imprimir(tipo(3.14))        # Decimal
imprimir(tipo("hola"))      # Texto
imprimir(tipo([1, 2]))      # Lista
```

### Coerción numérica

Los enteros y decimales se mezclan sin conversión explícita. Si alguno de los
dos es decimal, el resultado es decimal:

```vaca
imprimir(10 + 2.5)   # 12.5
imprimir(10 == 10.0) # verdadero
```

La división entre enteros devuelve un entero truncado; usa un decimal para
obtener fracción:

```vaca
imprimir(7 / 2)      # 3
imprimir(7 / 2.0)    # 3.5
```

## Texto e interpolación

El prefijo `a` activa la interpolación de expresiones entre llaves:

```vaca
nombre = "Ada"
edad = 36

imprimir(a"{nombre} tiene {edad} años")
imprimir(a"El doble de 21 es {21 * 2}")
```

Dentro de la interpolación puedes usar variables, operadores, llamadas y
accesos a miembros:

```vaca
persona = {"nombre": "Ada", "pais": "Reino Unido"}
imprimir(a"{persona.nombre} vive en {persona.pais}")
```

Escapes disponibles: `\n`, `\t`, `\"`, `\\` y `\{`.

```vaca
imprimir("Línea 1\nLínea 2")
imprimir("Comillas: \"hola\"")
```

<a id="metodos-de-texto"></a>

### Métodos de texto

Además de las funciones del módulo `cadenas`, cada texto tiene métodos:

| Método | Descripción |
| :--- | :--- |
| `longitud()` | Número de caracteres |
| `mayusculas()` / `minusculas()` | Cambia el caso |
| `recortar()` | Quita espacios de los extremos |
| `contiene(texto)` | Lógico |
| `empieza_con(prefijo)` / `termina_con(sufijo)` | Lógico |
| `dividir(separador)` | Lista de partes |
| `separar_lineas()` | Lista de líneas |
| `reemplazar(viejo, nuevo)` | Texto nuevo |
| `subtexto(inicio, fin)` | Copia un rango; `fin` exclusivo |
| `indice_de(texto)` | Posición o `-1` |
| `repetir(veces)` | Repite el texto |
| `invertir()` | Invierte los caracteres |
| `caracteres()` | Lista de caracteres |
| `a_entero()` / `a_decimal()` / `a_logico()` | Conversión |

```vaca
frase = "  Vaca habla español  "
limpia = frase.recortar()
imprimir(limpia.mayusculas())          # VACA HABLA ESPAÑOL
imprimir(limpia.dividir(" "))          # ["Vaca", "habla", "español"]
imprimir(limpia.reemplazar("Vaca", "Rust"))
```

### Métodos de número

| Método | Descripción |
| :--- | :--- |
| `absoluto()` | Valor absoluto |
| `redondear(decimales = 0)` | Redondeo |
| `piso()` / `techo()` | Suelo y techo |
| `a_texto()` | Convierte a texto |

```vaca
imprimir((-3).absoluto())        # 3
imprimir((3.14159).redondear(2)) # 3.14
imprimir((3.9).piso())           # 3
imprimir((3.1).techo())          # 4
```

## Operadores

| Categoría | Operadores |
| :--- | :--- |
| Aritmética | `+`, `-`, `*`, `/`, `%` |
| Comparación | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| Lógica | `y`, `o`, `no` |
| Asignación compuesta | `+=`, `-=`, `*=`, `/=` |
| Indexación | `lista[i]`, `texto[i]`, `dict["clave"]` |
| Acceso | `objeto.campo`, `modulo.funcion` |

Precedencia, de mayor a menor:

1. Llamadas, miembros e índices.
2. Unarios: `no`, `-`, `+`, `esperar`.
3. `*`, `/`, `%`.
4. `+`, `-`.
5. Comparaciones.
6. `y`.
7. `o`.

`y` y `o` hacen cortocircuito: el lado derecho solo se evalúa si hace falta.

```vaca
si lista.longitud() > 0 y lista[0] == "primero" {
    imprimir("coincide")
}
```

`+` concatena textos y listas:

```vaca
imprimir("hola" + " " + "mundo")
imprimir([1, 2] + [3, 4])   # [1, 2, 3, 4]
```

### Verdad y falsedad

Son falsos: `falso`, `nulo`, `0`, `0.0`, el texto vacío, la lista vacía y el
diccionario vacío. Todo lo demás es verdadero.

```vaca
si no "" {
    imprimir("el texto vacío es falso")
}
```

## Conversiones y builtins

| Función | Qué hace |
| :--- | :--- |
| `entero(valor)` | Convierte a `Entero` (texto, decimal o lógico) |
| `decimal(valor)` | Convierte a `Decimal` |
| `texto(valor)` | Convierte a `Texto` |
| `logico(valor)` | Aplica la verdad de Vaca |
| `longitud(valor)` | Longitud de texto, lista o diccionario |
| `tipo(valor)` | Nombre del tipo |
| `rango(inicio, fin, paso)` | Lista de enteros; `fin` es exclusivo |
| `copiar(valor)` | Copia superficial de listas y diccionarios |
| `afirmar(condicion, mensaje)` | Lanza si la condición es falsa |
| `error(valor)` | Lanza un error |
| `leer(prompt)` | Lee una línea de la entrada estándar |
| `imprimir(...)` | Imprime valores separados por un espacio |

`rango` es exclusivo en el final, a diferencia de `para ... hasta`:

```vaca
imprimir(rango(3))        # [0, 1, 2]
imprimir(rango(1, 4))     # [1, 2, 3]
imprimir(rango(5, 0, -1)) # [5, 4, 3, 2, 1]
```

## Condicionales

```vaca
si edad >= 18 y edad <= 65 {
    imprimir("en edad laboral")
} sino si edad < 18 {
    imprimir("menor")
} sino {
    imprimir("jubilado")
}
```

Las condiciones no necesitan paréntesis. Los bloques siempre usan llaves.

## Bucles

### `mientras`

```vaca
contador = 0

mientras contador < 5 {
    imprimir(contador)
    contador = contador + 1
}
```

### `para ... hasta`

El límite es **inclusivo**. El paso es opcional y puede ser negativo:

```vaca
para i = 1 hasta 5 {
    imprimir(i)          # 1, 2, 3, 4, 5
}

para i = 10 hasta 0 paso -2 {
    imprimir(i)          # 10, 8, 6, 4, 2, 0
}
```

### `para ... en`

Recorre listas, textos (carácter a carácter) y diccionarios (sus claves,
ordenadas):

```vaca
para fruta en ["pera", "uva", "higo"] {
    imprimir(fruta)
}

para letra en "vaca" {
    imprimir(letra)
}

para clave en {"b": 2, "a": 1} {
    imprimir(clave)      # a, b
}
```

### `romper` y `continuar`

```vaca
para i = 1 hasta 10 {
    si i == 3 { continuar }
    si i == 6 { romper }
    imprimir(i)          # 1, 2, 4, 5
}
```

## Funciones

```vaca
fn sumar(a, b) {
    retornar a + b
}

imprimir(sumar(2, 3))    # 5
```

- Las funciones se **hoisten**: puedes llamar a una función declarada más
  abajo en el mismo ámbito.
- Sin `retornar`, una función devuelve `nulo`.
- Una función puede llamarse a sí misma (recursividad).

### Parámetros por defecto

```vaca
fn saludar(nombre, idioma = "ES") {
    si idioma == "EN" {
        retornar a"Hello, {nombre}"
    }
    retornar a"Hola, {nombre}"
}

imprimir(saludar("Ada"))
imprimir(saludar("Ada", "EN"))
```

### Funciones anónimas y closures

```vaca
doble = fn(x) { retornar x * 2 }
imprimir(doble(21))      # 42

fn crear_contador() {
    cuenta = 0
    retornar fn() {
        cuenta = cuenta + 1
        retornar cuenta
    }
}

siguiente = crear_contador()
imprimir(siguiente())    # 1
imprimir(siguiente())    # 2
```

### Callbacks

```vaca
fn ejecutar(accion, valor) {
    retornar accion(valor)
}

imprimir(ejecutar(doble, 5))          # 10
imprimir(ejecutar(fn(x) { retornar x * x }, 5))  # 25
```

### Ámbito

Cada bloque `{ }` crea un ámbito. Las variables declaradas dentro no existen
fuera; las asignaciones a variables exteriores sí las modifican.

```vaca
total = 0

si verdadero {
    total = 10
    _interna = 99
}

imprimir(total)          # 10
# imprimir(_interna)     # error: no está definida
```

### Funciones async

Una función `async` arranca en un hilo y devuelve una **tarea**. `esperar`
bloquea hasta obtener el resultado:

```vaca
async fn consultar(nombre, ms) {
    esperar tiempo.dormir(ms)
    retornar a"{nombre} lista"
}

a = consultar("A", 200)
b = consultar("B", 200)

resultado_a = esperar a
resultado_b = esperar b
```

Consulta [HTTP y concurrencia](/docs/servidor/#concurrencia) para el modelo
completo, los contadores atómicos y los límites.

## Listas

```vaca
numeros = [3, 1, 2]
mixta = [1, "dos", verdadero, nulo]
vacia = []
```

Los índices empiezan en 0. Los índices negativos cuentan desde el final:

```vaca
imprimir(numeros[0])     # 3
imprimir(numeros[-1])    # 2
```

### Métodos de lista

| Método | Descripción |
| :--- | :--- |
| `agregar(valor)` | Añade al final |
| `insertar(indice, valor)` | Inserta en una posición |
| `eliminar(indice)` | Elimina y devuelve el elemento |
| `pop()` | Elimina y devuelve el último |
| `limpiar()` | Vacía la lista |
| `longitud()` | Número de elementos |
| `vacia()` | `verdadero` si está vacía |
| `unir(otra)` | Devuelve una lista nueva concatenada |
| `sublista(inicio, fin)` | Copia un rango; `fin` exclusivo |
| `invertir()` | Invierte en el sitio |
| `ordenar()` / `ordenar(fn)` | Ordena en el sitio con comparador opcional |
| `contiene(valor)` | Busca un valor |
| `indice_de(valor)` | Índice o `-1` |
| `mapear(fn)` | Transforma y devuelve una lista nueva |
| `filtrar(fn)` | Conserva los que cumplen |
| `reducir(fn, inicial)` | Acumula de izquierda a derecha |
| `para_cada(fn)` | Ejecuta por cada elemento |
| `buscar(fn)` | Primer elemento que cumple, o `nulo` |
| `unir_texto(separador)` | Junta los elementos como texto |
| `suma()` / `promedio()` | Solo para números |
| `copiar()` | Copia superficial |

```vaca
numeros = [3, 1, 2]

numeros.agregar(4)
numeros.ordenar()
imprimir(numeros)                       # [1, 2, 3, 4]

pares = numeros.filtrar(fn(n) { retornar n % 2 == 0 })
dobles = numeros.mapear(fn(n) { retornar n * 2 })
total = numeros.reducir(fn(acc, n) { retornar acc + n }, 0)

imprimir(pares)                         # [2, 4]
imprimir(dobles)                        # [2, 4, 6, 8]
imprimir(total)                         # 10
```

## Diccionarios

```vaca
persona = {
    "nombre": "Ada",
    "edad": 36,
    "lenguajes": ["Vaca", "Rust"]
}
```

Se accede con corchetes o con punto. Una clave inexistente devuelve `nulo`
en ambos casos; en una instancia, un campo inexistente sí es un error:

```vaca
imprimir(persona["nombre"])       # Ada
imprimir(persona.nombre)          # Ada
imprimir(persona["falta"])        # nulo
imprimir(persona.falta)           # nulo
```

### Métodos de diccionario

| Método | Descripción |
| :--- | :--- |
| `get(clave, defecto)` | Valor o defecto (`nulo` si se omite) |
| `set(clave, valor)` | Inserta o actualiza |
| `incrementar(clave, cantidad)` | Suma atómicamente a un entero |
| `existe(clave)` | `verdadero` si la clave existe |
| `eliminar(clave)` | Elimina y devuelve el valor |
| `longitud()` | Número de pares |
| `llaves()` | Lista de claves ordenadas |
| `valores()` | Lista de valores en orden de clave |
| `items()` | Lista de pares `[clave, valor]` |
| `unir(otro)` | Devuelve un diccionario combinado |
| `limpiar()` | Vacía el diccionario |
| `copiar()` | Copia superficial |

```vaca
visitas = {}
visitas.incrementar("total")
visitas.incrementar("total", 2)

imprimir(visitas.get("total"))    # 3
imprimir(visitas.llaves())        # ["total"]

para clave en visitas {
    imprimir(a"{clave} = {visitas.get(clave)}")
}
```

## Clases y POO

```vaca
clase Animal {
    fn iniciar(nombre) {
        este.nombre = nombre
    }

    fn hablar() {
        lanzar "cada animal debe implementar hablar"
    }
}

clase Perro(Animal) {
    fn iniciar(nombre, raza) {
        este.nombre = nombre
        este.raza = raza
    }

    fn hablar() {
        retornar a"{este.nombre} dice guau"
    }
}

perro = nuevo Perro("Toby", "mestizo")
imprimir(perro.hablar())
imprimir(perro.raza)
```

- `iniciar` es el constructor y se llama automáticamente con `nuevo`.
- `este` es el receptor dentro de un método.
- `clase Hijo(Padre)` hereda propiedades y métodos.
- Los métodos se resuelven dinámicamente en la instancia.
- `lanzar` en la clase base sirve como contrato abstracto.
- No existe `super`: si necesitas la lógica del padre, factorízala en una
  función o vuelve a implementarla.
- El constructor no puede ser `async`.

## Excepciones

```vaca
fn dividir(a, b) {
    si b == 0 {
        lanzar {"codigo": "DIV_CERO", "mensaje": "no se puede dividir"}
    }
    retornar a / b
}

intentar {
    imprimir(dividir(10, 2))
    imprimir(dividir(1, 0))
} atrapar error {
    imprimir(a"Error {error.codigo}: {error.mensaje}")
}
```

- `lanzar` puede lanzar cualquier valor: texto, diccionario, lista, etc.
- `error("mensaje")` es un atajo para lanzar un texto.
- `intentar` captura `lanzar`, `error(...)`, `afirmar(...)` y errores de
  ejecución (índices, variables inexistentes, aridad, etc.).
- `sistema.salir(codigo)` **no** se captura: termina el proceso.
- El bloque `atrapar` puede omitir la variable: `} atrapar { ... }`.

```vaca
intentar {
    afirmar(1 == 2, "las matemáticas están rotas")
} atrapar error {
    imprimir(a"Capturado: {error}")
}
```

## Importaciones

```vaca
importar json
importar mate como mat
importar "util.vaca" como util
```

- Los módulos nativos se importan por nombre.
- `como` permite usar un alias.
- Los archivos `.vaca` se importan entre comillas y exponen sus funciones,
  constantes y variables públicas.
- Las importaciones se cachean por ruta y se detectan los ciclos.
- Un módulo de archivo puede importar otros módulos.

## Comentarios y estilo

```vaca
# Comentario de una línea
// También de una línea

/*
   Comentario
   de bloque
*/
```

- Las llaves son obligatorias para los bloques.
- El punto y coma es opcional.
- Se recomienda `constante` para valores que no cambian y nombres en
  `MAYÚSCULAS` por convención.
- El analizador rechaza variables locales declaradas y nunca usadas. Usa `_`
  para descartar un valor o un nombre que empiece por `_` para silenciar la
  comprobación.

## Siguiente paso

La biblioteca estándar está documentada en
[Módulos nativos](/docs/modulos/). Para ver programas completos, consulta
[Ejemplos de uso](/docs/ejemplos/).
