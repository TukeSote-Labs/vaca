---
layout: ../../layouts/Docs.astro
title: "Referencia rápida"
description: "Chuleta de Vaca: palabras clave, operadores, builtins, métodos, módulos, errores y limitaciones."
---

# Referencia rápida

## Palabras clave

| Categoría | Palabras |
| :--- | :--- |
| Control | `si`, `sino`, `mientras`, `para`, `hasta`, `paso`, `en`, `romper`, `continuar`, `retornar` |
| Errores | `lanzar`, `intentar`, `atrapar`, `afirmar`, `error` |
| Declaraciones | `fn`, `clase`, `nuevo`, `este`, `constante`, `importar`, `como` |
| Concurrencia | `async`, `esperar`, `await` |
| Literales | `verdadero`, `falso`, `nulo` |
| Lógica | `y`, `o`, `no` |
| Comentarios | `#`, `//`, `/* ... */` |

## Operadores

| Prioridad | Operadores |
| :--- | :--- |
| 1 (mayor) | Llamada `f()`, miembro `a.b`, índice `a[i]` |
| 2 | `no`, `-`, `+`, `esperar` |
| 3 | `*`, `/`, `%` |
| 4 | `+`, `-` |
| 5 | `==`, `!=`, `<`, `>`, `<=`, `>=` |
| 6 | `y` |
| 7 (menor) | `o` |

Asignación compuesta: `+=`, `-=`, `*=`, `/=`. El `;` es opcional.

## Tipos y verdad

| Tipo | Literales |
| :--- | :--- |
| `Entero` | `42`, `-7` |
| `Decimal` | `3.14` |
| `Texto` | `"hola"`, `a"Hola {nombre}"` |
| `Logico` | `verdadero`, `falso` |
| `Nulo` | `nulo` |
| `Lista` | `[1, 2, 3]` |
| `Diccionario` | `{"clave": valor}` |

Son falsos: `falso`, `nulo`, `0`, `0.0`, `""`, `[]`, `{}`. Todo lo demás es
verdadero.

## Builtins

| Función | Uso |
| :--- | :--- |
| `imprimir(...)` | Imprime valores separados por espacio |
| `leer(prompt)` | Lee una línea |
| `entero(x)` | Convierte a entero |
| `decimal(x)` | Convierte a decimal |
| `texto(x)` | Convierte a texto |
| `logico(x)` | Convierte a lógico |
| `longitud(x)` | Longitud de texto, lista o diccionario |
| `dividir(texto, separador)` | Divide un texto y devuelve una lista |
| `tipo(x)` | Nombre del tipo |
| `rango(inicio, fin, paso)` | Lista de enteros; `fin` exclusivo |
| `copiar(x)` | Copia superficial |
| `afirmar(cond, msg)` | Lanza si es falsa |
| `error(valor)` | Lanza un error |

## Métodos esenciales

### Listas

`agregar`, `insertar`, `eliminar`, `pop`, `limpiar`, `longitud`, `vacia`,
`unir`, `sublista`, `invertir`, `ordenar`, `contiene`, `indice_de`, `mapear`,
`filtrar`, `reducir`, `para_cada`, `buscar`, `unir_texto`, `suma`, `promedio`,
`copiar`.

### Diccionarios

`obtener`, `set`, `incrementar`, `existe`, `eliminar`, `longitud`, `llaves`,
`valores`, `items`, `unir`, `limpiar`, `copiar`.

### Texto

`longitud`, `mayusculas`, `minusculas`, `contiene`, `empieza_con`,
`termina_con`, `dividir`, `separar_lineas`, `reemplazar`, `recortar`,
`subtexto`, `indice_de`, `repetir`, `invertir`, `caracteres`, `a_entero`,
`a_decimal`, `a_logico`.

### Números

`absoluto`, `redondear`, `piso`, `techo`, `a_texto`.

## Módulos

