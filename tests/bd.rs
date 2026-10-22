//! Pruebas del módulo `bd` (SQLite).

use std::fs;
use std::path::PathBuf;

struct Directorio {
    raiz: PathBuf,
}

impl Drop for Directorio {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.raiz);
    }
}

fn directorio(nombre: &str) -> Directorio {
    let raiz = std::env::temp_dir().join(format!("vaca-bd-{}-{}", nombre, std::process::id()));
    let _ = fs::remove_dir_all(&raiz);
    fs::create_dir_all(&raiz).expect("crear directorio temporal");
    Directorio { raiz }
}

#[test]
fn sqlite_crud_parametros_y_transacciones() {
    let programa = r#"
importar bd

db = bd.conectar("sqlite::memory:")
afirmar(db.motor() == "sqlite", "motor incorrecto")
afirmar(db.tipo == "conexion", "tipo incorrecto")
afirmar(db.ruta() == ":memory:", "ruta incorrecta")
afirmar(db.abierta(), "la conexión debería estar abierta")
afirmar(longitud(db.version()) > 0, "sin versión de SQLite")

db.ejecutar("CREATE TABLE usuarios (id INTEGER PRIMARY KEY AUTOINCREMENT, nombre TEXT NOT NULL, saldo REAL, activo INTEGER, nota TEXT)")
insercion = db.ejecutar("INSERT INTO usuarios (nombre, saldo, activo, nota) VALUES (?, ?, ?, ?)", ["Ana", 10.5, verdadero, nulo])
afirmar(insercion.filas == 1, "filas del INSERT incorrectas")
afirmar(insercion.ultimo_id == 1, "ultimo_id incorrecto")
db.ejecutar("INSERT INTO usuarios (nombre, saldo, activo, nota) VALUES (?, ?, ?, ?)", ["Luis", 3.0, falso, "x"])

filas = db.consultar("SELECT id, nombre, saldo, activo, nota FROM usuarios ORDER BY id")
afirmar(longitud(filas) == 2, "esperaba 2 filas")
afirmar(filas[0].nombre == "Ana", "nombre incorrecto")
afirmar(filas[1].saldo == 3.0, "saldo incorrecto")
afirmar(filas[0].nota == nulo, "la nota debería ser nula")

afirmar(db.una("SELECT nombre FROM usuarios WHERE id = ?", [1]).nombre == "Ana", "una() incorrecto")
afirmar(db.valor("SELECT COUNT(*) FROM usuarios") == 2, "valor() incorrecto")
afirmar(longitud(db.consultar("SELECT * FROM usuarios WHERE id = ?", [999])) == 0, "consulta vacía incorrecta")

db.iniciar()
db.ejecutar("INSERT INTO usuarios (nombre) VALUES (?)", ["Temporal"])
db.revertir()
afirmar(db.valor("SELECT COUNT(*) FROM usuarios") == 2, "revertir no deshizo el insert")

db.iniciar()
db.ejecutar("INSERT INTO usuarios (nombre) VALUES (?)", ["Confirmado"])
db.confirmar()
afirmar(db.valor("SELECT COUNT(*) FROM usuarios") == 3, "confirmar no guardó el insert")

db.cerrar()
afirmar(no db.abierta(), "la conexión debería estar cerrada")

intentar {
    db.consultar("SELECT 1")
} atrapar error {
    afirmar(longitud(error) > 0, "se esperaba un error de conexión cerrada")
}

imprimir("ok")
"#;

    let salida = vaca::ejecutar_fuente(programa).expect("ejecutar programa SQLite");
    assert_eq!(salida.trim(), "ok");
}

#[test]
fn sqlite_persiste_en_archivo() {
    let proyecto = directorio("archivo");

    let crear = proyecto.raiz.join("crear.vaca");
    fs::write(
        &crear,
        r#"
importar bd

db = bd.conectar("sqlite:datos.db")
db.ejecutar("CREATE TABLE IF NOT EXISTS notas (id INTEGER PRIMARY KEY, texto TEXT)")
db.ejecutar("INSERT INTO notas (texto) VALUES (?)", ["persistente"])
imprimir(db.valor("SELECT COUNT(*) FROM notas"))
"#,
    )
    .expect("escribir crear.vaca");

    let leer = proyecto.raiz.join("leer.vaca");
    fs::write(
        &leer,
        r#"
importar bd

db = bd.conectar("sqlite:datos.db")
imprimir(db.valor("SELECT texto FROM notas WHERE id = 1"))
"#,
    )
    .expect("escribir leer.vaca");

    let salida = vaca::ejecutar_archivo(&crear).expect("crear base");
    assert_eq!(salida.trim(), "1");

    let salida = vaca::ejecutar_archivo(&leer).expect("leer base");
    assert_eq!(salida.trim(), "persistente");
}

