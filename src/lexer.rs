//! Analizador léxico de Vaca.

use crate::error::Error;
use crate::token::{Token, TokenTipo};

pub struct Lexer {
    caracteres: Vec<char>,
    pos: usize,
    linea: usize,
    columna: usize,
}

impl Lexer {
    pub fn nuevo(fuente: &str) -> Self {
        Lexer {
            caracteres: fuente.chars().collect(),
            pos: 0,
            linea: 1,
            columna: 1,
        }
    }

    /// Convierte todo el código fuente en una lista de tokens terminada en `Fin`.
    pub fn tokenizar(mut self) -> Result<Vec<Token>, Error> {
        let mut tokens = Vec::new();
        loop {
            self.saltar_espacios_y_comentarios()?;
            if self.pos >= self.caracteres.len() {
                tokens.push(Token::nuevo(TokenTipo::Fin, self.linea, self.columna));
                return Ok(tokens);
            }

            let linea = self.linea;
            let columna = self.columna;
            let c = self.caracteres[self.pos];

            // Texto interpolado: a"..." o a'...'
            if (c == 'a' || c == 'A') && self.ve(1).is_some_and(|s| s == '"' || s == '\'') {
                self.avanzar(); // a
                let cita = self.caracteres[self.pos];
                let contenido = self.leer_texto(cita, false)?;
                tokens.push(Token::nuevo(
                    TokenTipo::TextoInterpolado(contenido),
                    linea,
                    columna,
                ));
                continue;
            }

            if c == '"' || c == '\'' {
                let contenido = self.leer_texto(c, true)?;
                tokens.push(Token::nuevo(TokenTipo::Texto(contenido), linea, columna));
                continue;
            }

            if c.is_ascii_digit() {
                let tipo = self.leer_numero()?;
                tokens.push(Token::nuevo(tipo, linea, columna));
                continue;
            }

            if c == '_' || c.is_alphabetic() {
                let ident = self.leer_identificador();
                let tipo = palabra_clave(&ident).unwrap_or(TokenTipo::Identificador(ident));
                tokens.push(Token::nuevo(tipo, linea, columna));
                continue;
            }

            let tipo = match c {
                '+' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::MasIgual
                    } else {
                        TokenTipo::Mas
                    }
                }
                '-' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::MenosIgual
                    } else {
                        TokenTipo::Menos
                    }
                }
                '*' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::AsteriscoIgual
                    } else {
                        TokenTipo::Asterisco
                    }
                }
                '/' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::DiagonalIgual
                    } else {
                        TokenTipo::Diagonal
                    }
                }
                '%' => {
                    self.avanzar();
                    TokenTipo::Porcentaje
                }
                '=' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::IgualIgual
                    } else {
                        TokenTipo::Igual
                    }
                }
                '!' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::Distinto
                    } else {
                        return Err(Error::lexico(
                            "se esperaba '!='; el operador '!' no existe en Vaca",
                            linea,
                            columna,
                        ));
                    }
                }
                '<' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::MenorIgual
                    } else {
                        TokenTipo::Menor
                    }
                }
                '>' => {
                    self.avanzar();
                    if self.coincidir_char('=') {
                        TokenTipo::MayorIgual
                    } else {
                        TokenTipo::Mayor
                    }
                }
                '(' => {
                    self.avanzar();
                    TokenTipo::ParentesisIzq
                }
                ')' => {
                    self.avanzar();
                    TokenTipo::ParentesisDer
                }
                '{' => {
                    self.avanzar();
                    TokenTipo::LlaveIzq
                }
                '}' => {
                    self.avanzar();
                    TokenTipo::LlaveDer
                }
                '[' => {
                    self.avanzar();
                    TokenTipo::CorcheteIzq
                }
                ']' => {
                    self.avanzar();
                    TokenTipo::CorcheteDer
                }
                ',' => {
                    self.avanzar();
                    TokenTipo::Coma
                }
                ':' => {
                    self.avanzar();
                    TokenTipo::DosPuntos
                }
                '.' => {
                    self.avanzar();
                    TokenTipo::Punto
                }
                ';' => {
                    self.avanzar();
                    TokenTipo::PuntoYComa
                }
                otro => {
                    return Err(Error::lexico(
                        format!("carácter inesperado '{}'", otro),
                        linea,
                        columna,
                    ));
                }
            };
            tokens.push(Token::nuevo(tipo, linea, columna));
        }
    }

    fn ve(&self, desplazamiento: usize) -> Option<char> {
        self.caracteres.get(self.pos + desplazamiento).copied()
    }

    fn avanzar(&mut self) {
        if let Some(c) = self.caracteres.get(self.pos).copied() {
            self.pos += 1;
            if c == '\n' {
                self.linea += 1;
                self.columna = 1;
            } else {
                self.columna += 1;
            }
        }
    }

    fn coincidir_char(&mut self, esperado: char) -> bool {
        if self.ve(0) == Some(esperado) {
            self.avanzar();
            true
        } else {
            false
        }
    }

    fn saltar_espacios_y_comentarios(&mut self) -> Result<(), Error> {
        loop {
            while let Some(c) = self.ve(0) {
                if c.is_whitespace() {
                    self.avanzar();
                } else {
                    break;
                }
            }

            match (self.ve(0), self.ve(1)) {
                (Some('#'), _) => {
                    while let Some(c) = self.ve(0) {
                        if c == '\n' {
                            break;
                        }
                        self.avanzar();
                    }
                }
                (Some('/'), Some('/')) => {
                    while let Some(c) = self.ve(0) {
                        if c == '\n' {
                            break;
                        }
                        self.avanzar();
                    }
                }
                (Some('/'), Some('*')) => {
                    let linea = self.linea;
                    let columna = self.columna;
                    self.avanzar();
                    self.avanzar();
                    let mut profundidad = 1usize;
                    while profundidad > 0 {
                        match (self.ve(0), self.ve(1)) {
                            (Some('/'), Some('*')) => {
                                self.avanzar();
                                self.avanzar();
                                profundidad += 1;
                            }
                            (Some('*'), Some('/')) => {
                                self.avanzar();
                                self.avanzar();
                                profundidad -= 1;
                            }
                            (Some(_), _) => self.avanzar(),
                            (None, _) => {
                                return Err(Error::lexico(
                                    "comentario de bloque sin cerrar",
                                    linea,
                                    columna,
                                ))
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn leer_texto(&mut self, cita: char, procesar: bool) -> Result<String, Error> {
        let linea = self.linea;
        let columna = self.columna;
        self.avanzar(); // comilla de apertura
        let mut crudo = String::new();
        loop {
            match self.ve(0) {
                None => {
                    return Err(Error::lexico(
                        "texto sin cerrar (falta la comilla final)",
                        linea,
                        columna,
                    ))
                }
                Some(c) if c == cita => {
                    self.avanzar();
                    break;
                }
                Some('\\') => {
                    crudo.push('\\');
                    self.avanzar();
                    match self.ve(0) {
                        Some(siguiente) => {
                            crudo.push(siguiente);
                            self.avanzar();
                        }
                        None => {
                            return Err(Error::lexico(
                                "secuencia de escape incompleta",
                                linea,
                                columna,
                            ))
                        }
                    }
                }
                Some(c) => {
                    crudo.push(c);
                    self.avanzar();
                }
            }
        }
        if procesar {
            Ok(desescapar(&crudo))
        } else {
            Ok(crudo)
        }
    }

    fn leer_numero(&mut self) -> Result<TokenTipo, Error> {
        let linea = self.linea;
        let columna = self.columna;
        let mut texto = String::new();
        let mut es_decimal = false;

        while let Some(c) = self.ve(0) {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    texto.push(c);
                }
                self.avanzar();
            } else {
                break;
            }
        }

        if self.ve(0) == Some('.') && self.ve(1).is_some_and(|c| c.is_ascii_digit()) {
            es_decimal = true;
            texto.push('.');
            self.avanzar();
            while let Some(c) = self.ve(0) {
                if c.is_ascii_digit() || c == '_' {
                    if c != '_' {
                        texto.push(c);
                    }
                    self.avanzar();
                } else {
                    break;
                }
            }
        }

        if matches!(self.ve(0), Some('e') | Some('E')) {
            let mut copia = texto.clone();
            copia.push('e');
            es_decimal = true;
            self.avanzar();
            if matches!(self.ve(0), Some('+') | Some('-')) {
                copia.push(self.ve(0).unwrap());
                self.avanzar();
            }
            let mut tiene_digito = false;
            while let Some(c) = self.ve(0) {
                if c.is_ascii_digit() {
                    tiene_digito = true;
                    copia.push(c);
                    self.avanzar();
                } else {
                    break;
                }
            }
            if !tiene_digito {
                return Err(Error::lexico(
                    "exponente inválido en número decimal",
                    linea,
                    columna,
                ));
            }
            texto = copia;
        }

        if es_decimal {
            let valor = texto.parse::<f64>().map_err(|_| {
                Error::lexico(
                    format!("número decimal inválido '{}'", texto),
                    linea,
                    columna,
                )
            })?;
            Ok(TokenTipo::Decimal(valor))
        } else {
            let valor = texto.parse::<i64>().map_err(|_| {
                Error::lexico(
                    format!("número entero fuera de rango '{}'", texto),
                    linea,
                    columna,
                )
            })?;
            Ok(TokenTipo::Entero(valor))
        }
    }

    fn leer_identificador(&mut self) -> String {
        let mut texto = String::new();
        while let Some(c) = self.ve(0) {
            if c == '_' || c.is_alphanumeric() {
                texto.push(c);
                self.avanzar();
            } else {
                break;
            }
        }
        texto
    }
}

/// Procesa las secuencias de escape de un texto.
pub fn desescapar(crudo: &str) -> String {
    let mut salida = String::with_capacity(crudo.len());
    let mut chars = crudo.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            salida.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => salida.push('\n'),
            Some('t') => salida.push('\t'),
            Some('r') => salida.push('\r'),
            Some('0') => salida.push('\0'),
            Some('\\') => salida.push('\\'),
            Some('"') => salida.push('"'),
            Some('\'') => salida.push('\''),
            Some('{') => salida.push('{'),
            Some('}') => salida.push('}'),
            Some(otro) => {
                salida.push('\\');
                salida.push(otro);
            }
            None => salida.push('\\'),
        }
    }
    salida
}

fn palabra_clave(ident: &str) -> Option<TokenTipo> {
    Some(match ident {
        "si" => TokenTipo::Si,
        "sino" => TokenTipo::Sino,
        "mientras" => TokenTipo::Mientras,
        "para" => TokenTipo::Para,
        "hasta" => TokenTipo::Hasta,
        "paso" => TokenTipo::Paso,
        "en" => TokenTipo::En,
        "fn" => TokenTipo::Fn,
        "metodo" | "método" => TokenTipo::Metodo,
        "funcion" | "función" => TokenTipo::Funcion,
        "retornar" => TokenTipo::Retornar,
        "devolver" => TokenTipo::Devolver,
        "clase" => TokenTipo::Clase,
        "nuevo" => TokenTipo::Nuevo,
        "este" => TokenTipo::Este,
        "verdadero" => TokenTipo::Verdadero,
        "falso" => TokenTipo::Falso,
        "nulo" => TokenTipo::Nulo,
        "y" => TokenTipo::Y,
        "o" => TokenTipo::O,
        "no" => TokenTipo::No,
        "importar" => TokenTipo::Importar,
        "como" => TokenTipo::Como,
        "romper" => TokenTipo::Romper,
        "continuar" => TokenTipo::Continuar,
        "lanzar" => TokenTipo::Lanzar,
        "intentar" => TokenTipo::Intentar,
        "atrapar" => TokenTipo::Atrapar,
        "async" => TokenTipo::Async,
        "esperar" | "await" => TokenTipo::Esperar,
        "constante" | "const" => TokenTipo::Constante,
        _ => return None,
    })
}
