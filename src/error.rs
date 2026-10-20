//! Errores del lenguaje Vaca.

use std::fmt;

use crate::value::Value;

#[derive(Debug, Clone)]
pub enum Error {
    Lexico {
        mensaje: String,
        linea: usize,
        columna: usize,
    },
    Sintactico {
        mensaje: String,
        linea: usize,
        columna: usize,
    },
    Ejecucion(String),
    /// Error producido por `lanzar`. Puede capturarse con `intentar`/`atrapar`.
    Lanzado(Value),
    /// Producido por `sistema.salir(codigo)`.
    Salir(i32),
    /// Error ya formateado, usado al cruzar límites de hilo.
    Reportado(String),
    /// Error detectado por el analizador semántico antes de ejecutar.
    Semantico(String),
}

impl Error {
    pub fn lexico(mensaje: impl Into<String>, linea: usize, columna: usize) -> Self {
        Error::Lexico {
            mensaje: mensaje.into(),
            linea,
            columna,
        }
    }

    pub fn sintactico(mensaje: impl Into<String>, linea: usize, columna: usize) -> Self {
        Error::Sintactico {
            mensaje: mensaje.into(),
            linea,
            columna,
        }
    }

    pub fn ejecucion(mensaje: impl Into<String>) -> Self {
        Error::Ejecucion(mensaje.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lexico {
                mensaje,
                linea,
                columna,
            } => write!(
                f,
                "error léxico en la línea {}, columna {}: {}",
                linea, columna, mensaje
            ),
            Error::Sintactico {
                mensaje,
                linea,
                columna,
            } => write!(
                f,
                "error de sintaxis en la línea {}, columna {}: {}",
                linea, columna, mensaje
            ),
            Error::Ejecucion(mensaje) => write!(f, "error de ejecución: {}", mensaje),
            Error::Lanzado(valor) => write!(f, "excepción lanzada: {}", valor.a_texto()),
            Error::Salir(codigo) => write!(f, "salida del programa con código {}", codigo),
            Error::Reportado(mensaje) => write!(f, "{}", mensaje),
            Error::Semantico(mensaje) => {
                write!(f, "error de compilación: {}", mensaje)
            }
        }
    }
}

impl std::error::Error for Error {}
