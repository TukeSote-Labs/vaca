//! Prueba de integración del servidor HTTP concurrente.
//!
//! Levanta el binario `vaca` con un servidor de prueba, lanza varias
//! peticiones lentas a la vez y comprueba que se atienden en paralelo y que
//! el límite de conexiones responde 503.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Hijo(Child);

impl Drop for Hijo {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn directorio_temporal(nombre: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut ruta = std::env::temp_dir();
    ruta.push(format!(
        "vaca_servidor_{}_{}_{}",
        nombre,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&ruta).unwrap();
    ruta
}

fn puerto_libre() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn peticion(puerto: u16, ruta: &str) -> Result<(u16, String), String> {
    let mut flujo =
        TcpStream::connect(("127.0.0.1", puerto)).map_err(|e| format!("conexión: {}", e))?;
    flujo
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    flujo
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    let solicitud = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        ruta
    );
    flujo
        .write_all(solicitud.as_bytes())
        .map_err(|e| format!("escritura: {}", e))?;

    let mut respuesta = String::new();
    flujo
        .read_to_string(&mut respuesta)
        .map_err(|e| format!("lectura: {}", e))?;

    let estado = respuesta
        .lines()
        .next()
        .and_then(|linea| linea.split_whitespace().nth(1))
        .and_then(|codigo| codigo.parse().ok())
        .ok_or_else(|| format!("respuesta HTTP inválida: {:?}", respuesta))?;
    let cuerpo = respuesta
        .split_once("\r\n\r\n")
        .map(|(_, cuerpo)| cuerpo.to_string())
        .unwrap_or_default();
    Ok((estado, cuerpo))
}

fn esperar_servidor(puerto: u16) {
    let limite = Instant::now() + Duration::from_secs(10);
    while Instant::now() < limite {
        if peticion(puerto, "/metricas").is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("el servidor no respondió a tiempo");
}

#[test]
fn servidor_atiende_en_paralelo_y_respeta_el_limite() {
    let puerto = puerto_libre();
    let dir = directorio_temporal("concurrente");
    let programa = dir.join("servidor.vaca");
    fs::write(
        &programa,
        format!(
            r#"
            importar http
            importar tiempo
            importar concurrencia
            importar sistema

            procesadas = concurrencia.contador(0)

            fn manejador(peticion) {{
                si peticion.ruta == "/lento" {{
                    numero = procesadas.incrementar()
                    tiempo.dormir(200)
                    retornar {{"cuerpo": {{"numero": numero}}}}
                }}
                si peticion.ruta == "/metricas" {{
                    retornar {{"cuerpo": {{"procesadas": procesadas.valor()}}}}
                }}
                si peticion.ruta == "/detener" {{
                    sistema.salir(0)
                }}
                retornar {{"estado": 404, "cuerpo": {{}}}}
            }}

            http.servidor("127.0.0.1", {}, manejador, 2)
            "#,
            puerto
        ),
    )
    .unwrap();

    let hijo = Command::new(env!("CARGO_BIN_EXE_vaca"))
        .arg(&programa)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("no se pudo iniciar el servidor");
    let _hijo = Hijo(hijo);

    esperar_servidor(puerto);
    // Deja que el servidor cierre la conexión de sondeo y libere su plaza.
    std::thread::sleep(Duration::from_millis(50));

    let inicio = Instant::now();
    let mut hilos = Vec::new();
    for _ in 0..6 {
        hilos.push(std::thread::spawn(move || peticion(puerto, "/lento")));
    }
    let resultados: Vec<_> = hilos.into_iter().map(|hilo| hilo.join().unwrap()).collect();
    let transcurrido = inicio.elapsed();

    let mut exitos = 0;
    let mut rechazos = 0;
    for resultado in resultados {
        let (estado, _) = resultado.expect("petición fallida");
        match estado {
            200 => exitos += 1,
            503 => rechazos += 1,
            otro => panic!("estado inesperado: {}", otro),
        }
    }

    assert_eq!(exitos, 2, "el límite debe permitir dos conexiones a la vez");
    assert_eq!(rechazos, 4, "las demás conexiones deben recibir 503");
    assert!(
        transcurrido < Duration::from_millis(900),
        "las peticiones tardaron {:?}; no se atendieron en paralelo",
        transcurrido
    );

    // Espera a que los hilos de atención liberen sus plazas.
    std::thread::sleep(Duration::from_millis(50));
    let (estado, cuerpo) = peticion(puerto, "/metricas").unwrap();
    assert_eq!(estado, 200);
    assert!(
        cuerpo.contains("\"procesadas\":2"),
        "el contador atómico no coincide: {}",
        cuerpo
    );

    // Detiene el servidor desde un manejador. La respuesta se corta porque el
    // proceso termina, así que el error de conexión es esperado.
    let _ = peticion(puerto, "/detener");

    let _ = fs::remove_dir_all(&dir);
}