#[test]
fn rechaza_un_dsn_no_soportado() {
    let error = vaca::ejecutar_fuente(
        r#"
importar bd
db = bd.conectar("mysql://localhost/base")
"#,
    )
    .expect_err("debe rechazar el DSN");

    assert!(
        error.to_string().contains("DSN no soportado"),
        "error: {}",
        error
    );
}

#[test]
fn crear_falso_no_crea_el_archivo() {
    let proyecto = directorio("sin-crear");
    let programa = proyecto.raiz.join("programa.vaca");
    fs::write(
        &programa,
        r#"
importar bd

db = bd.conectar("sqlite:no-existe.db", {"crear": falso})
imprimir(db.motor())
"#,
    )
    .expect("escribir programa");

    let error = vaca::ejecutar_archivo(&programa).expect_err("debe fallar sin crear");
    assert!(error.to_string().contains("no existe"), "error: {}", error);
    assert!(!proyecto.raiz.join("no-existe.db").exists());
}

#[test]
fn crear_falso_abre_una_base_existente() {
    let proyecto = directorio("sin-crear-existente");
    let crear = proyecto.raiz.join("crear.vaca");
    fs::write(
        &crear,
        r#"
importar bd

db = bd.conectar("sqlite:datos.db")
db.ejecutar("CREATE TABLE notas (id INTEGER PRIMARY KEY, texto TEXT)")
db.ejecutar("INSERT INTO notas (texto) VALUES (?)", ["hola"])
"#,
    )
    .expect("escribir crear");
    vaca::ejecutar_archivo(&crear).expect("crear base");

    let leer = proyecto.raiz.join("leer.vaca");
    fs::write(
        &leer,
        r#"
importar bd

db = bd.conectar("sqlite:datos.db", {"crear": falso})
afirmar(db.solo_lectura() == falso, "no es solo lectura")
imprimir(db.valor("SELECT texto FROM notas WHERE id = 1"))
"#,
    )
    .expect("escribir leer");

    let salida = vaca::ejecutar_archivo(&leer).expect("abrir existente sin crear");
    assert_eq!(salida.trim(), "hola");
}

#[test]
fn solo_lectura_lee_y_no_escribe() {
    let proyecto = directorio("solo-lectura");
    let crear = proyecto.raiz.join("crear.vaca");
    fs::write(
        &crear,
        r#"
importar bd

db = bd.conectar("sqlite:datos.db")
db.ejecutar("CREATE TABLE notas (id INTEGER PRIMARY KEY, texto TEXT)")
db.ejecutar("INSERT INTO notas (texto) VALUES (?)", ["hola"])
"#,
    )
    .expect("escribir crear");
    vaca::ejecutar_archivo(&crear).expect("crear base");

    let leer = proyecto.raiz.join("leer.vaca");
    fs::write(
        &leer,
        r#"
importar bd

db = bd.abrir_solo_lectura("datos.db")
afirmar(db.solo_lectura(), "debería ser solo lectura")
afirmar(db.valor("SELECT COUNT(*) FROM notas") == 1, "no leyó la fila")

intentar {
    db.ejecutar("INSERT INTO notas (texto) VALUES (?)", ["nuevo"])
} atrapar error {
    afirmar(longitud(error) > 0, "se esperaba un error de escritura")
}

db.cerrar()
"#,
    )
    .expect("escribir leer");
    vaca::ejecutar_archivo(&leer).expect("abrir en solo lectura");
}

#[test]
fn solo_lectura_no_crea_una_base_inexistente() {
    let proyecto = directorio("solo-lectura-nueva");
    let programa = proyecto.raiz.join("programa.vaca");
    fs::write(
        &programa,
        r#"
importar bd

db = bd.abrir_solo_lectura("no-existe.db")
"#,
    )
    .expect("escribir programa");

    let error = vaca::ejecutar_archivo(&programa).expect_err("debe fallar");
    assert!(
        error
            .to_string()
            .contains("no se creará en modo solo lectura"),
        "error: {}",
        error
    );
    assert!(!proyecto.raiz.join("no-existe.db").exists());
}

#[test]
fn rechaza_opciones_desconocidas() {
    let proyecto = directorio("opciones");
    let programa = proyecto.raiz.join("programa.vaca");
    fs::write(
        &programa,
        r#"
importar bd

db = bd.conectar("sqlite::memory:", {"inventada": verdadero})
"#,
    )
    .expect("escribir programa");

    let error = vaca::ejecutar_archivo(&programa).expect_err("debe fallar");
    assert!(
        error.to_string().contains("opción de conexión desconocida"),
        "error: {}",
        error
    );
}
