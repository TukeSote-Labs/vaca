//! Valores en tiempo de ejecución.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::ast::{Parametro, Stmt};
use crate::error::Error;
use crate::interprete::{Entorno, Interprete};

pub type NativaFn = fn(&mut Interprete, Vec<Value>) -> Result<Value, Error>;

pub struct Funcion {
    pub nombre: String,
    pub parametros: Vec<Parametro>,
    pub cuerpo: Vec<Stmt>,
    pub entorno: Rc<RefCell<Entorno>>,
    pub es_async: bool,
}

pub struct Clase {
    pub nombre: String,
    pub padre: Option<Rc<Clase>>,
    pub metodos: HashMap<String, Rc<Funcion>>,
}

pub struct Instancia {
    pub clase: Rc<Clase>,
    pub campos: HashMap<String, Value>,
}

pub struct Modulo {
    pub nombre: String,
    pub miembros: HashMap<String, Value>,
}

pub struct Nativa {
    pub nombre: String,
    pub min_args: usize,
    pub max_args: usize,
    pub funcion: NativaFn,
}

#[derive(Clone)]
pub enum Value {
    Entero(i64),
    Decimal(f64),
    Texto(String),
    Logico(bool),
    Nulo,
    Lista(Rc<RefCell<Vec<Value>>>),
    Diccionario(Rc<RefCell<HashMap<String, Value>>>),
    Funcion(Rc<Funcion>),
    Nativa(Rc<Nativa>),
    Clase(Rc<Clase>),
    Instancia(Rc<RefCell<Instancia>>),
    Modulo(Rc<Modulo>),
    /// Método nativo pendiente de llamada, por ejemplo `lista.agregar`.
    MetodoNativo(Rc<(Value, String)>),
}

impl Value {
    pub fn lista(elementos: Vec<Value>) -> Value {
        Value::Lista(Rc::new(RefCell::new(elementos)))
    }

    pub fn diccionario(pares: Vec<(String, Value)>) -> Value {
        let mut mapa = HashMap::new();
        for (clave, valor) in pares {
            mapa.insert(clave, valor);
        }
        Value::Diccionario(Rc::new(RefCell::new(mapa)))
    }

    pub fn nativa(nombre: &str, min_args: usize, max_args: usize, funcion: NativaFn) -> Value {
        Value::Nativa(Rc::new(Nativa {
            nombre: nombre.to_string(),
            min_args,
            max_args,
            funcion,
        }))
    }

    pub fn tipo_nombre(&self) -> &'static str {
        match self {
            Value::Entero(_) => "Entero",
            Value::Decimal(_) => "Decimal",
            Value::Texto(_) => "Texto",
            Value::Logico(_) => "Logico",
            Value::Nulo => "Nulo",
            Value::Lista(_) => "Lista",
            Value::Diccionario(_) => "Diccionario",
            Value::Funcion(_) => "Funcion",
            Value::Nativa(_) => "Funcion",
            Value::Clase(_) => "Clase",
            Value::Instancia(_) => "Instancia",
            Value::Modulo(_) => "Modulo",
            Value::MetodoNativo(_) => "Funcion",
        }
    }

    pub fn a_texto(&self) -> String {
        self.representacion(0)
    }

    pub fn a_repr(&self) -> String {
        self.repr(0)
    }

    fn representacion(&self, profundidad: usize) -> String {
        match self {
            Value::Entero(n) => n.to_string(),
            Value::Decimal(n) => formatear_decimal(*n),
            Value::Texto(t) => t.clone(),
            Value::Logico(b) => if *b { "verdadero" } else { "falso" }.to_string(),
            Value::Nulo => "nulo".to_string(),
            Value::Lista(lista) => {
                if profundidad > 50 {
                    return "[...]".to_string();
                }
                let elementos: Vec<String> = lista
                    .borrow()
                    .iter()
                    .map(|v| v.repr(profundidad + 1))
                    .collect();
                format!("[{}]", elementos.join(", "))
            }
            Value::Diccionario(dict) => {
                if profundidad > 50 {
                    return "{...}".to_string();
                }
                let mut claves: Vec<String> = dict.borrow().keys().cloned().collect();
                claves.sort();
                let pares: Vec<String> = claves
                    .into_iter()
                    .filter_map(|k| {
                        dict.borrow()
                            .get(&k)
                            .map(|v| format!("{:?}: {}", k, v.repr(profundidad + 1)))
                    })
                    .collect();
                format!("{{{}}}", pares.join(", "))
            }
            Value::Funcion(f) => {
                if f.nombre.is_empty() {
                    "<fn anónima>".to_string()
                } else {
                    format!("<fn {}>", f.nombre)
                }
            }
            Value::Nativa(n) => format!("<fn nativa {}>", n.nombre),
            Value::Clase(c) => format!("<clase {}>", c.nombre),
            Value::Instancia(i) => format!("<{}>", i.borrow().clase.nombre),
            Value::Modulo(m) => format!("<módulo {}>", m.nombre),
            Value::MetodoNativo(_) => "<método nativo>".to_string(),
        }
    }

    fn repr(&self, profundidad: usize) -> String {
        match self {
            Value::Texto(t) => format!("{:?}", t),
            _ => self.representacion(profundidad),
        }
    }

    pub fn es_numero(&self) -> bool {
        matches!(self, Value::Entero(_) | Value::Decimal(_))
    }

    pub fn a_decimal(&self) -> Option<f64> {
        match self {
            Value::Entero(n) => Some(*n as f64),
            Value::Decimal(n) => Some(*n),
            _ => None,
        }
    }

    pub fn a_entero_exacto(&self) -> Option<i64> {
        match self {
            Value::Entero(n) => Some(*n),
            Value::Decimal(n) if n.fract() == 0.0 && n.is_finite() => Some(*n as i64),
            _ => None,
        }
    }

    pub fn es_llamable(&self) -> bool {
        matches!(
            self,
            Value::Funcion(_) | Value::Nativa(_) | Value::Clase(_) | Value::MetodoNativo(_)
        )
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.a_repr())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.a_texto())
    }
}

