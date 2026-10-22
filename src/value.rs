//! Valores en tiempo de ejecución.

use std::any::Any;
use std::cmp::Ordering;
use std::fmt;
use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use crate::ast::{Parametro, Stmt};
use crate::bytecode::Chunk;
use crate::error::Error;
use crate::interprete::{Entorno, Interprete};
use crate::rapido::MapaRapido;

pub type NativaFn = fn(&mut Interprete, Vec<Value>) -> Result<Value, Error>;

/// Despachador de métodos de un objeto nativo opaco (conexiones, etc.).
pub type DespachadorNativo =
    fn(&mut Interprete, &Arc<ObjetoNativo>, &str, Vec<Value>) -> Result<Value, Error>;

/// Valor opaco creado por un módulo nativo.
///
/// El intérprete no conoce su funcionamiento: solo expone `tipo` como
/// propiedad y delega los métodos en `despachar`. Así se pueden añadir
/// conexiones, cursores u otros recursos sin tocar el lenguaje.
pub struct ObjetoNativo {
    pub tipo: String,
    pub datos: Arc<dyn Any + Send + Sync>,
    pub despachar: DespachadorNativo,
}

impl ObjetoNativo {
    pub fn nuevo<T>(tipo: &str, datos: T, despachar: DespachadorNativo) -> Arc<ObjetoNativo>
    where
        T: Any + Send + Sync,
    {
        Arc::new(ObjetoNativo {
            tipo: tipo.to_string(),
            datos: Arc::new(datos),
            despachar,
        })
    }

    pub fn datos<T: Any>(&self) -> Option<&T> {
        self.datos.downcast_ref::<T>()
    }
}

/// Mapa de claves de texto usado por diccionarios, instancias y módulos.
pub type MapaValores = MapaRapido<String, Value>;

pub struct Funcion {
    pub nombre: String,
    pub parametros: Vec<Parametro>,
    pub cuerpo: Vec<Stmt>,
    pub entorno: Arc<Mutex<Entorno>>,
    pub es_async: bool,
    /// Bytecode compilado (se calcula la primera vez que se llama).
    pub compilado: Arc<OnceLock<Option<Arc<Chunk>>>>,
}

pub struct Clase {
    pub nombre: String,
    pub padre: Option<Arc<Clase>>,
    pub metodos: MapaRapido<String, Arc<Funcion>>,
}

pub struct Instancia {
    pub clase: Arc<Clase>,
    pub campos: MapaValores,
}

pub struct Modulo {
    pub nombre: String,
    pub miembros: MapaValores,
}

pub struct Nativa {
    pub nombre: String,
    pub min_args: usize,
    pub max_args: usize,
    pub funcion: NativaFn,
}

/// Tarea concurrente creada por una función `async`.
pub struct Tarea {
    handle: Mutex<Option<JoinHandle<Result<Value, Error>>>>,
    resultado: Mutex<Option<Result<Value, Error>>>,
    espera: Mutex<()>,
}

impl Tarea {
    pub fn nueva(handle: JoinHandle<Result<Value, Error>>) -> Self {
        Tarea {
            handle: Mutex::new(Some(handle)),
            resultado: Mutex::new(None),
            espera: Mutex::new(()),
        }
    }

    /// Espera a que la tarea termine. Se puede llamar varias veces; el
    /// resultado queda cacheado.
    pub fn esperar(&self) -> Result<Value, Error> {
        if let Some(resultado) = self.resultado.lock().unwrap().clone() {
            return resultado;
        }

        let _guardia = self.espera.lock().unwrap();
        if let Some(resultado) = self.resultado.lock().unwrap().clone() {
            return resultado;
        }

        let handle = self.handle.lock().unwrap().take();
        let resultado = match handle {
            Some(handle) => handle
                .join()
                .unwrap_or_else(|_| Err(Error::ejecucion("la tarea terminó de forma anormal"))),
            None => Err(Error::ejecucion(
                "la tarea no tiene un resultado disponible",
            )),
        };

        *self.resultado.lock().unwrap() = Some(resultado.clone());
        resultado
    }
}

