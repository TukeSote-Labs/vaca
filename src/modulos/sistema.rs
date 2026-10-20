//! Módulo `sistema`: entorno, argumentos y salida del proceso.

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::Value;

pub fn modulo() -> Value {
    crear_modulo(
        "sistema",
        vec![
            ("env", nativa("sistema.env", 1, 2, env)),
            ("argumentos", nativa("sistema.argumentos", 0, 0, argumentos)),
            ("salir", nativa("sistema.salir", 0, 1, salir)),
            ("directorio", nativa("sistema.directorio", 0, 0, directorio)),
            ("plataforma", nativa("sistema.plataforma", 0, 0, plataforma)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn env(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let nombre = args[0].a_texto();
    match std::env::var(&nombre) {
        Ok(valor) => Ok(Value::Texto(valor)),
        Err(_) => Ok(args.get(1).cloned().unwrap_or(Value::Nulo)),
    }
}

fn argumentos(interprete: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::lista(
        interprete
            .argumentos
            .iter()
            .map(|a| Value::Texto(a.clone()))
            .collect(),
    ))
}

fn salir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let codigo = match args.first() {
        Some(v) => v
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("'salir' espera un código entero"))?,
        None => 0,
    };
    Err(Error::Salir(codigo as i32))
}

fn directorio(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    let ruta = std::env::current_dir()
        .map_err(|e| Error::ejecucion(format!("no se pudo obtener el directorio: {}", e)))?;
    Ok(Value::Texto(ruta.to_string_lossy().to_string()))
}

fn plataforma(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(std::env::consts::OS.to_string()))
}