| Módulo | Funciones principales |
| :--- | :--- |
| `json` | `parse`, `stringify`, `valido` |
| `bd` | `conectar`, `sqlite`, `abrir_solo_lectura`; conexión: `consultar`, `ejecutar`, `una`, `valor`, `lote`, `iniciar`, `confirmar`, `revertir`, `cerrar`, `abierta`, `solo_lectura`, `motor`, `ruta`, `version` |
| `mate` | `PI`, `E`, `raiz`, `potencia`, `redondear`, `piso`, `techo`, `absoluto`, `minimo`, `maximo`, `sumar`, `promedio`, `aleatorio`, `aleatorio_entero`, `logaritmo`, `seno`, `coseno`, `tangente` |
| `tiempo` | `ahora`, `millis`, `dormir`, `fecha`, `fecha_iso`, `iso`, `formatear`, `partes`, `año`, `mes`, `dia`, `hora`, `minuto`, `segundo`, `dia_semana`, `dia_del_año`, `semana`, `minutos_totales`, `horas_totales`, `dias_totales` |
| `sistema` | `env`, `cargar_env`, `argumentos`, `salir`, `directorio`, `plataforma` |
| `archivo` | `leer`, `escribir`, `anexar`, `existe`, `eliminar`, `listar`, `crear_dir`, `tamano`, `ruta_actual` |
| `cadenas` | `unir`, `mayusculas`, `minusculas`, `recortar`, `reemplazar`, `dividir`, `contiene`, `empieza_con`, `termina_con`, `repetir`, `rellenar_izquierda`, `rellenar_derecha`, `longitud` |
| `datos` | `leer_csv`, `guardar_csv`, `dimensiones`, `nombres`, `columna`, `filtrar`, `ordenar`, `mutar`, `agrupar_resumir`, `ver`, `a_json`, `desde_json`, `unir`, `seleccionar`, `eliminar_columna` |
| `concurrencia` | `tarea`, `paralelo`, `esperar_todas`, `contador`, `incrementar`, `valor`, `reiniciar`, `dormir`, `hilos` |
| `http` | `servidor`, `servidor_tls`, `peticion`, `codificar_url`, `decodificar_url`, `estado` |

## Formas de importar

```vaca
importar json
importar mate como mat
importar "util.vaca" como util
importar util
```

## Chuleta

```vaca
# Variables y constantes
contador: Entero = 0
constante LIMITE = 10

# Condicional
si contador > 0 {
    imprimir("positivo")
} sino {
    imprimir("cero o negativo")
}

# Bucles
para i = 1 hasta 3 { imprimir(i) }
mientras contador < LIMITE { contador += 1 }

# Funciones
fn sumar(a, b = 0) { retornar a + b }

# Listas y diccionarios
frutas = ["pera", "uva"]
frutas.agregar("higo")
datos = {"nombre": "Ada"}
datos.set("edad", 36)

# Clases
clase Perro {
    fn iniciar(nombre) { este.nombre = nombre }
    fn hablar() { retornar a"{este.nombre} dice guau" }
}

# Tareas
t = async fn() { retornar 42 }
imprimir(esperar t())

# Servidor
importar http
http.servidor(8080, fn(peticion) {
    retornar {"cuerpo": {"ok": verdadero}}
})
```

## Errores frecuentes

| Error | Solución |
| :--- | :--- |
| `variables declaradas pero nunca usadas` | Usa la variable, elimínala o usa `_` |
| `las constantes ... deben escribirse en MAYÚSCULAS` | Renombra la constante: `constante LIMITE` |
| `la variable 'x' no está definida` | Revisa el nombre o el ámbito |
| `índice N fuera de rango` | Comprueba la longitud antes de acceder |
| `la función 'f' espera N argumento(s)` | Ajusta la llamada o los valores por defecto |
| `división entre cero` | Valida el divisor |
| `desbordamiento de pila` | Añade un caso base |
| `'romper' fuera de un bucle` | Mueve `romper` dentro del bucle |
| `invalid peer certificate` | Revisa la cadena, el nombre del host o `VACA_CA` |
| `el constructor 'iniciar' no puede ser async` | Quita `async` de `iniciar` |

## Limitaciones

- **VM parcial con respaldo de árbol**: las funciones con `intentar`/`atrapar`, cierres creados en su interior o `importar` se interpretan sobre el árbol.
- **Tipado dinámico**: las anotaciones son informativas.
- **Sin recolector de basura**: los ciclos de `Arc` no se liberan.
- **Sin `super`**: no se puede llamar explícitamente al método del padre.
- **Sin ACME**: los certificados se renuevan fuera de Vaca y hay que reiniciar el proceso.
- **Concurrencia con hilos**: 8 MiB de pila por tarea/conexión, sin event loop.
- **Sin canales** entre tareas.
- **No esperes una tarea desde dentro de sí misma**: se bloquea.
- **Sin formateador, LSP ni `vaca test`**.
- **Rutas de módulos**: las funciones importadas resuelven rutas contra el
  archivo principal después del import.
- **Errores de ejecución sin línea**: solo léxicos y sintácticos indican
  posición.
