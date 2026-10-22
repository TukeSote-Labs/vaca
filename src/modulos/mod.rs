//! Módulos nativos de Vaca.

pub mod archivo;
pub mod bd;
pub mod cadenas;
pub mod concurrencia;
pub mod datos;
pub mod http;
pub mod json;
pub mod mate;
pub mod sistema;
pub mod tiempo;

use std::sync::Arc;

use crate::value::{MapaValores, Modulo, NativaFn, Value};

pub fn nativa(nombre: &str, min: usize, max: usize, funcion: NativaFn) -> Value {
    Value::nativa(nombre, min, max, funcion)
}

pub fn crear_modulo(nombre: &str, miembros: Vec<(&str, Value)>) -> Value {
    let mut mapa = MapaValores::default();
    for (clave, valor) in miembros {
        mapa.insert(clave.to_string(), valor);
    }
    Value::Modulo(Arc::new(Modulo {
        nombre: nombre.to_string(),
        miembros: mapa,
    }))
}

/// Devuelve un módulo nativo por nombre, o `None` si no existe.
pub fn obtener_modulo(nombre: &str) -> Option<Value> {
    match nombre {
        "json" => Some(json::modulo()),
        "mate" | "matematica" | "matemática" => Some(mate::modulo()),
        "tiempo" => Some(tiempo::modulo()),
        "sistema" => Some(sistema::modulo()),
        "archivo" | "fs" => Some(archivo::modulo()),
        "datos" => Some(datos::modulo()),
        "http" => Some(http::modulo()),
        "cadenas" | "texto" => Some(cadenas::modulo()),
        "concurrencia" | "concurrente" => Some(concurrencia::modulo()),
        "bd" | "db" | "base_datos" | "basedatos" => Some(bd::modulo()),
        _ => None,
    }
}
