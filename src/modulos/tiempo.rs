//! Módulo `tiempo`: reloj, fechas y esperas.

use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::Value;

pub fn modulo() -> Value {
    crear_modulo(
        "tiempo",
        vec![
            ("ahora", nativa("tiempo.ahora", 0, 0, ahora)),
            ("millis", nativa("tiempo.millis", 0, 0, millis)),
            ("dormir", nativa("tiempo.dormir", 1, 1, dormir)),
            ("fecha", nativa("tiempo.fecha", 0, 1, fecha)),
            ("fecha_iso", nativa("tiempo.fecha_iso", 0, 1, fecha_iso)),
            ("iso", nativa("tiempo.iso", 0, 1, fecha_iso)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn ahora(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    let segundos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::ejecucion(format!("reloj del sistema inválido: {}", e)))?
        .as_secs();
    Ok(Value::Entero(segundos as i64))
}

fn millis(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    let milisegundos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::ejecucion(format!("reloj del sistema inválido: {}", e)))?
        .as_millis();
    Ok(Value::Entero(milisegundos as i64))
}

fn dormir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ms = args[0]
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("'dormir' espera milisegundos enteros"))?;
    if ms < 0 {
        return Err(Error::ejecucion(
            "'dormir' no admite una cantidad negativa de milisegundos",
        ));
    }
    thread::sleep(Duration::from_millis(ms as u64));
    Ok(Value::Nulo)
}

fn fecha(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let epoch = epoch_de_args(&args)?;
    Ok(Value::Texto(formatear_utc(epoch, false)))
}

fn fecha_iso(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let epoch = epoch_de_args(&args)?;
    Ok(Value::Texto(formatear_utc(epoch, true)))
}

fn epoch_de_args(args: &[Value]) -> Result<i64, Error> {
    match args.first() {
        Some(v) => v
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("la marca de tiempo debe ser un entero")),
        None => Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::ejecucion(format!("reloj inválido: {}", e)))?
            .as_secs() as i64),
    }
}

/// Formatea una marca Unix en UTC: "AAAA-MM-DD HH:MM:SS" o ISO 8601.
pub fn formatear_utc(epoch: i64, iso: bool) -> String {
    let dias = epoch.div_euclid(86_400);
    let segundos_dia = epoch.rem_euclid(86_400);
    let (anio, mes, dia) = civil_from_days(dias);
    let hora = segundos_dia / 3600;
    let minuto = (segundos_dia % 3600) / 60;
    let segundo = segundos_dia % 60;
    if iso {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            anio, mes, dia, hora, minuto, segundo
        )
    } else {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            anio, mes, dia, hora, minuto, segundo
        )
    }
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
