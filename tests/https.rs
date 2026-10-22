//! Pruebas de integración de TLS: handshake real contra un servidor de Vaca.
//!
//! La PKI de `tests/certificados/` es **solo para pruebas**: una CA propia que
//! firma un certificado para `localhost`/`127.0.0.1`. No la uses fuera de aquí.

use std::fs;
use std::io::{BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

fn ruta_certificado() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("certificados")
        .join("servidor.pem")
}

fn ruta_clave() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("certificados")
        .join("servidor-clave.pem")
}

fn ruta_ca() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("certificados")
        .join("ca.pem")
}

fn puerto_libre() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("puerto libre");
    listener.local_addr().expect("dirección local").port()
}

struct Servidor {
    hijo: Child,
    dir: PathBuf,
}

impl Drop for Servidor {
    fn drop(&mut self) {
        let _ = self.hijo.kill();
        let _ = self.hijo.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

impl Servidor {
    fn logs(&self) -> String {
        let salida = fs::read_to_string(self.dir.join("stdout.log")).unwrap_or_default();
        let errores = fs::read_to_string(self.dir.join("stderr.log")).unwrap_or_default();
        format!("stdout:\n{}\nstderr:\n{}", salida, errores)
    }
}

fn iniciar_servidor_tls(nombre: &str) -> (Servidor, u16) {
    let puerto = puerto_libre();
    let dir = std::env::temp_dir().join(format!("vaca-tls-{}-{}", nombre, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("directorio temporal");

    let certificado = ruta_certificado();
    let clave = ruta_clave();
    let script = format!(
        r#"importar http

fn manejador(peticion) {{
    retornar {{"cuerpo": a"tls:{{peticion.ruta}}"}}
}}

http.servidor_tls("127.0.0.1", {puerto}, "{cert}", "{key}", manejador)
"#,
        puerto = puerto,
        cert = certificado.display(),
        key = clave.display(),
    );
    fs::write(dir.join("servidor.vaca"), script).expect("escribir servidor");

    let salida = fs::File::create(dir.join("stdout.log")).expect("log de salida");
    let errores = fs::File::create(dir.join("stderr.log")).expect("log de errores");
    let hijo = Command::new(env!("CARGO_BIN_EXE_vaca"))
        .arg("servidor.vaca")
        .current_dir(&dir)
        .stdout(Stdio::from(salida))
        .stderr(Stdio::from(errores))
        .spawn()
        .expect("iniciar vaca");

    let servidor = Servidor { hijo, dir };

    // Esperamos a que el puerto acepte conexiones.
    let limite = Instant::now() + Duration::from_secs(15);
    loop {
        if TcpStream::connect(("127.0.0.1", puerto)).is_ok() {
            break;
        }
        if Instant::now() >= limite {
            panic!(
                "el servidor TLS no arrancó en el puerto {}\n{}",
                puerto,
                servidor.logs()
            );
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    (servidor, puerto)
}

fn cliente_rustls(certificado: &Path) -> Arc<ClientConfig> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let bytes = fs::read(certificado).expect("leer certificado de prueba");
    let mut raices = RootCertStore::empty();
    for certificado in rustls_pemfile::certs(&mut BufReader::new(&bytes[..])) {
        raices
            .add(certificado.expect("certificado PEM"))
            .expect("añadir raíz");
    }
    Arc::new(
        ClientConfig::builder()
            .with_root_certificates(raices)
            .with_no_client_auth(),
    )
}

#[test]
fn servidor_tls_completa_handshake_y_responde_con_hsts() {
    let (servidor, puerto) = iniciar_servidor_tls("servidor");

    let tcp = TcpStream::connect(("127.0.0.1", puerto)).expect("conectar");
    tcp.set_read_timeout(Some(Duration::from_secs(10)))
        .expect("timeout de lectura");
    tcp.set_write_timeout(Some(Duration::from_secs(10)))
        .expect("timeout de escritura");

    let nombre = ServerName::try_from("localhost").expect("nombre TLS");
    let conexion = ClientConnection::new(cliente_rustls(&ruta_ca()), nombre).expect("conexión TLS");
    let mut flujo = StreamOwned::new(conexion, tcp);

    flujo
        .write_all(b"GET /salud HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .expect("enviar petición");
    flujo.flush().expect("flush");

    let mut respuesta = Vec::new();
    flujo.read_to_end(&mut respuesta).expect("leer respuesta");
    let texto = String::from_utf8_lossy(&respuesta);

    assert!(
        texto.starts_with("HTTP/1.1 200"),
        "respuesta inesperada: {}\n{}",
        texto,
        servidor.logs()
    );
    assert!(texto.contains("tls:/salud"), "cuerpo inesperado: {}", texto);
    assert!(
        texto.to_lowercase().contains("strict-transport-security"),
        "falta HSTS: {}",
        texto
    );
}

#[test]
fn cliente_https_valida_el_certificado_con_vaca_ca() {
    let (servidor, puerto) = iniciar_servidor_tls("cliente");

    let script = format!(
        r#"importar http

respuesta = http.peticion(a"https://127.0.0.1:{puerto}/salud", "GET")
imprimir(respuesta.estado)
imprimir(respuesta.cuerpo)
"#,
        puerto = puerto,
    );
    fs::write(servidor.dir.join("cliente.vaca"), script).expect("escribir cliente");

    let salida = Command::new(env!("CARGO_BIN_EXE_vaca"))
        .arg("cliente.vaca")
        .current_dir(&servidor.dir)
        .env("VACA_CA", ruta_ca())
        .output()
        .expect("ejecutar cliente");

    let stdout = String::from_utf8_lossy(&salida.stdout).to_string();
    let stderr = String::from_utf8_lossy(&salida.stderr).to_string();
    assert!(
        salida.status.success(),
        "el cliente falló: {}\n{}{}",
        stderr,
        stdout,
        servidor.logs()
    );
    assert!(stdout.contains("200"), "estado inesperado: {}", stdout);
    assert!(
        stdout.contains("tls:/salud"),
        "cuerpo inesperado: {}",
        stdout
    );
}

#[test]
fn cliente_rechaza_un_certificado_no_confiable() {
    let (servidor, puerto) = iniciar_servidor_tls("rechazo");

    let script = format!(
        r#"importar http

respuesta = http.peticion(a"https://127.0.0.1:{puerto}/salud", "GET")
imprimir(respuesta.estado)
"#,
        puerto = puerto,
    );
    fs::write(servidor.dir.join("cliente.vaca"), script).expect("escribir cliente");

    // Sin VACA_CA, el certificado autofirmado no está en las raíces de Mozilla.
    let salida = Command::new(env!("CARGO_BIN_EXE_vaca"))
        .arg("cliente.vaca")
        .current_dir(&servidor.dir)
        .env_remove("VACA_CA")
        .output()
        .expect("ejecutar cliente");

    let stderr = String::from_utf8_lossy(&salida.stderr).to_string();
    assert!(
        !salida.status.success(),
        "el cliente no debería aceptar un certificado no confiable\n{}",
        stderr
    );
    assert!(
        stderr.to_lowercase().contains("tls")
            || stderr.to_lowercase().contains("certificado")
            || stderr.to_lowercase().contains("certificate"),
        "error poco claro: {}",
        stderr
    );
}
