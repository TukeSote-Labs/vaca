//! Módulo `cadenas`: utilidades de texto funcionales.

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::Value;

pub fn modulo() -> Value {
    crear_modulo(
        "cadenas",
        vec![
            ("unir", nativa("cadenas.unir", 1, 2, unir)),
            ("mayusculas", nativa("cadenas.mayusculas", 1, 1, mayusculas)),
            ("minusculas", nativa("cadenas.minusculas", 1, 1, minusculas)),
            ("recortar", nativa("cadenas.recortar", 1, 1, recortar)),
            ("reemplazar", nativa("cadenas.reemplazar", 3, 3, reemplazar)),
            ("dividir", nativa("cadenas.dividir", 2, 2, dividir)),
            ("contiene", nativa("cadenas.contiene", 2, 2, contiene)),
            (
                "empieza_con",
                nativa("cadenas.empieza_con", 2, 2, empieza_con),
            ),
            (
                "termina_con",
                nativa("cadenas.termina_con", 2, 2, termina_con),
            ),
            ("repetir", nativa("cadenas.repetir", 2, 2, repetir)),
            (
                "rellenar_izquierda",
                nativa("cadenas.rellenar_izquierda", 2, 3, rellenar_izquierda),
            ),
            (
                "rellenar_derecha",
                nativa("cadenas.rellenar_derecha", 2, 3, rellenar_derecha),
            ),
            ("longitud", nativa("cadenas.longitud", 1, 1, longitud)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn unir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let lista = match &args[0] {
        Value::Lista(l) => l.borrow().clone(),
        otro => {
            return Err(Error::ejecucion(format!(
                "'cadenas.unir' espera una lista y recibió {}",
                otro.tipo_nombre()
            )))
        }
    };
    let separador = args.get(1).map(|v| v.a_texto()).unwrap_or_default();
    Ok(Value::Texto(
        lista
            .iter()
            .map(|v| v.a_texto())
            .collect::<Vec<_>>()
            .join(&separador),
    ))
}

fn mayusculas(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(args[0].a_texto().to_uppercase()))
}

fn minusculas(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(args[0].a_texto().to_lowercase()))
}

fn recortar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(args[0].a_texto().trim().to_string()))
}

fn reemplazar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(
        args[0]
            .a_texto()
            .replace(&args[1].a_texto(), &args[2].a_texto()),
    ))
}

fn dividir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let texto = args[0].a_texto();
    let separador = args[1].a_texto();
    let partes: Vec<Value> = if separador.is_empty() {
        texto.chars().map(|c| Value::Texto(c.to_string())).collect()
    } else {
        texto
            .split(&separador)
            .map(|p| Value::Texto(p.to_string()))
            .collect()
    };
    Ok(Value::lista(partes))
}

fn contiene(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Logico(
        args[0].a_texto().contains(&args[1].a_texto()),
    ))
}

fn empieza_con(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Logico(
        args[0].a_texto().starts_with(&args[1].a_texto()),
    ))
}

fn termina_con(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Logico(
        args[0].a_texto().ends_with(&args[1].a_texto()),
    ))
}

fn repetir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let veces = args[1].a_entero_exacto().unwrap_or(0).max(0) as usize;
    Ok(Value::Texto(args[0].a_texto().repeat(veces)))
}

fn rellenar_izquierda(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let texto = args[0].a_texto();
    let ancho = args[1].a_entero_exacto().unwrap_or(0).max(0) as usize;
    let relleno = args
        .get(2)
        .map(|v| v.a_texto())
        .unwrap_or_else(|| " ".to_string());
    Ok(Value::Texto(rellenar(&texto, ancho, &relleno, true)))
}

fn rellenar_derecha(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let texto = args[0].a_texto();
    let ancho = args[1].a_entero_exacto().unwrap_or(0).max(0) as usize;
    let relleno = args
        .get(2)
        .map(|v| v.a_texto())
        .unwrap_or_else(|| " ".to_string());
    Ok(Value::Texto(rellenar(&texto, ancho, &relleno, false)))
}

fn rellenar(texto: &str, ancho: usize, relleno: &str, izquierda: bool) -> String {
    let actual = texto.chars().count();
    if actual >= ancho || relleno.is_empty() {
        return texto.to_string();
    }
    let mut faltante = String::new();
    while faltante.chars().count() < ancho - actual {
        faltante.push_str(relleno);
    }
    let faltante: String = faltante.chars().take(ancho - actual).collect();
    if izquierda {
        format!("{}{}", faltante, texto)
    } else {
        format!("{}{}", texto, faltante)
    }
}

fn longitud(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Entero(args[0].a_texto().chars().count() as i64))
}
