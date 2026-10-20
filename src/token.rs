//! Tokens del lenguaje Vaca.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenTipo {
    // Literales
    Entero(i64),
    Decimal(f64),
    Texto(String),
    TextoInterpolado(String),
    Identificador(String),

    // Palabras clave
    Si,
    Sino,
    Mientras,
    Para,
    Hasta,
    Paso,
    En,
    Fn,
    Metodo,
    Funcion,
    Retornar,
    Devolver,
    Clase,
    Nuevo,
    Este,
    Verdadero,
    Falso,
    Nulo,
    Y,
    O,
    No,
    Importar,
    Como,
    Romper,
    Continuar,
    Lanzar,
    Intentar,
    Atrapar,
    Async,
    Esperar,
    Constante,

    // Operadores y símbolos
    Mas,
    Menos,
    Asterisco,
    Diagonal,
    Porcentaje,
    Igual,
    IgualIgual,
    Distinto,
    Menor,
    Mayor,
    MenorIgual,
    MayorIgual,
    MasIgual,
    MenosIgual,
    AsteriscoIgual,
    DiagonalIgual,
    ParentesisIzq,
    ParentesisDer,
    LlaveIzq,
    LlaveDer,
    CorcheteIzq,
    CorcheteDer,
    Coma,
    DosPuntos,
    Punto,
    PuntoYComa,

    Fin,
}

impl TokenTipo {
    /// Descripción legible usada en los mensajes de error del parser.
    pub fn descripcion(&self) -> String {
        match self {
            TokenTipo::Entero(n) => format!("el número {}", n),
            TokenTipo::Decimal(n) => format!("el número {}", n),
            TokenTipo::Texto(_) => "un texto".to_string(),
            TokenTipo::TextoInterpolado(_) => "un texto interpolado".to_string(),
            TokenTipo::Identificador(n) => format!("el identificador '{}'", n),
            TokenTipo::Fin => "el final del archivo".to_string(),
            otro => format!("'{}'", otro.simbolo()),
        }
    }

    /// Indica si el token es una palabra clave (no un símbolo).
    pub fn es_palabra(&self) -> bool {
        matches!(
            self,
            TokenTipo::Si
                | TokenTipo::Sino
                | TokenTipo::Mientras
                | TokenTipo::Para
                | TokenTipo::Hasta
                | TokenTipo::Paso
                | TokenTipo::En
                | TokenTipo::Fn
                | TokenTipo::Metodo
                | TokenTipo::Funcion
                | TokenTipo::Retornar
                | TokenTipo::Devolver
                | TokenTipo::Clase
                | TokenTipo::Nuevo
                | TokenTipo::Este
                | TokenTipo::Verdadero
                | TokenTipo::Falso
                | TokenTipo::Nulo
                | TokenTipo::Y
                | TokenTipo::O
                | TokenTipo::No
                | TokenTipo::Importar
                | TokenTipo::Como
                | TokenTipo::Romper
                | TokenTipo::Continuar
                | TokenTipo::Lanzar
                | TokenTipo::Intentar
                | TokenTipo::Atrapar
                | TokenTipo::Async
                | TokenTipo::Esperar
                | TokenTipo::Constante
        )
    }

    pub fn simbolo(&self) -> &'static str {
        match self {
            TokenTipo::Si => "si",
            TokenTipo::Sino => "sino",
            TokenTipo::Mientras => "mientras",
            TokenTipo::Para => "para",
            TokenTipo::Hasta => "hasta",
            TokenTipo::Paso => "paso",
            TokenTipo::En => "en",
            TokenTipo::Fn => "fn",
            TokenTipo::Metodo => "metodo",
            TokenTipo::Funcion => "funcion",
            TokenTipo::Retornar => "retornar",
            TokenTipo::Devolver => "devolver",
            TokenTipo::Clase => "clase",
            TokenTipo::Nuevo => "nuevo",
            TokenTipo::Este => "este",
            TokenTipo::Verdadero => "verdadero",
            TokenTipo::Falso => "falso",
            TokenTipo::Nulo => "nulo",
            TokenTipo::Y => "y",
            TokenTipo::O => "o",
            TokenTipo::No => "no",
            TokenTipo::Importar => "importar",
            TokenTipo::Como => "como",
            TokenTipo::Romper => "romper",
            TokenTipo::Continuar => "continuar",
            TokenTipo::Lanzar => "lanzar",
            TokenTipo::Intentar => "intentar",
            TokenTipo::Atrapar => "atrapar",
            TokenTipo::Async => "async",
            TokenTipo::Esperar => "esperar",
            TokenTipo::Constante => "constante",
            TokenTipo::Mas => "+",
            TokenTipo::Menos => "-",
            TokenTipo::Asterisco => "*",
            TokenTipo::Diagonal => "/",
            TokenTipo::Porcentaje => "%",
            TokenTipo::Igual => "=",
            TokenTipo::IgualIgual => "==",
            TokenTipo::Distinto => "!=",
            TokenTipo::Menor => "<",
            TokenTipo::Mayor => ">",
            TokenTipo::MenorIgual => "<=",
            TokenTipo::MayorIgual => ">=",
            TokenTipo::MasIgual => "+=",
            TokenTipo::MenosIgual => "-=",
            TokenTipo::AsteriscoIgual => "*=",
            TokenTipo::DiagonalIgual => "/=",
            TokenTipo::ParentesisIzq => "(",
            TokenTipo::ParentesisDer => ")",
            TokenTipo::LlaveIzq => "{",
            TokenTipo::LlaveDer => "}",
            TokenTipo::CorcheteIzq => "[",
            TokenTipo::CorcheteDer => "]",
            TokenTipo::Coma => ",",
            TokenTipo::DosPuntos => ":",
            TokenTipo::Punto => ".",
            TokenTipo::PuntoYComa => ";",
            TokenTipo::Fin => "fin",
            _ => "?",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tipo: TokenTipo,
    pub linea: usize,
    pub columna: usize,
}

impl Token {
    pub fn nuevo(tipo: TokenTipo, linea: usize, columna: usize) -> Self {
        Token {
            tipo,
            linea,
            columna,
        }
    }
}

impl fmt::Display for TokenTipo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.descripcion())
    }
}