#[derive(Clone)]
pub enum Value {
    Entero(i64),
    Decimal(f64),
    Texto(String),
    Logico(bool),
    Nulo,
    Lista(Arc<Mutex<Vec<Value>>>),
    Diccionario(Arc<Mutex<MapaValores>>),
    Funcion(Arc<Funcion>),
    Nativa(Arc<Nativa>),
    Clase(Arc<Clase>),
    Instancia(Arc<Mutex<Instancia>>),
    Modulo(Arc<Modulo>),
    /// Método nativo pendiente de llamada, por ejemplo `lista.agregar`.
    MetodoNativo(Arc<(Value, String)>),
    /// Tarea concurrente creada por `async fn`.
    Tarea(Arc<Tarea>),
    /// Contador atómico seguro para múltiples hilos.
    Contador(Arc<AtomicI64>),
    /// Recurso opaco de un módulo nativo (por ejemplo, una conexión SQL).
    Nativo(Arc<ObjetoNativo>),
}

impl Value {
    pub fn lista(elementos: Vec<Value>) -> Value {
        Value::Lista(Arc::new(Mutex::new(elementos)))
    }

    pub fn diccionario(pares: Vec<(String, Value)>) -> Value {
        let mut mapa = MapaValores::default();
        for (clave, valor) in pares {
            mapa.insert(clave, valor);
        }
        Value::Diccionario(Arc::new(Mutex::new(mapa)))
    }

    pub fn nativa(nombre: &str, min_args: usize, max_args: usize, funcion: NativaFn) -> Value {
        Value::Nativa(Arc::new(Nativa {
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
            Value::Tarea(_) => "Tarea",
            Value::Contador(_) => "Contador",
            Value::Nativo(_) => "Objeto",
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
                let elementos: Vec<Value> = lista.lock().unwrap().clone();
                let elementos: Vec<String> =
                    elementos.iter().map(|v| v.repr(profundidad + 1)).collect();
                format!("[{}]", elementos.join(", "))
            }
            Value::Diccionario(dict) => {
                if profundidad > 50 {
                    return "{...}".to_string();
                }
                let mapa: MapaValores = dict.lock().unwrap().clone();
                let mut claves: Vec<String> = mapa.keys().cloned().collect();
                claves.sort();
                let pares: Vec<String> = claves
                    .into_iter()
                    .map(|k| {
                        let valor = mapa.get(&k).cloned().unwrap_or(Value::Nulo);
                        format!("{:?}: {}", k, valor.repr(profundidad + 1))
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
            Value::Instancia(i) => format!("<{}>", i.lock().unwrap().clase.nombre),
            Value::Modulo(m) => format!("<módulo {}>", m.nombre),
            Value::MetodoNativo(_) => "<método nativo>".to_string(),
            Value::Tarea(_) => "<tarea>".to_string(),
            Value::Contador(c) => c.load(AtomicOrdering::SeqCst).to_string(),
            Value::Nativo(objeto) => format!("<objeto {}>", objeto.tipo),
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
        Value::Lista(l) => !l.lock().unwrap().is_empty(),
        Value::Diccionario(d) => !d.lock().unwrap().is_empty(),
        Value::Tarea(_) => true,
        Value::Contador(c) => c.load(AtomicOrdering::SeqCst) != 0,
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
            if Arc::ptr_eq(x, y) {
                return true;
            }
            let x = x.lock().unwrap().clone();
            let y = y.lock().unwrap().clone();
            x.len() == y.len()
                && x.iter()
                    .zip(y.iter())
                    .all(|(a, b)| iguales_profundidad(a, b, profundidad + 1))
        }
        (Value::Diccionario(x), Value::Diccionario(y)) => {
            if Arc::ptr_eq(x, y) {
                return true;
            }
            let x = x.lock().unwrap().clone();
            let y = y.lock().unwrap().clone();
            x.len() == y.len()
                && x.iter().all(|(clave, valor)| {
                    y.get(clave)
                        .is_some_and(|otro| iguales_profundidad(valor, otro, profundidad + 1))
                })
        }
        (Value::Instancia(x), Value::Instancia(y)) => Arc::ptr_eq(x, y),
        (Value::Clase(x), Value::Clase(y)) => Arc::ptr_eq(x, y),
        (Value::Funcion(x), Value::Funcion(y)) => Arc::ptr_eq(x, y),
        (Value::Nativa(x), Value::Nativa(y)) => Arc::ptr_eq(x, y),
        (Value::Modulo(x), Value::Modulo(y)) => Arc::ptr_eq(x, y),
        (Value::Tarea(x), Value::Tarea(y)) => Arc::ptr_eq(x, y),
        (Value::Contador(x), Value::Contador(y)) => Arc::ptr_eq(x, y),
        (Value::Nativo(x), Value::Nativo(y)) => Arc::ptr_eq(x, y),
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
