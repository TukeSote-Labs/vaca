//! Módulo `bd`: conexiones a SQLite.
//!
//! ```vaca
//! importar bd
//!
//! db = bd.conectar("sqlite:datos.db")
//! db.ejecutar("CREATE TABLE usuarios (id INTEGER PRIMARY KEY, nombre TEXT)")
//! db.ejecutar("INSERT INTO usuarios (nombre) VALUES (?)", ["Ana"])
//!
//! para fila en db.consultar("SELECT id, nombre FROM usuarios") {
//!     imprimir(a"{fila.id}: {fila.nombre}")
//! }
//! ```
//!
//! Los parámetros siempre se enlazan; nunca se interpola texto en el SQL.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::types::{Value as ValorSql, ValueRef};
use rusqlite::{Connection as ConexionSqlite, OpenFlags};

use crate::error::Error;
use crate::interprete::{verificar_aridad, Interprete};
use crate::modulos::crear_modulo;
use crate::value::{MapaValores, ObjetoNativo, Value};

pub fn modulo() -> Value {
    crear_modulo(
        "bd",
        vec![
            ("conectar", nativa("bd.conectar", 1, 2, conectar)),
            ("sqlite", nativa("bd.sqlite", 1, 2, conectar_sqlite)),
            (
                "abrir_solo_lectura",
                nativa("bd.abrir_solo_lectura", 1, 1, conectar_solo_lectura),
            ),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

// ----------------------------------------------------------------------
// Conexión
// ----------------------------------------------------------------------

struct ConexionBd {
    descripcion: String,
    solo_lectura: bool,
    motor: Mutex<Option<ConexionSqlite>>,
}

impl ConexionBd {
    fn nuevo(conexion: ConexionSqlite, descripcion: String, solo_lectura: bool) -> Value {
        let conexion = ConexionBd {
            descripcion,
            solo_lectura,
            motor: Mutex::new(Some(conexion)),
        };
        Value::Nativo(ObjetoNativo::nuevo("conexion", conexion, despachar))
    }
}

#[derive(Clone, Copy)]
struct OpcionesBd {
    crear: bool,
    solo_lectura: bool,
    tiempo_espera_ms: u64,
}

impl Default for OpcionesBd {
    fn default() -> Self {
        OpcionesBd {
            crear: true,
            solo_lectura: false,
            tiempo_espera_ms: 5000,
        }
    }
}

fn leer_opciones(valor: Option<&Value>) -> Result<OpcionesBd, Error> {
    let mut opciones = OpcionesBd::default();
    let Some(valor) = valor else {
        return Ok(opciones);
    };
    if matches!(valor, Value::Nulo) {
        return Ok(opciones);
    }
    let Value::Diccionario(mapa) = valor else {
        return Err(Error::ejecucion(format!(
            "las opciones de conexión deben ser un diccionario, no un valor de tipo {}",
            valor.tipo_nombre()
        )));
    };
    let mapa = mapa.lock().unwrap().clone();

    for (clave, valor) in &mapa {
        match clave.as_str() {
            "crear" => opciones.crear = leer_logico("crear", valor)?,
            "solo_lectura" => opciones.solo_lectura = leer_logico("solo_lectura", valor)?,
            "tiempo_espera" => {
                opciones.tiempo_espera_ms = match valor {
                    Value::Entero(ms) if *ms >= 0 => *ms as u64,
                    _ => {
                        return Err(Error::ejecucion(
                            "'tiempo_espera' debe ser un entero de milisegundos".to_string(),
                        ))
                    }
                };
            }
            otra => {
                return Err(Error::ejecucion(format!(
                    "opción de conexión desconocida '{}'; usa crear, solo_lectura o tiempo_espera",
                    otra
                )))
            }
        }
    }

    if opciones.solo_lectura {
        opciones.crear = false;
    }
    Ok(opciones)
}

fn leer_logico(nombre: &str, valor: &Value) -> Result<bool, Error> {
    match valor {
        Value::Logico(valor) => Ok(*valor),
        otro => Err(Error::ejecucion(format!(
            "'{}' debe ser verdadero o falso, no un valor de tipo {}",
            nombre,
            otro.tipo_nombre()
        ))),
    }
}

fn conectar(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let dsn = args[0].a_texto();
    if !dsn.starts_with("sqlite:") {
        return Err(Error::ejecucion(format!(
            "DSN no soportado '{}'; Vaca solo incluye SQLite: usa 'sqlite:ruta.db' o 'sqlite::memory:'",
            dsn
        )));
    }
    let opciones = leer_opciones(args.get(1))?;
    abrir_sqlite(interprete, &dsn, &opciones)
}

fn conectar_sqlite(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let dsn = dsn_sqlite(&args[0].a_texto());
    let opciones = leer_opciones(args.get(1))?;
    abrir_sqlite(interprete, &dsn, &opciones)
}

fn conectar_solo_lectura(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let dsn = dsn_sqlite(&args[0].a_texto());
    let opciones = OpcionesBd {
        crear: false,
        solo_lectura: true,
        ..OpcionesBd::default()
    };
    abrir_sqlite(interprete, &dsn, &opciones)
}

fn dsn_sqlite(ruta: &str) -> String {
    if ruta.starts_with("sqlite:") {
        ruta.to_string()
    } else {
        format!("sqlite:{}", ruta)
    }
}

fn abrir_sqlite(interprete: &Interprete, dsn: &str, opciones: &OpcionesBd) -> Result<Value, Error> {
    let resto = dsn.strip_prefix("sqlite:").unwrap_or(dsn);
    let resto = resto.strip_prefix("//").unwrap_or(resto);
    let memoria = resto.is_empty() || resto == ":memory:";

    if memoria && opciones.solo_lectura {
        return Err(Error::ejecucion(
            "una base SQLite en memoria no puede abrirse en solo lectura".to_string(),
        ));
    }

    let (conexion, descripcion) = if memoria {
        (ConexionSqlite::open_in_memory(), ":memory:".to_string())
    } else {
        let ruta = resolver(interprete, resto);
        validar_ruta_bd(&ruta, opciones)?;

        let flags = if opciones.solo_lectura {
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI
        } else if opciones.crear {
            OpenFlags::default()
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_URI
        };
        (
            ConexionSqlite::open_with_flags(&ruta, flags),
            ruta.display().to_string(),
        )
    };
    let conexion = conexion.map_err(|e| {
        Error::ejecucion(format!("no se pudo abrir SQLite '{}': {}", descripcion, e))
    })?;

    conexion
        .busy_timeout(Duration::from_millis(opciones.tiempo_espera_ms))
        .map_err(|e| Error::ejecucion(format!("no se pudo configurar SQLite: {}", e)))?;
    conexion
        .execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| Error::ejecucion(format!("no se pudo inicializar SQLite: {}", e)))?;

    Ok(ConexionBd::nuevo(
        conexion,
        descripcion,
        opciones.solo_lectura,
    ))
}

fn validar_ruta_bd(ruta: &Path, opciones: &OpcionesBd) -> Result<(), Error> {
    if ruta.is_dir() {
        return Err(Error::ejecucion(format!(
            "'{}' es un directorio, no una base de datos",
            ruta.display()
        )));
    }

    if opciones.solo_lectura && !ruta.is_file() {
        return Err(Error::ejecucion(format!(
            "la base de datos '{}' no existe; no se creará en modo solo lectura",
            ruta.display()
        )));
    }

    if !opciones.crear && !ruta.is_file() {
        return Err(Error::ejecucion(format!(
            "la base de datos '{}' no existe y la opción 'crear' es falsa",
            ruta.display()
        )));
    }

    if opciones.crear {
        if let Some(padre) = ruta.parent() {
            if !padre.as_os_str().is_empty() && !padre.is_dir() {
                return Err(Error::ejecucion(format!(
                    "el directorio '{}' no existe; créalo antes de conectar",
                    padre.display()
                )));
            }
        }
    }

    Ok(())
}

fn resolver(interprete: &Interprete, ruta: &str) -> PathBuf {
    let ruta = Path::new(ruta);
    if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        interprete.ruta_base.join(ruta)
    }
}

// ----------------------------------------------------------------------
// Métodos de la conexión
// ----------------------------------------------------------------------

fn despachar(
    _interprete: &mut Interprete,
    objeto: &Arc<ObjetoNativo>,
    nombre: &str,
    args: Vec<Value>,
) -> Result<Value, Error> {
    let conexion = objeto
        .datos::<ConexionBd>()
        .ok_or_else(|| Error::ejecucion("objeto de base de datos inválido".to_string()))?;

    match nombre {
        "consultar" => {
            verificar_aridad("conexión.consultar", 1, 2, &args)?;
            consultar(conexion, &args)
        }
        "ejecutar" => {
            verificar_aridad("conexión.ejecutar", 1, 2, &args)?;
            ejecutar(conexion, &args)
        }
        "una" => {
            verificar_aridad("conexión.una", 1, 2, &args)?;
            una(conexion, &args)
        }
        "valor" => {
            verificar_aridad("conexión.valor", 1, 2, &args)?;
            valor(conexion, &args)
        }
        "lote" => {
            verificar_aridad("conexión.lote", 1, 1, &args)?;
            lote(conexion, &args[0].a_texto())
        }
        "iniciar" => orden(conexion, "BEGIN", "conexión.iniciar", &args),
        "confirmar" => orden(conexion, "COMMIT", "conexión.confirmar", &args),
        "revertir" => orden(conexion, "ROLLBACK", "conexión.revertir", &args),
        "cerrar" => {
            verificar_aridad("conexión.cerrar", 0, 0, &args)?;
            cerrar(conexion)
        }
        "abierta" => {
            verificar_aridad("conexión.abierta", 0, 0, &args)?;
            Ok(Value::Logico(conexion.motor.lock().unwrap().is_some()))
        }
        "solo_lectura" => {
            verificar_aridad("conexión.solo_lectura", 0, 0, &args)?;
            Ok(Value::Logico(conexion.solo_lectura))
        }
        "motor" => {
            verificar_aridad("conexión.motor", 0, 0, &args)?;
            Ok(Value::Texto("sqlite".to_string()))
        }
        "ruta" => {
            verificar_aridad("conexión.ruta", 0, 0, &args)?;
            Ok(Value::Texto(conexion.descripcion.clone()))
        }
        "version" => {
            verificar_aridad("conexión.version", 0, 0, &args)?;
            version(conexion)
        }
        otro => Err(Error::ejecucion(format!(
            "una conexión no tiene el método '{}'; usa consultar, ejecutar, una, valor, lote, iniciar, confirmar, revertir, cerrar, abierta, solo_lectura, motor, ruta o version",
            otro
        ))),
    }
}

fn conectar_motor<'a>(
    conexion: &'a ConexionBd,
    operacion: &str,
) -> Result<std::sync::MutexGuard<'a, Option<ConexionSqlite>>, Error> {
    let guardia = conexion.motor.lock().unwrap();
    if guardia.is_none() {
        return Err(Error::ejecucion(format!(
            "la conexión está cerrada; no se puede {}",
            operacion
        )));
    }
    Ok(guardia)
}

