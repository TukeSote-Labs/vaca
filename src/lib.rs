//! Vaca: lenguaje de programación en español para servidores.

pub mod analizador;
pub mod ast;
pub mod bytecode;
pub mod compilador;
pub mod error;
pub mod interprete;
pub mod lexer;
pub mod modulos;
pub mod parser;
pub mod rapido;
pub mod token;
pub mod value;

use std::fs;
use std::path::Path;

pub use error::Error;
pub use interprete::Interprete;
pub use parser::Parser;
pub use value::Value;

const PILA_EJECUCION: usize = 32 * 1024 * 1024;

/// Ejecuta código Vaca y devuelve todo lo que se imprimió por consola.
///
/// La ejecución ocurre en un hilo con una pila amplia para que la
/// recursividad del intérprete no desborde la pila del hilo llamador.
pub fn ejecutar_fuente(fuente: &str) -> Result<String, Error> {
    let codigo = fuente.to_string();
    ejecutar_en_hilo(move || {
        let mut interprete = Interprete::con_captura();
        // El hilo de ejecución tiene 32 MiB de pila.
        interprete.max_profundidad = 512;
        interprete.ejecutar_fuente(&codigo)?;
        Ok(interprete.tomar_salida())
    })
}

/// Ejecuta un archivo `.vaca`. El directorio del archivo se usa como base
/// para imports y rutas relativas.
pub fn ejecutar_archivo(ruta: impl AsRef<Path>) -> Result<String, Error> {
    let ruta = ruta.as_ref().to_path_buf();
    ejecutar_en_hilo(move || {
        let fuente = fs::read_to_string(&ruta).map_err(|e| {
            Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e))
        })?;
        let mut interprete = Interprete::con_captura();
        interprete.max_profundidad = 512;
        if let Some(padre) = ruta.parent() {
            if !padre.as_os_str().is_empty() {
                interprete.ruta_base =
                    fs::canonicalize(padre).unwrap_or_else(|_| padre.to_path_buf());
            }
        }
        interprete.ejecutar_fuente(&fuente)?;
        Ok(interprete.tomar_salida())
    })
}

fn ejecutar_en_hilo<F>(tarea: F) -> Result<String, Error>
where
    F: FnOnce() -> Result<String, Error> + Send + 'static,
{
    let manejador = std::thread::Builder::new()
        .stack_size(PILA_EJECUCION)
        .spawn(move || tarea().map_err(|error| error.to_string()))
        .map_err(|e| Error::ejecucion(format!("no se pudo crear el hilo de ejecución: {}", e)))?;
    match manejador.join() {
        Ok(Ok(salida)) => Ok(salida),
        Ok(Err(mensaje)) => Err(Error::Reportado(mensaje)),
        Err(_) => Err(Error::ejecucion(
            "el programa terminó de forma anormal (posible desbordamiento de pila)",
        )),
    }
}
