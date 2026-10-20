---
layout: ../../layouts/Docs.astro
title: "HTTP y concurrencia"
description: "Servidor y cliente HTTP, tareas async, hilos, contadores atómicos, límites de conexión, despliegue y laboratorio de estrés."
---

# HTTP y concurrencia

Vaca incluye un servidor HTTP/1.1 concurrente, un cliente HTTP y un modelo de
tareas basado en **hilos del sistema**. No hay `tokio` ni dependencias
externas: el runtime es el binario.

## Servidor HTTP

```vaca
importar http
importar json

fn manejador(peticion) {
    si peticion.metodo == "GET" y peticion.ruta == "/salud" {
        retornar {"cuerpo": {"estado": "ok"}}
    }

    si peticion.metodo == "POST" y peticion.ruta == "/eco" {
        datos = json.parse(peticion.cuerpo)
        retornar {"estado": 201, "cuerpo": {"recibido": datos}}
    }

    retornar {"estado": 404, "cuerpo": {"error": "no encontrada"}}
}

http.servidor(8080, manejador)
```

### Firma

```vaca
http.servidor(puerto, manejador)
http.servidor(host, puerto, manejador)
http.servidor(puerto, manejador, limite)
http.servidor(host, puerto, manejador, limite)
```

- `host` por defecto es `"0.0.0.0"`.
- `limite` es el número máximo de conexiones simultáneas. `0` o un valor
  negativo significa ilimitado.
- Al superar el límite se responde `503 Service Unavailable` con
  `Retry-After: 1`.
- El manejador puede ser una función normal o `async fn`; si es async, el
  servidor espera la tarea antes de responder.

### Petición

El manejador recibe un diccionario con estos campos:

| Campo | Tipo | Descripción |
| :--- | :--- | :--- |
| `metodo` | Texto | `GET`, `POST`, `PUT`, `DELETE`, etc. |
| `ruta` | Texto | Ruta decodificada, sin query |
| `consulta` | Diccionario | Parámetros de la query |
| `cabeceras` | Diccionario | Cabeceras con claves en minúsculas |
| `cuerpo` | Texto | Cuerpo completo |
| `version` | Texto | Versión HTTP |
| `ip` | Texto | IP del cliente |

```vaca
nombre = peticion.consulta.get("nombre", "invitado")
autorizacion = peticion.cabeceras.get("authorization", "")
```

### Respuesta

El manejador puede devolver:

- Un **texto**: se envía como `200 text/plain`.
- `nulo`: `204 No Content`.
- Una **lista** o **diccionario**: se serializa a JSON automáticamente.
- Un **diccionario de respuesta** con campos opcionales:

| Campo | Tipo | Descripción |
| :--- | :--- | :--- |
| `estado` | Entero | Código HTTP; por defecto `200` |
| `cuerpo` | Cualquiera | Cuerpo de la respuesta |
| `tipo` | Texto | `Content-Type` |
| `cabeceras` | Diccionario | Cabeceras adicionales |

```vaca
retornar {
    "estado": 201,
    "cuerpo": {"id": 42, "creado": verdadero},
    "tipo": "application/json; charset=utf-8",
    "cabeceras": {"X-Request-Id": "abc-123"}
}
```

> Si devuelves un diccionario de datos directamente, se interpreta como
> **sobre de respuesta** (busca `estado` y `cuerpo`). Para devolver datos
> planos, envuélvelos: `{"cuerpo": {"ok": verdadero}}`.

### API completa