fn consultar(conexion: &ConexionBd, args: &[Value]) -> Result<Value, Error> {
    let sql = args[0].a_texto();
    let parametros = leer_parametros(args.get(1))?;
    let mut guardia = conectar_motor(conexion, "consultar")?;
    consultar_sqlite(guardia.as_mut().unwrap(), &sql, &parametros)
}

fn ejecutar(conexion: &ConexionBd, args: &[Value]) -> Result<Value, Error> {
    let sql = args[0].a_texto();
    let parametros = leer_parametros(args.get(1))?;
    let mut guardia = conectar_motor(conexion, "ejecutar")?;
    ejecutar_sqlite(guardia.as_mut().unwrap(), &sql, &parametros)
}

fn una(conexion: &ConexionBd, args: &[Value]) -> Result<Value, Error> {
    let filas = consultar(conexion, args)?;
    let Value::Lista(lista) = filas else {
        return Ok(Value::Nulo);
    };
    let primera = lista.lock().unwrap().first().cloned();
    Ok(primera.unwrap_or(Value::Nulo))
}

fn valor(conexion: &ConexionBd, args: &[Value]) -> Result<Value, Error> {
    let sql = args[0].a_texto();
    let parametros = leer_parametros(args.get(1))?;
    let mut guardia = conectar_motor(conexion, "consultar un valor")?;
    valor_sqlite(guardia.as_mut().unwrap(), &sql, &parametros)
}

