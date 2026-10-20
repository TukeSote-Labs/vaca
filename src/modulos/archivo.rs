//! Módulo `archivo`: lectura y escritura de archivos.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::Value;

pub fn modulo() -> Value {
    crear_modulo(
        "archivo",
        vec![
            ("leer", nativa("archivo.leer", 1, 1, leer)),
            ("escribir", nativa("archivo.escribir", 2, 2, escribir)),
            ("anexar", nativa("archivo.anexar", 2, 2, anexar)),
            ("existe", nativa("archivo.existe", 1, 1, existe)),
            ("eliminar", nativa("archivo.eliminar", 1, 1, eliminar)),
            ("listar", nativa("archivo.listar", 0, 1, listar)),
            ("crear_dir", nativa("archivo.crear_dir", 1, 1, crear_dir)),
            ("tamano", nativa("archivo.tamano", 1, 1, tamano)),
            (
                "ruta_actual",
                nativa("archivo.ruta_actual", 0, 0, ruta_actual),
            ),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn resolver(interprete: &Interprete, ruta: &str) -> PathBuf {
    let ruta = Path::new(ruta);
    if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        interprete.ruta_base.join(ruta)
    }
}

fn leer(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver(interprete, &args[0].a_texto());
    let contenido = fs::read_to_string(&ruta)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e)))?;
    Ok(Value::Texto(contenido))
}

fn escribir(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver(interprete, &args[0].a_texto());
    fs::write(&ruta, args[1].a_texto()).map_err(|e| {
        Error::ejecucion(format!("no se pudo escribir '{}': {}", ruta.display(), e))
    })?;
    Ok(Value::Nulo)
}

fn anexar(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    use std::io::Write;
    let ruta = resolver(interprete, &args[0].a_texto());
    let mut archivo = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ruta)
        .map_err(|e| Error::ejecucion(format!("no se pudo abrir '{}': {}", ruta.display(), e)))?;
    archivo
        .write_all(args[1].a_texto().as_bytes())
        .map_err(|e| {
            Error::ejecucion(format!("no se pudo anexar a '{}': {}", ruta.display(), e))
        })?;
    Ok(Value::Nulo)
}

fn existe(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver(interprete, &args[0].a_texto());
    Ok(Value::Logico(ruta.exists()))
}

fn eliminar(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver(interprete, &args[0].a_texto());
    if ruta.is_dir() {
        fs::remove_dir_all(&ruta).map_err(|e| {
            Error::ejecucion(format!(
                "no se pudo eliminar el directorio '{}': {}",
                ruta.display(),
                e
            ))
        })?;
    } else {
        fs::remove_file(&ruta).map_err(|e| {
            Error::ejecucion(format!("no se pudo eliminar '{}': {}", ruta.display(), e))
        })?;
    }
    Ok(Value::Nulo)
}

fn listar(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = match args.first() {
        Some(v) => resolver(interprete, &v.a_texto()),
        None => interprete.ruta_base.clone(),
    };
    let mut nombres = Vec::new();
    for entrada in fs::read_dir(&ruta)
        .map_err(|e| Error::ejecucion(format!("no se pudo listar '{}': {}", ruta.display(), e)))?
    {
        let entrada =
            entrada.map_err(|e| Error::ejecucion(format!("error al listar directorio: {}", e)))?;
        nombres.push(Value::Texto(
            entrada.file_name().to_string_lossy().to_string(),
        ));
    }
    nombres.sort_by_key(|a| a.a_texto());
    Ok(Value::lista(nombres))
}

fn crear_dir(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver(interprete, &args[0].a_texto());
    fs::create_dir_all(&ruta).map_err(|e| {
        Error::ejecucion(format!(
            "no se pudo crear el directorio '{}': {}",
            ruta.display(),
            e
        ))
    })?;
    Ok(Value::Nulo)
}

fn tamano(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver(interprete, &args[0].a_texto());
    let metadatos = fs::metadata(&ruta)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e)))?;
    Ok(Value::Entero(metadatos.len() as i64))
}

fn ruta_actual(interprete: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(
        interprete.ruta_base.to_string_lossy().to_string(),
    ))
}
