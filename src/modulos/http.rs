//! Módulo `http`: servidor y cliente HTTP/1.1 sobre TCP.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::{crear_modulo, json};
use crate::value::{MapaValores, Value};

pub fn modulo() -> Value {
    crear_modulo(
        "http",
        vec![
            ("servidor", nativa("http.servidor", 2, 4, servidor)),
            ("peticion", nativa("http.peticion", 1, 4, peticion)),
            (
                "codificar_url",
                nativa("http.codificar_url", 1, 1, codificar_url),
            ),
            (
                "decodificar_url",
                nativa("http.decodificar_url", 1, 1, decodificar_url),
            ),
            ("estado", nativa("http.estado", 1, 1, estado)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

// ----------------------------------------------------------------------
// Servidor
// ----------------------------------------------------------------------

/// Decrementa el contador de conexiones activas incluso si el hilo de
/// atención termina de forma anormal (panic).
struct GuardiaActiva(Arc<AtomicUsize>);

impl Drop for GuardiaActiva {
    fn drop(&mut self) {
        self.0.fetch_sub(1, AtomicOrdering::SeqCst);
    }
}

struct Solicitud {
    metodo: String,
    ruta: String,
    consulta: HashMap<String, String>,
    cabeceras: HashMap<String, String>,
    cuerpo: String,
    version: String,
    ip: String,
}

impl Solicitud {
    fn a_valor(&self) -> Value {
        let consulta = Value::diccionario(
            self.consulta
                .iter()
                .map(|(k, v)| (k.clone(), Value::Texto(v.clone())))
                .collect(),
        );
        let cabeceras = Value::diccionario(
            self.cabeceras
                .iter()
                .map(|(k, v)| (k.clone(), Value::Texto(v.clone())))
                .collect(),
        );
        Value::diccionario(vec![
            ("metodo".to_string(), Value::Texto(self.metodo.clone())),
            ("ruta".to_string(), Value::Texto(self.ruta.clone())),
            ("consulta".to_string(), consulta),
            ("cabeceras".to_string(), cabeceras),
            ("cuerpo".to_string(), Value::Texto(self.cuerpo.clone())),
            ("version".to_string(), Value::Texto(self.version.clone())),
            ("ip".to_string(), Value::Texto(self.ip.clone())),
        ])
    }
}

fn servidor(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let (host, puerto, manejador, limite) = match args.len() {
        2 => (
            "0.0.0.0".to_string(),
            puerto_valor(&args[0])?,
            args[1].clone(),
            None,
        ),
        3 => {
            // `http.servidor(puerto, manejador, limite)` o
            // `http.servidor(host, puerto, manejador)`.
            if args[0].a_entero_exacto().is_some() {
                (
                    "0.0.0.0".to_string(),
                    puerto_valor(&args[0])?,
                    args[1].clone(),
                    limite_valor(&args[2])?,
                )
            } else {
                (
                    args[0].a_texto(),
                    puerto_valor(&args[1])?,
                    args[2].clone(),
                    None,
                )
            }
        }
        4 => (
            args[0].a_texto(),
            puerto_valor(&args[1])?,
            args[2].clone(),
            limite_valor(&args[3])?,
        ),
        _ => unreachable!(),
    };
    if !manejador.es_llamable() {
        return Err(Error::ejecucion(
            "http.servidor espera una función manejadora como último argumento",
        ));
    }
    let direccion = format!("{}:{}", host, puerto);
    let listener = TcpListener::bind(&direccion).map_err(|e| {
        Error::ejecucion(format!(
            "no se pudo iniciar el servidor en '{}': {}",
            direccion, e
        ))
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|e| Error::ejecucion(format!("no se pudo configurar el servidor: {}", e)))?;
    let local = listener
        .local_addr()
        .map(|a| a.to_string())
        .unwrap_or(direccion);
    let hilos = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    match limite {
        Some(limite) => interprete.escribir_linea(&format!(
            "Servidor Vaca escuchando en http://{} ({} hilos de CPU, límite {} conexiones)",
            local, hilos, limite
        )),
        None => interprete.escribir_linea(&format!(
            "Servidor Vaca escuchando en http://{} ({} hilos de CPU, conexiones ilimitadas)",
            local, hilos
        )),
    }

    // Código de salida compartido: -1 significa «seguir atendiendo». Cuando
    // cualquier manejador llama a `sistema.salir`, el bucle principal lo ve y
    // termina el servidor de forma ordenada.
    let salir = Arc::new(AtomicI32::new(-1));
    let activas = Arc::new(AtomicUsize::new(0));

    loop {
        let codigo = salir.load(AtomicOrdering::SeqCst);
        if codigo >= 0 {
            return Err(Error::Salir(codigo));
        }

        let (mut stream, remoto) = match listener.accept() {
            Ok(par) => par,
            Err(ref error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                // Sin conexiones pendientes: dormimos un instante para no
                // consumir CPU mientras esperamos la siguiente.
                std::thread::sleep(Duration::from_millis(2));
                continue;
            }
            Err(error) => {
                interprete.escribir_linea(&format!("Error de conexión: {}", error));
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
        };

        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));
        let _ = stream.set_nodelay(true);

        if let Some(limite) = limite {
            if activas.load(AtomicOrdering::SeqCst) >= limite {
                // Descarta la petición ya enviada antes de cerrar. Sin esto,
                // el socket se cierra con datos sin leer y el cliente puede
                // recibir un RST en lugar del 503.
                descartar_entrada(&mut stream);
                let _ = escribir_respuesta(
                    &mut stream,
                    503,
                    "Servidor ocupado, intenta de nuevo\n",
                    "text/plain; charset=utf-8",
                    &[("Retry-After".to_string(), "1".to_string())],
                );
                let _ = stream.shutdown(std::net::Shutdown::Read);
                continue;
            }
        }

        let mut trabajador = interprete.crear_tarea();
        let manejador = manejador.clone();
        let salir = Arc::clone(&salir);
        let guardia = GuardiaActiva(Arc::clone(&activas));
        let ip = remoto.ip().to_string();
        activas.fetch_add(1, AtomicOrdering::SeqCst);

        let inicio = std::thread::Builder::new()
            .name(format!("vaca-http-{}", ip))
            .stack_size(8 * 1024 * 1024)
            .spawn(move || {
                // El guardián libera la plaza de conexión al salir del hilo,
                // también si el intérprete entra en pánico.
                let _guardia = guardia;
                let resultado = atender_conexion(&mut trabajador, &mut stream, &manejador);
                match resultado {
                    Ok(()) => {}
                    Err(Error::Salir(codigo)) => {
                        salir.store(codigo.max(0), AtomicOrdering::SeqCst);
                    }
                    Err(error) => {
                        trabajador.escribir_linea(&format!("Error atendiendo conexión: {}", error));
                    }
                }
            });

        if let Err(error) = inicio {
            // Si el hilo no llegó a crearse, el cierre de la clausura libera
            // la plaza mediante el guardián.
            interprete.escribir_linea(&format!("No se pudo atender la conexión: {}", error));
        }
    }
}

/// Lee y descarta lo que el cliente ya haya enviado, sin bloquearse.
fn descartar_entrada(stream: &mut TcpStream) {
    if stream.set_nonblocking(true).is_err() {
        return;
    }
    let mut buffer = [0u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    let _ = stream.set_nonblocking(false);
}

fn limite_valor(valor: &Value) -> Result<Option<usize>, Error> {
    let n = valor
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("el límite de conexiones debe ser un entero"))?;
    if n <= 0 {
        Ok(None)
    } else {
        Ok(Some(n as usize))
    }
}

fn atender_conexion(
    interprete: &mut Interprete,
    stream: &mut TcpStream,
    manejador: &Value,
) -> Result<(), Error> {
    let solicitud = match leer_solicitud(stream) {
        Ok(s) => s,
        Err(mensaje) => {
            let _ = escribir_respuesta(stream, 400, &mensaje, "text/plain; charset=utf-8", &[]);
            return Ok(());
        }
    };

    let resultado = interprete
        .llamar(manejador.clone(), vec![solicitud.a_valor()])
        .and_then(|valor| match valor {
            // Un manejador `async` devuelve una tarea: la esperamos antes de
            // construir la respuesta HTTP.
            Value::Tarea(tarea) => tarea.esperar(),
            otro => Ok(otro),
        });

    let respuesta = match resultado {
        Ok(valor) => valor,
        Err(Error::Salir(codigo)) => return Err(Error::Salir(codigo)),
        Err(Error::Lanzado(valor)) => {
            let (estado, cuerpo, tipo, extra) = construir_respuesta(Value::diccionario(vec![
                ("estado".to_string(), Value::Entero(500)),
                (
                    "cuerpo".to_string(),
                    Value::Texto(format!("Error interno: {}", valor.a_texto())),
                ),
            ]));
            escribir_respuesta(stream, estado, &cuerpo, &tipo, &extra).map_err(Error::ejecucion)?;
            return Ok(());
        }
        Err(error) => {
            let (estado, cuerpo, tipo, extra) = construir_respuesta(Value::diccionario(vec![
                ("estado".to_string(), Value::Entero(500)),
                (
                    "cuerpo".to_string(),
                    Value::Texto(format!("Error interno: {}", error)),
                ),
            ]));
            escribir_respuesta(stream, estado, &cuerpo, &tipo, &extra).map_err(Error::ejecucion)?;
            return Ok(());
        }
    };

    let (estado, cuerpo, tipo, extra) = construir_respuesta(respuesta);
    escribir_respuesta(stream, estado, &cuerpo, &tipo, &extra).map_err(Error::ejecucion)?;
    Ok(())
}

fn construir_respuesta(valor: Value) -> (u16, String, String, Vec<(String, String)>) {
    match valor {
        Value::Diccionario(dict) => {
            let dict = dict.lock().unwrap().clone();
            let estado = dict
                .get("estado")
                .and_then(|v| v.a_entero_exacto())
                .unwrap_or(200)
                .clamp(100, 599) as u16;
            let cuerpo_valor = dict
                .get("cuerpo")
                .or_else(|| dict.get("body"))
                .cloned()
                .unwrap_or(Value::Nulo);
            let es_json = matches!(cuerpo_valor, Value::Lista(_) | Value::Diccionario(_));
            let cuerpo = match &cuerpo_valor {
                Value::Texto(t) => t.clone(),
                Value::Nulo => String::new(),
                Value::Lista(_) | Value::Diccionario(_) => {
                    json::stringify(&cuerpo_valor, false).unwrap_or_else(|_| cuerpo_valor.a_texto())
                }
                otro => otro.a_texto(),
            };
            let tipo = dict
                .get("tipo")
                .or_else(|| dict.get("content-type"))
                .map(|v| v.a_texto())
                .unwrap_or_else(|| {
                    if es_json {
                        "application/json; charset=utf-8".to_string()
                    } else {
                        "text/plain; charset=utf-8".to_string()
                    }
                });
            let mut extra = Vec::new();
            if let Some(Value::Diccionario(cabeceras)) = dict.get("cabeceras") {
                for (clave, valor) in cabeceras.lock().unwrap().iter() {
                    let clave_min = clave.to_lowercase();
                    if clave_min != "content-length"
                        && clave_min != "content-type"
                        && clave_min != "connection"
                    {
                        extra.push((clave.clone(), valor.a_texto()));
                    }
                }
            }
            (estado, cuerpo, tipo, extra)
        }
        Value::Texto(texto) => (
            200,
            texto,
            "text/plain; charset=utf-8".to_string(),
            Vec::new(),
        ),
        Value::Nulo => (
            204,
            String::new(),
            "text/plain; charset=utf-8".to_string(),
            Vec::new(),
        ),
        otro => (
            200,
            otro.a_texto(),
            "text/plain; charset=utf-8".to_string(),
            Vec::new(),
        ),
    }
}

fn escribir_respuesta(
    stream: &mut TcpStream,
    estado: u16,
    cuerpo: &str,
    tipo: &str,
    extra: &[(String, String)],
) -> Result<(), String> {
    let mut respuesta = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        estado,
        razon(estado),
        tipo,
        cuerpo.len()
    );
    for (clave, valor) in extra {
        respuesta.push_str(&format!("{}: {}\r\n", clave, valor));
    }
    respuesta.push_str("\r\n");
    respuesta.push_str(cuerpo);
    stream
        .write_all(respuesta.as_bytes())
        .map_err(|e| format!("no se pudo escribir la respuesta: {}", e))?;
    let _ = stream.flush();
    Ok(())
}

fn leer_solicitud(stream: &mut TcpStream) -> Result<Solicitud, String> {
    let mut datos: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 8192];
    let fin_cabeceras;
    loop {
        let leidos = stream
            .read(&mut buffer)
            .map_err(|e| format!("no se pudo leer la solicitud: {}", e))?;
        if leidos == 0 {
            return Err("conexión cerrada antes de completar la solicitud".to_string());
        }
        datos.extend_from_slice(&buffer[..leidos]);
        if let Some(pos) = buscar(&datos, b"\r\n\r\n") {
            fin_cabeceras = pos + 4;
            break;
        }
        if datos.len() > 8 * 1024 * 1024 {
            return Err("solicitud demasiado grande".to_string());
        }
    }

    let cabecera_texto = String::from_utf8_lossy(&datos[..fin_cabeceras]).to_string();
    let mut lineas = cabecera_texto.split("\r\n");
    let primera = lineas
        .next()
        .ok_or_else(|| "solicitud HTTP vacía".to_string())?;
    let partes: Vec<&str> = primera.split_whitespace().collect();
    if partes.len() < 3 {
        return Err("línea de solicitud HTTP inválida".to_string());
    }
    let metodo = partes[0].to_uppercase();
    let objetivo = partes[1].to_string();
    let version = partes[2].to_string();

    let mut cabeceras = HashMap::new();
    for linea in lineas {
        if linea.is_empty() {
            break;
        }
        if let Some((clave, valor)) = linea.split_once(':') {
            cabeceras.insert(clave.trim().to_lowercase(), valor.trim().to_string());
        }
    }

    let longitud: usize = cabeceras
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let total = fin_cabeceras + longitud;
    while datos.len() < total {
        let leidos = stream
            .read(&mut buffer)
            .map_err(|e| format!("no se pudo leer el cuerpo: {}", e))?;
        if leidos == 0 {
            break;
        }
        datos.extend_from_slice(&buffer[..leidos]);
    }
    let cuerpo = String::from_utf8_lossy(&datos[fin_cabeceras..total.min(datos.len())]).to_string();

    let (ruta, consulta_texto) = match objetivo.split_once('?') {
        Some((r, c)) => (r.to_string(), c.to_string()),
        None => (objetivo, String::new()),
    };
    let ruta = decodificar(&ruta);
    let mut consulta = HashMap::new();
    for par in consulta_texto.split('&') {
        if par.is_empty() {
            continue;
        }
        match par.split_once('=') {
            Some((k, v)) => {
                consulta.insert(decodificar(k), decodificar(v));
            }
            None => {
                consulta.insert(decodificar(par), String::new());
            }
        }
    }

    let ip = stream
        .peer_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_default();

    Ok(Solicitud {
        metodo,
        ruta,
        consulta,
        cabeceras,
        cuerpo,
        version,
        ip,
    })
}

fn buscar(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|ventana| ventana == needle)
}

fn puerto_valor(valor: &Value) -> Result<u16, Error> {
    let puerto = valor
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("el puerto debe ser un entero"))?;
    if !(0..=65535).contains(&puerto) {
        return Err(Error::ejecucion(format!(
            "puerto inválido {} (rango 0-65535)",
            puerto
        )));
    }
    Ok(puerto as u16)
}