fn lote(conexion: &ConexionBd, sql: &str) -> Result<Value, Error> {
    let guardia = conectar_motor(conexion, "ejecutar un lote")?;
    guardia
        .as_ref()
        .unwrap()
        .execute_batch(sql)
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
    Ok(Value::Nulo)
}

fn orden(conexion: &ConexionBd, sql: &str, nombre: &str, args: &[Value]) -> Result<Value, Error> {
    verificar_aridad(nombre, 0, 0, args)?;
    lote(conexion, sql)
}

fn cerrar(conexion: &ConexionBd) -> Result<Value, Error> {
    let mut guardia = conexion.motor.lock().unwrap();
    if let Some(con) = guardia.take() {
        let _ = con.close();
    }
    Ok(Value::Nulo)
}

fn version(conexion: &ConexionBd) -> Result<Value, Error> {
    let guardia = conectar_motor(conexion, "consultar la versión")?;
    let version = guardia
        .as_ref()
        .unwrap()
        .query_row("SELECT sqlite_version()", [], |fila| {
            fila.get::<_, String>(0)
        })
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
    Ok(Value::Texto(version))
}

fn leer_parametros(valor: Option<&Value>) -> Result<Vec<Value>, Error> {
    match valor {
        None | Some(Value::Nulo) => Ok(Vec::new()),
        Some(Value::Lista(lista)) => Ok(lista.lock().unwrap().clone()),
        Some(otro) => Err(Error::ejecucion(format!(
            "los parámetros deben ser una lista, no un valor de tipo {}",
            otro.tipo_nombre()
        ))),
    }
}