```vaca
importar http
importar json
importar concurrencia

productos = [
    {"id": 1, "nombre": "Teclado", "precio": 30},
    {"id": 2, "nombre": "Monitor", "precio": 180}
]
siguiente_id = concurrencia.contador(2)
peticiones = concurrencia.contador(0)

fn responder(estado, cuerpo) {
    retornar {"estado": estado, "cuerpo": cuerpo}
}

fn manejador(peticion) {
    total = peticiones.incrementar()

    si peticion.metodo == "GET" y peticion.ruta == "/salud" {
        retornar responder(200, {
            "estado": "ok",
            "peticiones": total,
            "productos": productos.longitud()
        })
    }

    si peticion.metodo == "GET" y peticion.ruta == "/productos" {
        nombre = peticion.consulta.get("nombre", "").recortar()
        si nombre == "" {
            retornar responder(200, {"productos": productos})
        }
        filtrados = productos.filtrar(fn(p) {
            retornar p.nombre.minusculas().contiene(nombre.minusculas())
        })
        retornar responder(200, {"productos": filtrados})
    }

    si peticion.metodo == "POST" y peticion.ruta == "/productos" {
        intentar {
            datos = json.parse(peticion.cuerpo)
            si no datos.existe("nombre") {
                retornar responder(400, {"error": "falta nombre"})
            }
            producto = {
                "id": siguiente_id.incrementar(),
                "nombre": datos.get("nombre"),
                "precio": decimal(datos.get("precio", 0))
            }
            productos.agregar(producto)
            retornar responder(201, {"producto": producto})
        } atrapar error {
            retornar responder(400, {"error": a"JSON inválido: {error}"})
        }
    }

    retornar responder(404, {"error": "ruta no encontrada"})
}

http.servidor("0.0.0.0", 8080, manejador, 1000)
```

## Concurrencia

### Tareas `async`

Una función `async` se ejecuta en un hilo y devuelve una tarea. `esperar`
bloquea hasta que termina:

```vaca
importar tiempo

async fn consultar(nombre, ms) {
    tiempo.dormir(ms)
    retornar a"{nombre} lista"
}

a = consultar("A", 200)
b = consultar("B", 200)

resultado_a = esperar a
resultado_b = esperar b
```

- Dos tareas lanzadas seguidas se ejecutan a la vez.
- Esperar dos veces la misma tarea devuelve el resultado cacheado.
- Un error lanzado dentro de la tarea se propaga al `esperar`.
- También existen funciones anónimas async: `t = async fn() { ... }`.
- Un método de instancia puede ser async; el constructor `iniciar` no.

### Módulo `concurrencia`

```vaca
importar concurrencia

t1 = concurrencia.tarea(procesar, "a")
t2 = concurrencia.tarea(procesar, "b")
resultados = concurrencia.esperar_todas([t1, t2])

# O directamente:
resultados = concurrencia.paralelo([
    fn() { retornar procesar("a") },
    fn() { retornar procesar("b") }
])
```

| Función | Descripción |
| :--- | :--- |
| `tarea(funcion, ...args)` | Lanza una función en un hilo |
| `paralelo([funciones])` | Lanza todas y devuelve resultados |
| `esperar_todas(lista)` | Espera todas las tareas |
| `contador(inicial = 0)` | Contador atómico |
| `hilos()` | Hilos de CPU disponibles |

### Estado compartido

Los contadores atómicos evitan perder incrementos:

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
para _ = 1 hasta 8 {
    tareas.agregar(concurrencia.tarea(trabajar))
}
concurrencia.esperar_todas(tareas)
imprimir(contador.valor())    # 8000
```

Los diccionarios también ofrecen una operación atómica:

```vaca
visitas = {}
visitas.incrementar("total")       # devuelve el nuevo valor
visitas.incrementar("total", 10)
```

> Las lecturas y escrituras individuales de listas y diccionarios están
> protegidas por un mutex, pero una secuencia de varias operaciones no es
> atómica. Para contadores compartidos usa `concurrencia.contador` o
> `diccionario.incrementar`.

### Límites del modelo

- Cada tarea y cada conexión HTTP crea un hilo con **8 MiB de pila virtual**.
- No hay event loop: miles de tareas simultáneas consumen memoria.
- No hay canales entre tareas; comparte contadores atómicos o diccionarios.
- No esperes una tarea desde dentro de sí misma: se bloquearía para siempre.
- No hay recolector de basura; los ciclos de `Arc` no se liberan.

## Cliente HTTP

```vaca
importar http

