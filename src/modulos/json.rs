//! Módulo `json`: parse y stringify sin dependencias externas.

use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::{MapaValores, Value};

pub fn modulo() -> Value {
    crear_modulo(
        "json",
        vec![
            ("parse", nativa("json.parse", 1, 1, parse_nativo)),
            (
                "stringify",
                nativa("json.stringify", 1, 2, stringify_nativo),
            ),
            ("valido", nativa("json.valido", 1, 1, valido_nativo)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn parse_nativo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let texto = match &args[0] {
        Value::Texto(t) => t.clone(),
        otro => {
            return Err(Error::ejecucion(format!(
                "'json.parse' espera un texto y recibió {}",
                otro.tipo_nombre()
            )))
        }
    };
    parse(&texto).map_err(Error::ejecucion)
}

fn stringify_nativo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let indentar = args
        .get(1)
        .map(crate::value::valor_verdadero)
        .unwrap_or(false);
    let texto = stringify(&args[0], indentar).map_err(Error::ejecucion)?;
    Ok(Value::Texto(texto))
}

fn valido_nativo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let texto = match &args[0] {
        Value::Texto(t) => t.clone(),
        _ => return Ok(Value::Logico(false)),
    };
    Ok(Value::Logico(parse(&texto).is_ok()))
}

// ----------------------------------------------------------------------
// Parser JSON
// ----------------------------------------------------------------------

pub fn parse(texto: &str) -> Result<Value, String> {
    let mut parser = ParserJson {
        caracteres: texto.chars().collect(),
        pos: 0,
        profundidad: 0,
    };
    parser.saltar_espacios();
    let valor = parser.valor()?;
    parser.saltar_espacios();
    if parser.pos != parser.caracteres.len() {
        return Err(format!(
            "texto JSON inválido: contenido extra en la posición {}",
            parser.pos
        ));
    }
    Ok(valor)
}

struct ParserJson {
    caracteres: Vec<char>,
    pos: usize,
    profundidad: usize,
}

impl ParserJson {
    fn entrar(&mut self) -> Result<(), String> {
        self.profundidad += 1;
        if self.profundidad > 256 {
            return Err("texto JSON demasiado anidado (máximo 256 niveles)".to_string());
        }
        Ok(())
    }

    fn salir(&mut self) {
        self.profundidad = self.profundidad.saturating_sub(1);
    }
    fn actual(&self) -> Option<char> {
        self.caracteres.get(self.pos).copied()
    }

    fn avanzar(&mut self) -> Option<char> {
        let c = self.actual();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn saltar_espacios(&mut self) {
        while let Some(c) = self.actual() {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn valor(&mut self) -> Result<Value, String> {
        self.saltar_espacios();
        match self.actual() {
            Some('{') => {
                self.entrar()?;
                let valor = self.objeto();
                self.salir();
                valor
            }
            Some('[') => {
                self.entrar()?;
                let valor = self.lista();
                self.salir();
                valor
            }
            Some('"') => Ok(Value::Texto(self.texto()?)),
            Some('t') => {
                self.literal("true")?;
                Ok(Value::Logico(true))
            }
            Some('f') => {
                self.literal("false")?;
                Ok(Value::Logico(false))
            }
            Some('n') => {
                self.literal("null")?;
                Ok(Value::Nulo)
            }
            Some(c) if c == '-' || c.is_ascii_digit() => self.numero(),
            Some(c) => Err(format!(
                "texto JSON inválido: carácter inesperado '{}' en la posición {}",
                c, self.pos
            )),
            None => Err("texto JSON vacío o incompleto".to_string()),
        }
    }

    fn literal(&mut self, esperado: &str) -> Result<(), String> {
        for c in esperado.chars() {
            if self.actual() != Some(c) {
                return Err(format!(
                    "texto JSON inválido: se esperaba '{}' en la posición {}",
                    esperado, self.pos
                ));
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn objeto(&mut self) -> Result<Value, String> {
        self.pos += 1; // {
        let mut mapa = MapaValores::default();
        self.saltar_espacios();
        if self.actual() == Some('}') {
            self.pos += 1;
            return Ok(Value::Diccionario(Arc::new(Mutex::new(mapa))));
        }
        loop {
            self.saltar_espacios();
            if self.actual() != Some('"') {
                return Err(format!(
                    "texto JSON inválido: se esperaba una clave entre comillas en la posición {}",
                    self.pos
                ));
            }
            let clave = self.texto()?;
            self.saltar_espacios();
            if self.actual() != Some(':') {
                return Err(format!(
                    "texto JSON inválido: se esperaba ':' en la posición {}",
                    self.pos
                ));
            }
            self.pos += 1;
            let valor = self.valor()?;
            mapa.insert(clave, valor);
            self.saltar_espacios();
            match self.actual() {
                Some(',') => {
                    self.pos += 1;
                }
                Some('}') => {
                    self.pos += 1;
                    break;
                }
                _ => {
                    return Err(format!(
                        "texto JSON inválido: se esperaba ',' o '}}' en la posición {}",
                        self.pos
                    ))
                }
            }
        }
        Ok(Value::Diccionario(Arc::new(Mutex::new(mapa))))
    }

    fn lista(&mut self) -> Result<Value, String> {
        self.pos += 1; // [
        let mut elementos = Vec::new();
        self.saltar_espacios();
        if self.actual() == Some(']') {
            self.pos += 1;
            return Ok(Value::lista(elementos));
        }
        loop {
            elementos.push(self.valor()?);
            self.saltar_espacios();
            match self.actual() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(']') => {
                    self.pos += 1;
                    break;
                }
                _ => {
                    return Err(format!(
                        "texto JSON inválido: se esperaba ',' o ']' en la posición {}",
                        self.pos
                    ))
                }
            }
        }
        Ok(Value::lista(elementos))
    }

    fn texto(&mut self) -> Result<String, String> {
        self.pos += 1; // comilla
        let mut salida = String::new();
        loop {
            match self.avanzar() {
                None => return Err("texto JSON inválido: texto sin cerrar".to_string()),
                Some('"') => break,
                Some('\\') => match self.avanzar() {
                    Some('"') => salida.push('"'),
                    Some('\\') => salida.push('\\'),
                    Some('/') => salida.push('/'),
                    Some('b') => salida.push('\u{0008}'),
                    Some('f') => salida.push('\u{000C}'),
                    Some('n') => salida.push('\n'),
                    Some('r') => salida.push('\r'),
                    Some('t') => salida.push('\t'),
                    Some('u') => {
                        let codigo = self.hex4()?;
                        if (0xD800..=0xDBFF).contains(&codigo) {
                            // Par sustituto
                            if self.actual() == Some('\\')
                                && self.caracteres.get(self.pos + 1) == Some(&'u')
                            {
                                self.pos += 2;
                                let bajo = self.hex4()?;
                                if (0xDC00..=0xDFFF).contains(&bajo) {
                                    let punto =
                                        0x10000 + ((codigo - 0xD800) << 10) + (bajo - 0xDC00);
                                    if let Some(c) = char::from_u32(punto) {
                                        salida.push(c);
                                        continue;
                                    }
                                }
                                return Err(
                                    "texto JSON inválido: par sustituto inválido".to_string()
                                );
                            }
                            return Err("texto JSON inválido: par sustituto incompleto".to_string());
                        }
                        let c = char::from_u32(codigo).ok_or_else(|| {
                            "texto JSON inválido: código unicode inválido".to_string()
                        })?;
                        salida.push(c);
                    }
                    Some(otro) => {
                        return Err(format!(
                            "texto JSON inválido: escape '\\{}' desconocido",
                            otro
                        ))
                    }
                    None => return Err("texto JSON inválido: escape incompleto".to_string()),
                },
                Some(c) => salida.push(c),
            }
        }
        Ok(salida)
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let mut valor = 0u32;
        for _ in 0..4 {
            let c = self
                .avanzar()
                .ok_or_else(|| "texto JSON inválido: unicode incompleto".to_string())?;
            let digito = c
                .to_digit(16)
                .ok_or_else(|| "texto JSON inválido: dígito unicode inválido".to_string())?;
            valor = valor * 16 + digito;
        }
        Ok(valor)
    }

    fn numero(&mut self) -> Result<Value, String> {
        let inicio = self.pos;
        if self.actual() == Some('-') {
            self.pos += 1;
        }
        while let Some(c) = self.actual() {
            if c.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        let mut es_decimal = false;
        if self.actual() == Some('.') {
            es_decimal = true;
            self.pos += 1;
            while let Some(c) = self.actual() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        if matches!(self.actual(), Some('e') | Some('E')) {
            es_decimal = true;
            self.pos += 1;
            if matches!(self.actual(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            while let Some(c) = self.actual() {
                if c.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        let texto: String = self.caracteres[inicio..self.pos].iter().collect();
        if texto.is_empty() || texto == "-" {
            return Err("texto JSON inválido: número incompleto".to_string());
        }
        if !es_decimal {
            if let Ok(n) = texto.parse::<i64>() {
                return Ok(Value::Entero(n));
            }
        }
        texto
            .parse::<f64>()
            .map(Value::Decimal)
            .map_err(|_| format!("texto JSON inválido: número '{}' inválido", texto))
    }
}

// ----------------------------------------------------------------------
// Serialización
// ----------------------------------------------------------------------

pub fn stringify(valor: &Value, indentar: bool) -> Result<String, String> {
    stringify_prof(valor, indentar, 0)
}

fn stringify_prof(valor: &Value, indentar: bool, nivel: usize) -> Result<String, String> {
    if nivel > 128 {
        return Err("estructura demasiado profunda o circular".to_string());
    }
    match valor {
        Value::Nulo => Ok("null".to_string()),
        Value::Logico(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        Value::Entero(n) => Ok(n.to_string()),
        Value::Decimal(n) => {
            if !n.is_finite() {
                Err("no se puede serializar NaN o infinito a JSON".to_string())
            } else if n.fract() == 0.0 && n.abs() < 1e16 {
                Ok(format!("{:.0}", n))
            } else {
                Ok(format!("{}", n))
            }
        }
        Value::Texto(t) => Ok(escapar_texto(t)),
        Value::Lista(lista) => {
            let elementos = lista.lock().unwrap();
            if elementos.is_empty() {
                return Ok("[]".to_string());
            }
            let mut partes = Vec::with_capacity(elementos.len());
            for elemento in elementos.iter() {
                partes.push(stringify_prof(elemento, indentar, nivel + 1)?);
            }
            if indentar {
                let sangria = "  ".repeat(nivel + 1);
                let cierre = "  ".repeat(nivel);
                Ok(format!(
                    "[\n{}{}\n{}]",
                    sangria,
                    partes.join(&format!(",\n{}", sangria)),
                    cierre
                ))
            } else {
                Ok(format!("[{}]", partes.join(",")))
            }
        }
        Value::Diccionario(dict) => {
            let mut claves: Vec<String> = dict.lock().unwrap().keys().cloned().collect();
            claves.sort();
            if claves.is_empty() {
                return Ok("{}".to_string());
            }
            let mut partes = Vec::with_capacity(claves.len());
            for clave in claves {
                let valor = dict
                    .lock()
                    .unwrap()
                    .get(&clave)
                    .cloned()
                    .unwrap_or(Value::Nulo);
                partes.push(format!(
                    "{}:{}",
                    escapar_texto(&clave),
                    stringify_prof(&valor, indentar, nivel + 1)?
                ));
            }
            if indentar {
                let sangria = "  ".repeat(nivel + 1);
                let cierre = "  ".repeat(nivel);
                Ok(format!(
                    "{{\n{}{}\n{}}}",
                    sangria,
                    partes.join(&format!(",\n{}", sangria)),
                    cierre
                ))
            } else {
                Ok(format!("{{{}}}", partes.join(",")))
            }
        }
        Value::Instancia(instancia) => {
            let instancia = instancia.lock().unwrap();
            let mut claves: Vec<String> = instancia.campos.keys().cloned().collect();
            claves.sort();
            let mut partes = Vec::with_capacity(claves.len());
            for clave in claves {
                let valor = instancia.campos.get(&clave).cloned().unwrap_or(Value::Nulo);
                partes.push(format!(
                    "{}:{}",
                    escapar_texto(&clave),
                    stringify_prof(&valor, indentar, nivel + 1)?
                ));
            }
            if indentar && !partes.is_empty() {
                let sangria = "  ".repeat(nivel + 1);
                let cierre = "  ".repeat(nivel);
                Ok(format!(
                    "{{\n{}{}\n{}}}",
                    sangria,
                    partes.join(&format!(",\n{}", sangria)),
                    cierre
                ))
            } else {
                Ok(format!("{{{}}}", partes.join(",")))
            }
        }
        otro => Err(format!(
            "no se puede serializar un valor de tipo {} a JSON",
            otro.tipo_nombre()
        )),
    }
}

fn escapar_texto(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len() + 2);
    salida.push('"');
    for c in texto.chars() {
        match c {
            '"' => salida.push_str("\\\""),
            '\\' => salida.push_str("\\\\"),
            '\n' => salida.push_str("\\n"),
            '\r' => salida.push_str("\\r"),
            '\t' => salida.push_str("\\t"),
            '\u{0008}' => salida.push_str("\\b"),
            '\u{000C}' => salida.push_str("\\f"),
            c if (c as u32) < 0x20 => salida.push_str(&format!("\\u{:04x}", c as u32)),
            c => salida.push(c),
        }
    }
    salida.push('"');
    salida
}