fn resultado_ejecucion(filas: i64, ultimo_id: Value) -> Value {
    Value::diccionario(vec![
        ("filas".to_string(), Value::Entero(filas)),
        ("ultimo_id".to_string(), ultimo_id),
    ])
}

// ----------------------------------------------------------------------
// SQLite
// ----------------------------------------------------------------------

fn consultar_sqlite(con: &ConexionSqlite, sql: &str, parametros: &[Value]) -> Result<Value, Error> {
    let valores = convertir_sqlite(parametros)?;
    let mut sentencia = con
        .prepare(sql)
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
    let nombres: Vec<String> = sentencia
        .column_names()
        .iter()
        .map(|nombre| nombre.to_string())
        .collect();

    let mut filas = sentencia
        .query(rusqlite::params_from_iter(valores))
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;

    let mut resultado = Vec::new();
    while let Some(fila) = filas
        .next()
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?
    {
        let mut mapa = MapaValores::default();
        for (indice, nombre) in nombres.iter().enumerate() {
            let valor = fila
                .get_ref(indice)
                .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
            mapa.insert(nombre.clone(), convertir_valor_sqlite(valor));
        }
        resultado.push(Value::Diccionario(Arc::new(Mutex::new(mapa))));
    }
    Ok(Value::lista(resultado))
}

fn ejecutar_sqlite(con: &ConexionSqlite, sql: &str, parametros: &[Value]) -> Result<Value, Error> {
    let valores = convertir_sqlite(parametros)?;
    let filas = con
        .execute(sql, rusqlite::params_from_iter(valores))
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
    let es_insert = sql.trim_start().to_ascii_lowercase().starts_with("insert");
    let ultimo_id = if es_insert {
        Value::Entero(con.last_insert_rowid())
    } else {
        Value::Nulo
    };
    Ok(resultado_ejecucion(filas as i64, ultimo_id))
}

fn valor_sqlite(con: &ConexionSqlite, sql: &str, parametros: &[Value]) -> Result<Value, Error> {
    let valores = convertir_sqlite(parametros)?;
    let mut sentencia = con
        .prepare(sql)
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
    let mut filas = sentencia
        .query(rusqlite::params_from_iter(valores))
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?;
    match filas
        .next()
        .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?
    {
        Some(fila) => Ok(convertir_valor_sqlite(
            fila.get_ref(0)
                .map_err(|e| Error::ejecucion(format!("SQLite: {}", e)))?,
        )),
        None => Ok(Value::Nulo),
    }
}

fn convertir_sqlite(parametros: &[Value]) -> Result<Vec<ValorSql>, Error> {
    parametros
        .iter()
        .map(|valor| match valor {
            Value::Entero(n) => Ok(ValorSql::Integer(*n)),
            Value::Decimal(n) => Ok(ValorSql::Real(*n)),
            Value::Texto(texto) => Ok(ValorSql::Text(texto.clone())),
            Value::Logico(b) => Ok(ValorSql::Integer(i64::from(*b))),
            Value::Nulo => Ok(ValorSql::Null),
            otro => Err(Error::ejecucion(format!(
                "SQLite no admite un parámetro de tipo {}",
                otro.tipo_nombre()
            ))),
        })
        .collect()
}

fn convertir_valor_sqlite(valor: ValueRef<'_>) -> Value {
    match valor {
        ValueRef::Null => Value::Nulo,
        ValueRef::Integer(n) => Value::Entero(n),
        ValueRef::Real(n) => Value::Decimal(n),
        ValueRef::Text(texto) => Value::Texto(String::from_utf8_lossy(texto).to_string()),
        ValueRef::Blob(bytes) => Value::Texto(String::from_utf8_lossy(bytes).to_string()),
    }
}