pub fn formatear_decimal(n: f64) -> String {
    if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        if n.is_sign_negative() {
            "-infinito".to_string()
        } else {
            "infinito".to_string()
        }
    } else if n.fract() == 0.0 && n.abs() < 1e16 {
        format!("{:.0}", n)
    } else {
        // Evita ruido de coma flotante en la salida normal (108.89999999999999).
        let mut texto = format!("{:.12}", n);
        while texto.contains('.') && texto.ends_with('0') {
            texto.pop();
        }
        if texto.ends_with('.') {
            texto.pop();
        }
        if texto == "0" || texto == "-0" {
            format!("{:e}", n)
        } else {
            texto
        }
    }
}

pub fn valor_verdadero(valor: &Value) -> bool {
    match valor {
        Value::Logico(b) => *b,
        Value::Nulo => false,
        Value::Entero(n) => *n != 0,
        Value::Decimal(n) => *n != 0.0,
        Value::Texto(t) => !t.is_empty(),
        Value::Lista(l) => !l.borrow().is_empty(),
        Value::Diccionario(d) => !d.borrow().is_empty(),
        _ => true,
    }
}

pub fn valores_iguales(a: &Value, b: &Value) -> bool {
    iguales_profundidad(a, b, 0)
}

fn iguales_profundidad(a: &Value, b: &Value, profundidad: usize) -> bool {
    if profundidad > 100 {
        return false;
    }
    match (a, b) {
        (Value::Entero(x), Value::Entero(y)) => x == y,
        (Value::Entero(x), Value::Decimal(y)) => (*x as f64) == *y,
        (Value::Decimal(x), Value::Entero(y)) => *x == (*y as f64),
        (Value::Decimal(x), Value::Decimal(y)) => x == y,
        (Value::Texto(x), Value::Texto(y)) => x == y,
        (Value::Logico(x), Value::Logico(y)) => x == y,
        (Value::Nulo, Value::Nulo) => true,
        (Value::Lista(x), Value::Lista(y)) => {
            if Rc::ptr_eq(x, y) {
                return true;
            }
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|(a, b)| iguales_profundidad(a, b, profundidad + 1))
        }
        (Value::Diccionario(x), Value::Diccionario(y)) => {
            if Rc::ptr_eq(x, y) {
                return true;
            }
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len()
                && x.iter().all(|(clave, valor)| {
                    y.get(clave)
                        .is_some_and(|otro| iguales_profundidad(valor, otro, profundidad + 1))
                })
        }
        (Value::Instancia(x), Value::Instancia(y)) => Rc::ptr_eq(x, y),
        (Value::Clase(x), Value::Clase(y)) => Rc::ptr_eq(x, y),
        (Value::Funcion(x), Value::Funcion(y)) => Rc::ptr_eq(x, y),
        (Value::Nativa(x), Value::Nativa(y)) => Rc::ptr_eq(x, y),
        (Value::Modulo(x), Value::Modulo(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

/// Comparación de orden para números, textos, lógicos y nulo.
pub fn comparar_orden(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Entero(x), Value::Entero(y)) => Some(x.cmp(y)),
        (Value::Entero(x), Value::Decimal(y)) => (*x as f64).partial_cmp(y),
        (Value::Decimal(x), Value::Entero(y)) => x.partial_cmp(&(*y as f64)),
        (Value::Decimal(x), Value::Decimal(y)) => x.partial_cmp(y),
        (Value::Texto(x), Value::Texto(y)) => Some(x.cmp(y)),
        (Value::Logico(x), Value::Logico(y)) => Some(x.cmp(y)),
        (Value::Nulo, Value::Nulo) => Some(Ordering::Equal),
        (Value::Nulo, _) => Some(Ordering::Less),
        (_, Value::Nulo) => Some(Ordering::Greater),
        _ => None,
    }
}

pub fn clave_texto(valor: &Value) -> String {
    match valor {
        Value::Texto(t) => t.clone(),
        _ => valor.a_texto(),
    }
}
