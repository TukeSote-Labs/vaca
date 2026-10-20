//! Módulo `mate`: matemáticas para servidores y análisis.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::Error;
use crate::interprete::{verificar_aridad, Interprete};
use crate::modulos::crear_modulo;
use crate::value::Value;

pub fn modulo() -> Value {
    crear_modulo(
        "mate",
        vec![
            ("PI", Value::Decimal(std::f64::consts::PI)),
            ("E", Value::Decimal(std::f64::consts::E)),
            ("promedio", nativa("mate.promedio", 1, usize::MAX, promedio)),
            ("raiz", nativa("mate.raiz", 1, 1, raiz)),
            ("potencia", nativa("mate.potencia", 2, 2, potencia)),
            ("redondear", nativa("mate.redondear", 1, 2, redondear)),
            ("piso", nativa("mate.piso", 1, 1, piso)),
            ("techo", nativa("mate.techo", 1, 1, techo)),
            ("absoluto", nativa("mate.absoluto", 1, 1, absoluto)),
            ("minimo", nativa("mate.minimo", 1, usize::MAX, minimo)),
            ("maximo", nativa("mate.maximo", 1, usize::MAX, maximo)),
            ("sumar", nativa("mate.sumar", 1, usize::MAX, sumar)),
            ("aleatorio", nativa("mate.aleatorio", 0, 0, aleatorio)),
            (
                "aleatorio_entero",
                nativa("mate.aleatorio_entero", 2, 2, aleatorio_entero),
            ),
            ("logaritmo", nativa("mate.logaritmo", 1, 2, logaritmo)),
            ("seno", nativa("mate.seno", 1, 1, seno)),
            ("coseno", nativa("mate.coseno", 1, 1, coseno)),
            ("tangente", nativa("mate.tangente", 1, 1, tangente)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn a_numero(valor: &Value, nombre: &str) -> Result<f64, Error> {
    valor.a_decimal().ok_or_else(|| {
        Error::ejecucion(format!(
            "'{}' espera números y recibió {}",
            nombre,
            valor.tipo_nombre()
        ))
    })
}

fn recolectar_numeros(args: &[Value], nombre: &str) -> Result<Vec<f64>, Error> {
    if args.len() == 1 {
        if let Value::Lista(lista) = &args[0] {
            return lista
                .lock()
                .unwrap()
                .iter()
                .map(|v| a_numero(v, nombre))
                .collect();
        }
    }
    args.iter().map(|v| a_numero(v, nombre)).collect()
}

fn promedio(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let numeros = recolectar_numeros(&args, "promedio")?;
    if numeros.is_empty() {
        return Ok(Value::Decimal(0.0));
    }
    let suma: f64 = numeros.iter().sum();
    Ok(Value::Decimal(suma / numeros.len() as f64))
}

fn raiz(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let x = a_numero(&args[0], "raiz")?;
    if x < 0.0 {
        return Err(Error::ejecucion(
            "'raiz' no admite números negativos (resultado complejo)",
        ));
    }
    Ok(Value::Decimal(x.sqrt()))
}

fn potencia(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let base = a_numero(&args[0], "potencia")?;
    let exponente = a_numero(&args[1], "potencia")?;
    Ok(Value::Decimal(base.powf(exponente)))
}

fn redondear(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let x = a_numero(&args[0], "redondear")?;
    let decimales = match args.get(1) {
        Some(v) => v
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("'redondear' espera un número de decimales entero"))?,
        None => 0,
    };
    let factor = 10f64.powi(decimales as i32);
    Ok(Value::Decimal((x * factor).round() / factor))
}

fn piso(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Decimal(a_numero(&args[0], "piso")?.floor()))
}

fn techo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Decimal(a_numero(&args[0], "techo")?.ceil()))
}

fn absoluto(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    match &args[0] {
        Value::Entero(n) => Ok(Value::Entero(n.abs())),
        Value::Decimal(n) => Ok(Value::Decimal(n.abs())),
        otro => Err(Error::ejecucion(format!(
            "'absoluto' espera un número y recibió {}",
            otro.tipo_nombre()
        ))),
    }
}

fn minimo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let numeros = recolectar_numeros(&args, "minimo")?;
    let minimo = numeros.iter().copied().fold(f64::INFINITY, f64::min);
    Ok(Value::Decimal(minimo))
}

fn maximo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let numeros = recolectar_numeros(&args, "maximo")?;
    let maximo = numeros.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok(Value::Decimal(maximo))
}

fn sumar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let numeros = recolectar_numeros(&args, "sumar")?;
    Ok(Value::Decimal(numeros.iter().sum()))
}

static ESTADO_ALEATORIO: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);

fn siguiente_aleatorio() -> u64 {
    let mut x = ESTADO_ALEATORIO.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    ESTADO_ALEATORIO.store(x, Ordering::Relaxed);
    x
}

fn aleatorio(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    verificar_aridad("aleatorio", 0, 0, &args)?;
    let valor = (siguiente_aleatorio() as f64) / (u64::MAX as f64);
    Ok(Value::Decimal(valor))
}

fn aleatorio_entero(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let minimo = args[0]
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("'aleatorio_entero' espera enteros"))?;
    let maximo = args[1]
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("'aleatorio_entero' espera enteros"))?;
    if maximo < minimo {
        return Err(Error::ejecucion(
            "'aleatorio_entero' necesita un máximo mayor o igual que el mínimo",
        ));
    }
    let rango = (maximo - minimo + 1) as u64;
    let valor = minimo + (siguiente_aleatorio() % rango) as i64;
    Ok(Value::Entero(valor))
}

fn logaritmo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let x = a_numero(&args[0], "logaritmo")?;
    let base = match args.get(1) {
        Some(v) => a_numero(v, "logaritmo")?,
        None => std::f64::consts::E,
    };
    Ok(Value::Decimal(x.log(base)))
}

fn seno(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Decimal(a_numero(&args[0], "seno")?.sin()))
}

fn coseno(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Decimal(a_numero(&args[0], "coseno")?.cos()))
}

fn tangente(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Decimal(a_numero(&args[0], "tangente")?.tan()))
}
