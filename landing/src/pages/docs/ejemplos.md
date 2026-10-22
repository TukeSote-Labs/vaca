---
layout: ../../layouts/Docs.astro
title: "Ejemplos de uso"
description: "Programas completos en Vaca con su salida: algoritmos, POO, datos, HTTP, concurrencia y patrones."
---

# Ejemplos de uso

Programas completos y ejecutables. Todos usan la sintaxis real de Vaca y
muestran la salida esperada. Guarda cualquiera en un archivo `.vaca` y ejecútalo
con `vaca archivo.vaca`.

## Hola, mundo

```vaca
nombre = "Mundo"
imprimir(a"Hola, {nombre}!")
```

```text
Hola, Mundo!
```

## FizzBuzz

```vaca
para i = 1 hasta 15 {
    si i % 15 == 0 {
        imprimir("FizzBuzz")
    } sino si i % 3 == 0 {
        imprimir("Fizz")
    } sino si i % 5 == 0 {
        imprimir("Buzz")
    } sino {
        imprimir(i)
    }
}
```

```text
1
2
Fizz
4
Buzz
Fizz
7
8
Fizz
Buzz
11
Fizz
13
14
FizzBuzz
```

## Factorial y Fibonacci

```vaca
fn factorial(n) {
    si n <= 1 {
        retornar 1
    }
    retornar n * factorial(n - 1)
}

fn fib(n) {
    si n < 2 {
        retornar n
    }
    retornar fib(n - 1) + fib(n - 2)
}

imprimir(factorial(6))   # 720
imprimir(fib(10))        # 55
```

## Two Sum con diccionario

```vaca
fn dos_suma(numeros, objetivo) {
    vistos = {}

    para i = 0 hasta numeros.longitud() - 1 {
        faltante = objetivo - numeros[i]
        si vistos.existe(faltante) {
            retornar [vistos.obtener(faltante), i]
        }
        vistos.set(a"{numeros[i]}", i)
    }

    retornar nulo
}

imprimir(dos_suma([2, 7, 11, 15], 9))
```

```text
[0, 1]
```

## Búsqueda binaria

```vaca
fn busqueda_binaria(lista, objetivo) {
    izquierda = 0
    derecha = lista.longitud() - 1

    mientras izquierda <= derecha {
        medio = (izquierda + derecha) / 2
        valor = lista[medio]

        si valor == objetivo {
            retornar medio
        }

        si valor < objetivo {
            izquierda = medio + 1
        } sino {
            derecha = medio - 1
        }
    }

    retornar -1
}

imprimir(busqueda_binaria([1, 3, 5, 7, 9], 7))
```

```text
3
```

## Pila con una lista

```vaca
pila = []
pila.agregar(1)
pila.agregar(2)
pila.agregar(3)

imprimir(pila.pop())   # 3
imprimir(pila.pop())   # 2
imprimir(pila)         # [1]
```

## Contar palabras

```vaca
texto = "vaca habla español vaca rust español"
conteo = {}

para palabra en texto.dividir(" ") {
    conteo.incrementar(palabra)
}

imprimir(conteo)
```

```text
{"español": 2, "habla": 1, "rust": 1, "vaca": 2}
```

## POO con herencia

```vaca
clase Empleado {
    fn iniciar(nombre, salario) {
        este.nombre = nombre
        este.salario = salario
    }

    fn resumen() {
        retornar a"{este.nombre}: {este.salario}"
    }
}

clase Gerente(Empleado) {
    fn iniciar(nombre, salario, equipo) {
        este.nombre = nombre
        este.salario = salario
        este.equipo = equipo
    }

    fn resumen() {
        retornar a"{este.nombre} dirige {este.equipo} personas"
    }
}

gerente = nuevo Gerente("Ada", 5000, 8)
imprimir(gerente.resumen())
```

```text
Ada dirige 8 personas
```

## Excepciones

```vaca
fn dividir(a, b) {
    si b == 0 {
        lanzar "división entre cero"
    }
    retornar a / b
}

intentar {
    imprimir(dividir(10, 0))
} atrapar error {
    imprimir(a"Error: {error}")
}

imprimir("sigo vivo")
```

```text
Error: división entre cero
sigo vivo
```

## JSON y archivos

```vaca
importar json
importar archivo

usuario = {
    "nombre": "Ada",
    "activo": verdadero,
    "roles": ["admin", "dev"]
}

texto = json.serializar(usuario, verdadero)
archivo.escribir("usuario.json", texto)

leido = json.convertir(archivo.leer("usuario.json"))
imprimir(leido.nombre)      # Ada
imprimir(leido.roles[1])    # dev
```

