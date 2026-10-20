//! Módulo `concurrencia`: tareas, contadores atómicos y utilidades de hilos.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::error::Error;
use crate::interprete::{verificar_aridad, Interprete};
use crate::modulos::crear_modulo;
use crate::value::Value;

pub fn modulo() -> Value {
    crear_modulo(
        "concurrencia",
        vec![
            ("tarea", nativa("concurrencia.tarea", 1, usize::MAX, tarea)),
            (
                "esperar_todas",
                nativa("concurrencia.esperar_todas", 1, 1, esperar_todas),
            ),
            ("paralelo", nativa("concurrencia.paralelo", 1, 1, paralelo)),
            ("contador", nativa("concurrencia.contador", 0, 1, contador)),
            (
                "incrementar",
                nativa("concurrencia.incrementar", 1, 2, incrementar),
            ),
            ("valor", nativa("concurrencia.valor", 1, 1, valor)),
            (
                "reiniciar",
                nativa("concurrencia.reiniciar", 1, 2, reiniciar),
            ),
            ("dormir", nativa("concurrencia.dormir", 1, 1, dormir)),
            ("hilos", nativa("concurrencia.hilos", 0, 0, hilos)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn tarea(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let callee = args[0].clone();
    if !callee.es_llamable() {
        return Err(Error::ejecucion(format!(
            "'concurrencia.tarea' espera una función y recibió {}",
            callee.tipo_nombre()
        )));
    }
    interprete.crear_tarea_llamada(callee, args[1..].to_vec())
}

fn esperar_todas(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let elementos = match &args[0] {
        Value::Lista(lista) => lista.lock().unwrap().clone(),
        otro => {
            return Err(Error::ejecucion(format!(
                "'concurrencia.esperar_todas' espera una lista y recibió {}",
                otro.tipo_nombre()
            )))
        }
    };

    let mut resultados = Vec::with_capacity(elementos.len());
    for elemento in elementos {
        match elemento {
            Value::Tarea(tarea) => resultados.push(tarea.esperar()?),
            otro => resultados.push(otro),
        }
    }
    Ok(Value::lista(resultados))
}

fn paralelo(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let elementos = match &args[0] {
        Value::Lista(lista) => lista.lock().unwrap().clone(),
        otro => {
            return Err(Error::ejecucion(format!(
                "'concurrencia.paralelo' espera una lista de funciones y recibió {}",
                otro.tipo_nombre()
            )))
        }
    };

    let mut tareas = Vec::with_capacity(elementos.len());
    for elemento in elementos {
        if !elemento.es_llamable() {
            return Err(Error::ejecucion(
                "'concurrencia.paralelo' solo admite funciones",
            ));
        }
        tareas.push(interprete.crear_tarea_llamada(elemento, Vec::new())?);
    }

    esperar_todas(interprete, vec![Value::lista(tareas)])
}

fn contador(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let inicial = match args.first() {
        Some(valor) => valor
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("el contador inicial debe ser un entero"))?,
        None => 0,
    };
    Ok(Value::Contador(Arc::new(AtomicI64::new(inicial))))
}

fn como_contador(valor: &Value) -> Result<&Arc<AtomicI64>, Error> {
    match valor {
        Value::Contador(contador) => Ok(contador),
        otro => Err(Error::ejecucion(format!(
            "se esperaba un contador y se recibió {}",
            otro.tipo_nombre()
        ))),
    }
}

fn incrementar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let contador = como_contador(&args[0])?;
    let cantidad = match args.get(1) {
        Some(valor) => valor
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("la cantidad debe ser un entero"))?,
        None => 1,
    };
    let nuevo = contador.fetch_add(cantidad, Ordering::SeqCst) + cantidad;
    Ok(Value::Entero(nuevo))
}

fn valor(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    verificar_aridad("concurrencia.valor", 1, 1, &args)?;
    let contador = como_contador(&args[0])?;
    Ok(Value::Entero(contador.load(Ordering::SeqCst)))
}

fn reiniciar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let contador = como_contador(&args[0])?;
    let nuevo = match args.get(1) {
        Some(valor) => valor
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("el nuevo valor debe ser un entero"))?,
        None => 0,
    };
    contador.store(nuevo, Ordering::SeqCst);
    Ok(Value::Entero(nuevo))
}

fn dormir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    verificar_aridad("concurrencia.dormir", 1, 1, &args)?;
    let ms = args[0]
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("'dormir' espera milisegundos enteros"))?;
    if ms < 0 {
        return Err(Error::ejecucion(
            "'dormir' no admite una cantidad negativa de milisegundos",
        ));
    }
    std::thread::sleep(Duration::from_millis(ms as u64));
    Ok(Value::Nulo)
}

fn hilos(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    let hilos = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    Ok(Value::Entero(hilos as i64))
}
