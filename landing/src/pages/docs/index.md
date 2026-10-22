---
layout: ../../layouts/Docs.astro
title: "Introducción"
description: "Qué es Vaca, cómo instalarlo y cómo escribir tu primer programa. Documentación completa del lenguaje."
---

# Documentación de Vaca

Vaca es un lenguaje de programación **en español, con llaves y pensado para
servidores**. Su motor está escrito en Rust: usa la biblioteca estándar y
`rustls` para TLS, sin frameworks: el
binario, la biblioteca estándar y el servidor HTTP viajan juntos.

Esta documentación cubre el lenguaje, la biblioteca estándar, el modelo de
concurrencia, los ejemplos de uso, las herramientas y una referencia rápida.

## Ficha técnica

| Aspecto | Detalle |
| :--- | :--- |
| Sintaxis | Español, llaves `{ }`, `;` opcional |
| Tipado | Dinámico con anotaciones opcionales informativas |
| Paradigma | Imperativo, funcional y orientado a objetos |
| Motor | Compilador a bytecode y VM de pila en Rust |
| Concurrencia | Hilos del sistema, `async`/`esperar`, contadores atómicos |
| Biblioteca | JSON, matemáticas, tiempo, sistema, archivos, texto, CSV/DataFrames, HTTP y concurrencia |
| Dependencias | Rust estándar + `rustls` (TLS) |
| Licencia | MIT |

## Instalación

Necesitas **Rust estable** para compilar el motor. Después del build no hace
falta Node, npm ni ningún runtime adicional.

```bash
git clone https://github.com/ulf-frostypaw/vaca.git
cd vaca
cargo build --release
```

El binario queda en `target/release/vaca`. También puedes instalarlo como
comando global:

```bash
cargo install --path .
```

Comprueba la instalación:

```bash
vaca --version
```

## Tu primer programa

Crea `hola.vaca`:

```vaca
nombre = "Mundo"
imprimir(a"Hola, {nombre}!")
```

Ejecútalo:

```bash
vaca hola.vaca
```

Salida:

```text
Hola, Mundo!
```

Un programa algo más completo:

```vaca
# Variables y anotaciones opcionales
contador: Entero = 0

para i = 1 hasta 5 {
    contador = contador + i
}

imprimir(a"Suma del 1 al 5: {contador}")

# Listas y diccionarios
frutas = ["pera", "uva", "higo"]
frutas.agregar("melón")
frutas.ordenar()

persona = {"nombre": "Ada", "lenguaje": "Vaca"}
imprimir(frutas)
imprimir(persona.obtener("nombre"))
```

Salida:

```text
Suma del 1 al 5: 15
["higo", "melón", "pera", "uva"]
Ada
```

## Cómo leer esta documentación

- **[El lenguaje](/docs/lenguaje/)**: variables, tipos, operadores, control de
  flujo, funciones, colecciones, POO, errores e importaciones.
- **[Módulos nativos](/docs/modulos/)**: la biblioteca estándar completa, con
  firmas y ejemplos.
- **[HTTP y concurrencia](/docs/servidor/)**: servidor, cliente, hilos,
  límites de conexión y despliegue.
- **[Ejemplos de uso](/docs/ejemplos/)**: programas completos con su salida.
- **[Paquetes](/docs/paquetes/)**: manifiesto, versiones, git, lockfile y
  reemplazos locales.
- **[Herramientas](/docs/herramientas/)**: CLI, REPL, editor, empaquetado,
  pruebas y diagnóstico de errores.
- **[Referencia rápida](/docs/referencia/)**: palabras clave, operadores,
  builtins, métodos y tabla de errores.

## Convenciones

- Los bloques `vaca` contienen código del lenguaje.
- Los bloques `bash` contienen comandos de shell.
- Los bloques `text` contienen la salida esperada.
- `#`, `//` y `/* ... */` son comentarios en Vaca.
- Las constantes se declaran con `constante` y su nombre debe ir en
  **MAYÚSCULAS**.
- El punto y coma `;` nunca es obligatorio.

> **Nota:** Vaca comprueba antes de ejecutar que no haya variables locales
> declaradas y sin usar. Si un ejemplo no arranca, revisa esa regla en
> [Herramientas](/docs/herramientas/#variables-no-usadas).

## Estado del proyecto

Vaca está en la versión **0.1.0**. El lenguaje, la biblioteca estándar, la
concurrencia real y el servidor HTTP están implementados. Las funciones se
compilan a bytecode y se ejecutan en una VM de pila, con el intérprete de árbol
como respaldo para `intentar`, cierres anidados e `importar`. Todavía no hay
comprobación estática de tipos, recolector de basura, formateador ni gestor de
paquetes.

Consulta la [referencia rápida](/docs/referencia/#limitaciones) para ver la
lista completa de límites antes de llevar Vaca a producción.