// ----------------------------------------------------------------------
// Cliente
// ----------------------------------------------------------------------

struct Url {
    host: String,
    puerto: u16,
    ruta: String,
}

fn parsear_url(url: &str) -> Result<Url, String> {
    let resto = if let Some(resto) = url.strip_prefix("http://") {
        resto
    } else if url.starts_with("https://") {
        return Err("https no está soportado sin TLS; usa http:// o un proxy inverso".to_string());
    } else {
        url
    };
    let (autoridad, ruta) = match resto.find('/') {
        Some(i) => (&resto[..i], &resto[i..]),
        None => (resto, "/"),
    };
    if autoridad.is_empty() {
        return Err("URL sin host".to_string());
    }
    let (host, puerto) = match autoridad.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| format!("puerto inválido '{}'", p))?,
        ),
        _ => (autoridad.to_string(), 80),
    };
    Ok(Url {
        host,
        puerto,
        ruta: ruta.to_string(),
    })
}

fn peticion(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let url_texto = args[0].a_texto();
    let metodo = args
        .get(1)
        .map(|v| v.a_texto().to_uppercase())
        .unwrap_or_else(|| "GET".to_string());
    let cuerpo = args.get(2).map(|v| v.a_texto()).unwrap_or_default();
    let cabeceras_extra = match args.get(3) {
        Some(Value::Diccionario(d)) => d.lock().unwrap().clone(),
        None => MapaValores::default(),
        Some(otro) => {
            return Err(Error::ejecucion(format!(
                "las cabeceras de http.peticion deben ser un diccionario, se recibió {}",
                otro.tipo_nombre()
            )))
        }
    };

    let url = parsear_url(&url_texto).map_err(Error::ejecucion)?;
    let direccion = format!("{}:{}", url.host, url.puerto);
    let mut direcciones = direccion
        .to_socket_addrs()
        .map_err(|e| Error::ejecucion(format!("no se pudo resolver '{}': {}", direccion, e)))?;
    let socket = direcciones
        .next()
        .ok_or_else(|| Error::ejecucion(format!("no se pudo resolver '{}'", direccion)))?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(15))
        .map_err(|e| Error::ejecucion(format!("no se pudo conectar con '{}': {}", direccion, e)))?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(30)));

    let mut solicitud = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Vaca/0.1\r\nAccept: */*\r\nConnection: close\r\n",
        metodo, url.ruta, url.host
    );
    for (clave, valor) in &cabeceras_extra {
        let clave_min = clave.to_lowercase();
        if clave_min != "host" && clave_min != "content-length" && clave_min != "connection" {
            solicitud.push_str(&format!("{}: {}\r\n", clave, valor.a_texto()));
        }
    }
    if !cuerpo.is_empty() || !matches!(metodo.as_str(), "GET" | "HEAD") {
        solicitud.push_str(&format!("Content-Length: {}\r\n", cuerpo.len()));
    }
    solicitud.push_str("\r\n");
    solicitud.push_str(&cuerpo);

    stream
        .write_all(solicitud.as_bytes())
        .map_err(|e| Error::ejecucion(format!("no se pudo enviar la petición: {}", e)))?;
    let _ = stream.flush();

    let mut respuesta = Vec::new();
    stream
        .read_to_end(&mut respuesta)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer la respuesta: {}", e)))?;

    let separador = buscar(&respuesta, b"\r\n\r\n")
        .ok_or_else(|| Error::ejecucion("respuesta HTTP inválida: sin cabeceras"))?;
    let cabecera_texto = String::from_utf8_lossy(&respuesta[..separador]).to_string();
    let mut lineas = cabecera_texto.split("\r\n");
    let primera = lineas
        .next()
        .ok_or_else(|| Error::ejecucion("respuesta HTTP vacía"))?;
    let partes: Vec<&str> = primera.split_whitespace().collect();
    if partes.len() < 2 {
        return Err(Error::ejecucion("línea de estado HTTP inválida"));
    }
    let estado: u16 = partes[1]
        .parse()
        .map_err(|_| Error::ejecucion("código de estado HTTP inválido"))?;
    let razon = partes.get(2).copied().unwrap_or("").to_string();

    let mut cabeceras = HashMap::new();
    for linea in lineas {
        if linea.is_empty() {
            break;
        }
        if let Some((clave, valor)) = linea.split_once(':') {
            cabeceras.insert(clave.trim().to_lowercase(), valor.trim().to_string());
        }
    }

    let mut cuerpo_bytes = respuesta[separador + 4..].to_vec();
    if cabeceras
        .get("transfer-encoding")
        .map(|v| v.to_lowercase().contains("chunked"))
        .unwrap_or(false)
    {
        cuerpo_bytes = deschunk(&cuerpo_bytes).map_err(Error::ejecucion)?;
    }
    let cuerpo = String::from_utf8_lossy(&cuerpo_bytes).to_string();

    Ok(Value::diccionario(vec![
        ("estado".to_string(), Value::Entero(estado as i64)),
        ("razon".to_string(), Value::Texto(razon)),
        (
            "cabeceras".to_string(),
            Value::diccionario(
                cabeceras
                    .iter()
                    .map(|(k, v)| (k.clone(), Value::Texto(v.clone())))
                    .collect(),
            ),
        ),
        ("cuerpo".to_string(), Value::Texto(cuerpo)),
    ]))
}

fn deschunk(datos: &[u8]) -> Result<Vec<u8>, String> {
    let mut salida = Vec::new();
    let mut pos = 0;
    loop {
        let fin_linea = buscar(&datos[pos..], b"\r\n")
            .map(|i| pos + i)
            .ok_or_else(|| "respuesta chunked inválida".to_string())?;
        let linea = String::from_utf8_lossy(&datos[pos..fin_linea]).to_string();
        let tamano_texto = linea.split(';').next().unwrap_or("0").trim();
        let tamano = usize::from_str_radix(tamano_texto, 16)
            .map_err(|_| format!("tamaño de chunk inválido '{}'", tamano_texto))?;
        pos = fin_linea + 2;
        if tamano == 0 {
            break;
        }
        if pos + tamano > datos.len() {
            return Err("chunk incompleto".to_string());
        }
        salida.extend_from_slice(&datos[pos..pos + tamano]);
        pos += tamano;
        if datos.get(pos..pos + 2) == Some(b"\r\n") {
            pos += 2;
        }
    }
    Ok(salida)
}

// ----------------------------------------------------------------------
// Utilidades URL
// ----------------------------------------------------------------------

fn codificar_url(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(codificar(&args[0].a_texto())))
}

fn decodificar_url(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(decodificar(&args[0].a_texto())))
}

fn estado(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let codigo = args[0]
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("'http.estado' espera un código entero"))?;
    Ok(Value::Texto(razon(codigo as u16).to_string()))
}

pub fn codificar(texto: &str) -> String {
    let mut salida = String::new();
    for byte in texto.as_bytes() {
        let c = *byte as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            salida.push(c);
        } else {
            salida.push_str(&format!("%{:02X}", byte));
        }
    }
    salida
}

pub fn decodificar(texto: &str) -> String {
    let bytes = texto.as_bytes();
    let mut salida = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = &texto[i + 1..i + 3];
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        salida.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        salida.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                salida.push(b' ');
                i += 1;
            }
            byte => {
                salida.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&salida).to_string()
}

fn razon(estado: u16) -> &'static str {
    match estado {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "OK",
    }
}