## DataFrames con CSV

Con `ventas.csv`:

```text
ciudad,monto
Madrid,100
Lima,50
Madrid,30
```

```vaca
importar datos

df = datos.leer_csv("ventas.csv")
imprimir(datos.dimensiones(df))

resumen = datos.agrupar_resumir(df, "ciudad", "monto", "suma")
imprimir(datos.columna(resumen, "ciudad"))
imprimir(datos.columna(resumen, "monto"))

datos.guardar_csv(resumen, "resumen.csv")
```

```text
[3, 2]
["Lima", "Madrid"]
[50, 130]
```

## Contador atómico entre hilos

```vaca
importar concurrencia

contador = concurrencia.contador(0)

fn trabajar() {
    para _ = 1 hasta 1000 {
        contador.incrementar()
    }
    retornar nulo
}

tareas = []
para _ = 1 hasta 4 {
    tareas.agregar(concurrencia.tarea(trabajar))
}

concurrencia.esperar_todas(tareas)
imprimir(contador.valor())
```

```text
4000
```

## Servidor HTTP

```vaca
importar http
importar json

fn manejador(peticion) {
    si peticion.metodo == "GET" y peticion.ruta == "/salud" {
        retornar {"cuerpo": {"estado": "ok"}}
    }

    si peticion.metodo == "POST" y peticion.ruta == "/eco" {
        retornar {"estado": 201, "cuerpo": json.convertir(peticion.cuerpo)}
    }

    retornar {"estado": 404, "cuerpo": {"error": "no encontrada"}}
}

http.servidor(8080, manejador)
```

```bash
curl http://localhost:8080/salud
curl -X POST -d '{"hola":"mundo"}' http://localhost:8080/eco
```

```text
{"estado":"ok"}
{"hola":"mundo"}
```

## Patrón Strategy

```vaca
clase PagoStripe {
    fn iniciar(api_key) {
        este.api_key = api_key
    }

    fn pagar(monto) {
        retornar a"Stripe cobró {monto}"
    }
}

clase PagoPaypal {
    fn iniciar(email) {
        este.email = email
    }

    fn pagar(monto) {
        retornar a"PayPal cobró {monto} a {este.email}"
    }
}

clase Procesador {
    fn iniciar(estrategia) {
        este.estrategia = estrategia
    }

    fn cobrar(monto) {
        retornar este.estrategia.pagar(monto)
    }
}

stripe = nuevo Procesador(nuevo PagoStripe("sk_test"))
paypal = nuevo Procesador(nuevo PagoPaypal("ada@ejemplo.com"))

imprimir(stripe.cobrar(100))
imprimir(paypal.cobrar(50))
```

```text
Stripe cobró 100
PayPal cobró 50 a ada@ejemplo.com
```

## Rate limiter por minuto

```vaca
importar tiempo

peticiones = {}
limite = 5

fn permitir(ip) {
    minuto = entero(tiempo.ahora() / 60)
    clave = a"{ip}:{minuto}"
    cuenta = peticiones.incrementar(clave)
    retornar cuenta <= limite
}

para _ = 1 hasta 6 {
    imprimir(permitir("127.0.0.1"))
}
```

```text
verdadero
verdadero
verdadero
verdadero
verdadero
falso
```

> El diccionario crece con cada IP y minuto. En un servidor de larga duración,
> elimina las claves antiguas periódicamente.

## Más ejemplos en el repositorio

El directorio `ejemplos/` incluye programas más largos:

| Archivo | Tema |
| :--- | :--- |
| `algoritmos.vaca` | Two Sum, búsqueda binaria, Fibonacci, paréntesis y merge sort |
| `grafos.vaca` | BFS, DFS, caminos y orden topológico |
| `colecciones.vaca` | `mapear`, `filtrar`, `reducir`, orden y búsqueda |
| `closures.vaca` | Cierres, callbacks y memoización |
| `poo.vaca` / `poo_avanzada.vaca` | Herencia, contratos, observer y strategy |
| `patrones.vaca` | Singleton, middleware, cola FIFO, caché y rate limiter |
| `datos.vaca` | DataFrames con `ventas.csv` |
| `config.vaca` + `.env` | Configuración con `sistema.cargar_env` y comentarios al final |
| `async.vaca` y `concurrencia.vaca` | Tareas, hilos y contadores |
| `servidor.vaca` / `servidor_api.vaca` | Servidores HTTP |
| `servidor_concurrente.vaca` | Servidor de estrés con métricas |