respuesta = http.peticion("http://127.0.0.1:8080/salud")
imprimir(respuesta.estado)
imprimir(respuesta.cabeceras)
imprimir(respuesta.cuerpo)

respuesta = http.peticion(
    "http://api.local/productos",
    "POST",
    "{\"nombre\":\"Cable\"}",
    {"Content-Type": "application/json"}
)
```

| Campo de respuesta | Tipo |
| :--- | :--- |
| `estado` | Entero |
| `razon` | Texto |
| `cabeceras` | Diccionario |
| `cuerpo` | Texto |

Solo se admite `http://`. Para HTTPS usa un proxy inverso con TLS.

## Laboratorio de estrés

El ejemplo `ejemplos/servidor_concurrente.vaca` está pensado para pruebas de
carga:

```bash
cargo build --release
./target/release/vaca ejemplos/servidor_concurrente.vaca
python3 scripts/probar_concurrencia.py
```

Rutas:

| Ruta | Descripción |
| :--- | :--- |
| `GET /salud` | Estado, peticiones y hilos |
| `GET /metricas` | Peticiones, lentas y errores |
| `GET /lento?ms=200` | Duerme `ms` y responde |
| `POST /eco` | Devuelve el JSON recibido |

El script lanza 50 peticiones de 200 ms a la vez y comprueba que terminan en
~0,2 s, que los contadores atómicos son exactos y que los identificadores de
las peticiones concurrentes son únicos.

### Medir con Vaca

```vaca
importar tiempo
importar concurrencia

fn trabajo(id) {
    tiempo.dormir(200)
    retornar id
}

inicio = tiempo.millis()
tareas = []
para i = 1 hasta 50 {
    tareas.agregar(concurrencia.tarea(trabajo, i))
}
resultados = concurrencia.esperar_todas(tareas)
total = tiempo.millis() - inicio

imprimir(a"{resultados.longitud()} tareas en {total} ms")
```

### Apagar el servidor

`http.servidor` no expone una función de parada, pero puedes terminarlo desde
un manejador:

```vaca
importar sistema

si peticion.ruta == "/detener" {
    sistema.salir(0)
}
```

La petición que dispara la salida no recibe respuesta: el proceso termina.

## Despliegue

### systemd

```ini
[Unit]
Description=API Vaca
After=network.target

[Service]
ExecStart=/opt/vaca/api.vaca
WorkingDirectory=/opt/vaca
Restart=on-failure
User=vaca

[Install]
WantedBy=multi-user.target
```

### Docker

```bash
docker build -f packaging/Dockerfile -t vaca:0.1.0 .
docker run --rm -p 8080:8080 vaca:0.1.0
```

### TLS

Vaca no termina TLS. Usa Nginx, Caddy o un balanceador delante:

```nginx
server {
    listen 443 ssl;
    server_name api.ejemplo.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Seguridad para un laboratorio

Vaca **no es un sandbox**. Un programa tiene acceso al sistema de archivos, al
entorno y a la red según el usuario que lo ejecute.

- Ejecuta el servidor en un contenedor o usuario sin privilegios.
- Limita las conexiones con el cuarto argumento de `http.servidor`.
- No confíes en `Content-Length`: el servidor lee el cuerpo indicado sin un
  límite propio. Pon un proxy con límite de tamaño.
- `json.parse` rechaza JSON inválido y anidamiento excesivo, pero no valida
  esquemas; valida los campos a mano.
- No hay autenticación ni rate limiting nativos. Impleméntalos en el
  manejador con diccionarios y contadores atómicos.
- Las rutas del módulo `archivo` no están confinadas a un directorio.
- Para pruebas de carga, usa el límite de conexiones y monitoriza memoria: cada
  conexión reserva 8 MiB de pila virtual.
