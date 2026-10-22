//! Módulo `tiempo`: reloj, fechas, partes y esperas.
//!
//! Todas las fechas se interpretan en UTC. Los epoch son segundos desde
//! 1970-01-01T00:00:00Z.

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
            ("fecha", nativa("tiempo.fecha", 0, 2, fecha)),
            ("fecha_iso", nativa("tiempo.fecha_iso", 0, 2, fecha_iso)),
            ("iso", nativa("tiempo.iso", 0, 2, fecha_iso)),
            ("formatear", nativa("tiempo.formatear", 1, 2, formatear)),
            ("partes", nativa("tiempo.partes", 0, 1, partes)),
            ("año", nativa("tiempo.año", 0, 1, anio)),
            ("mes", nativa("tiempo.mes", 0, 1, mes)),
            ("dia", nativa("tiempo.dia", 0, 1, dia)),
            ("hora", nativa("tiempo.hora", 0, 1, hora)),
            ("minuto", nativa("tiempo.minuto", 0, 1, minuto)),
            ("segundo", nativa("tiempo.segundo", 0, 1, segundo)),
            ("dia_semana", nativa("tiempo.dia_semana", 0, 1, dia_semana)),
            ("dia_del_año", nativa("tiempo.dia_del_año", 0, 1, dia_anio)),
            ("semana", nativa("tiempo.semana", 0, 1, semana)),
            (
                "minutos_totales",
                nativa("tiempo.minutos_totales", 0, 1, minutos_totales),
            ),
            (
                "horas_totales",
                nativa("tiempo.horas_totales", 0, 1, horas_totales),
            ),
            (
                "dias_totales",
                nativa("tiempo.dias_totales", 0, 1, dias_totales),
            ),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn ahora(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Entero(ahora_epoch()?))
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

fn ahora_epoch() -> Result<i64, Error> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::ejecucion(format!("reloj del sistema inválido: {}", e)))?
        .as_secs() as i64)
}

fn epoch_de_args(args: &[Value]) -> Result<i64, Error> {
    match args.first() {
        Some(valor) => epoch_de_valor(valor),
        None => ahora_epoch(),
    }
}

fn epoch_de_valor(valor: &Value) -> Result<i64, Error> {
    match valor {
        Value::Nulo => ahora_epoch(),
        otro => otro
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("la marca de tiempo debe ser un entero")),
    }
}

fn epoch_opcional(valor: Option<&Value>) -> Result<i64, Error> {
    match valor {
        Some(valor) => epoch_de_valor(valor),
        None => ahora_epoch(),
    }
}

fn fecha(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    if let Some(Value::Texto(formato)) = args.first() {
        let epoch = epoch_opcional(args.get(1))?;
        return Ok(Value::Texto(formatear_epoch(epoch, formato)));
    }
    Ok(Value::Texto(formatear_utc(epoch_de_args(&args)?, false)))
}

fn fecha_iso(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    if let Some(Value::Texto(formato)) = args.first() {
        let epoch = epoch_opcional(args.get(1))?;
        return Ok(Value::Texto(formatear_epoch(epoch, formato)));
    }
    Ok(Value::Texto(formatear_utc(epoch_de_args(&args)?, true)))
}

fn formatear(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let formato = args[0].a_texto();
    let epoch = epoch_opcional(args.get(1))?;
    Ok(Value::Texto(formatear_epoch(epoch, &formato)))
}

fn partes(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let epoch = epoch_de_args(&args)?;
    let p = partes_utc(epoch);
    Ok(Value::diccionario(vec![
        ("epoch".to_string(), Value::Entero(p.epoch)),
        ("año".to_string(), Value::Entero(p.anio)),
        ("mes".to_string(), Value::Entero(p.mes as i64)),
        ("dia".to_string(), Value::Entero(p.dia as i64)),
        ("hora".to_string(), Value::Entero(p.hora as i64)),
        ("minuto".to_string(), Value::Entero(p.minuto as i64)),
        ("segundo".to_string(), Value::Entero(p.segundo as i64)),
        ("dia_semana".to_string(), Value::Entero(p.dia_semana as i64)),
        ("dia_del_año".to_string(), Value::Entero(p.dia_anio as i64)),
        ("semana".to_string(), Value::Entero(p.semana as i64)),
        ("zona".to_string(), Value::Texto("UTC".to_string())),
    ]))
}

fn campo(args: &[Value], seleccion: fn(&Partes) -> i64) -> Result<Value, Error> {
    let epoch = epoch_de_args(args)?;
    Ok(Value::Entero(seleccion(&partes_utc(epoch))))
}

fn anio(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.anio)
}

fn mes(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.mes as i64)
}

fn dia(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.dia as i64)
}

fn hora(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.hora as i64)
}

fn minuto(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.minuto as i64)
}

fn segundo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.segundo as i64)
}

fn dia_semana(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.dia_semana as i64)
}

fn dia_anio(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.dia_anio as i64)
}

fn semana(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    campo(&args, |p| p.semana as i64)
}

fn minutos_totales(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let epoch = epoch_de_args(&args)?;
    Ok(Value::Entero(epoch.div_euclid(60)))
}

fn horas_totales(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let epoch = epoch_de_args(&args)?;
    Ok(Value::Entero(epoch.div_euclid(3600)))
}

fn dias_totales(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let epoch = epoch_de_args(&args)?;
    Ok(Value::Entero(epoch.div_euclid(86_400)))
}

#[derive(Clone, Copy)]
struct Partes {
    epoch: i64,
    anio: i64,
    mes: u32,
    dia: u32,
    hora: u32,
    minuto: u32,
    segundo: u32,
    /// 1 = lunes, 7 = domingo.
    dia_semana: u32,
    /// 1..=366.
    dia_anio: u32,
    /// Semana ISO 8601.
    semana: u32,
}

fn partes_utc(epoch: i64) -> Partes {
    let dias = epoch.div_euclid(86_400);
    let segundos_dia = epoch.rem_euclid(86_400);
    let (anio, mes, dia) = civil_from_days(dias);
    Partes {
        epoch,
        anio,
        mes,
        dia,
        hora: (segundos_dia / 3600) as u32,
        minuto: ((segundos_dia % 3600) / 60) as u32,
        segundo: (segundos_dia % 60) as u32,
        dia_semana: ((dias + 3).rem_euclid(7) + 1) as u32,
        dia_anio: dia_del_anio(anio, mes, dia),
        semana: semana_iso(dias),
    }
}

/// Formatea una marca Unix en UTC: "AAAA-MM-DD HH:MM:SS" o ISO 8601.
pub fn formatear_utc(epoch: i64, iso: bool) -> String {
    let p = partes_utc(epoch);
    if iso {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            p.anio, p.mes, p.dia, p.hora, p.minuto, p.segundo
        )
    } else {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            p.anio, p.mes, p.dia, p.hora, p.minuto, p.segundo
        )
    }
}

/// Formato con códigos: A/Y año, M mes, D día, H hora, m minuto, S segundo,
/// W día de la semana (1 = lunes), J día del año, E epoch y Z = UTC.
/// `\` escapa el siguiente carácter.
fn formatear_epoch(epoch: i64, formato: &str) -> String {
    let p = partes_utc(epoch);
    let mut salida = String::with_capacity(formato.len() + 8);
    let mut caracteres = formato.chars();

    while let Some(caracter) = caracteres.next() {
        match caracter {
            '\\' => {
                if let Some(escapado) = caracteres.next() {
                    salida.push(escapado);
                } else {
                    salida.push('\\');
                }
            }
            'A' | 'Y' => salida.push_str(&format!("{:04}", p.anio)),
            'M' => salida.push_str(&format!("{:02}", p.mes)),
            'D' => salida.push_str(&format!("{:02}", p.dia)),
            'H' => salida.push_str(&format!("{:02}", p.hora)),
            'm' => salida.push_str(&format!("{:02}", p.minuto)),
            'S' => salida.push_str(&format!("{:02}", p.segundo)),
            'W' => salida.push_str(&p.dia_semana.to_string()),
            'J' => salida.push_str(&format!("{:03}", p.dia_anio)),
            'E' => salida.push_str(&p.epoch.to_string()),
            'Z' => salida.push_str("UTC"),
            otro => salida.push(otro),
        }
    }

    salida
}

fn dia_del_anio(anio: i64, mes: u32, dia: u32) -> u32 {
    const DIAS_MES: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let bisiesto = es_bisiesto(anio);
    let mut total = dia;
    for m in 1..mes {
        total += DIAS_MES[(m - 1) as usize];
        if m == 2 && bisiesto {
            total += 1;
        }
    }
    total
}

fn es_bisiesto(anio: i64) -> bool {
    anio.rem_euclid(4) == 0 && (anio.rem_euclid(100) != 0 || anio.rem_euclid(400) == 0)
}

fn semana_iso(dias: i64) -> u32 {
    let dia_semana = (dias + 3).rem_euclid(7) + 1;
    let jueves = dias - (dia_semana - 4);
    let (anio_jueves, _, _) = civil_from_days(jueves);
    let enero4 = dias_desde_civil(anio_jueves, 1, 4);
    let dia_semana_enero4 = (enero4 + 3).rem_euclid(7) + 1;
    let jueves_semana1 = enero4 - (dia_semana_enero4 - 4);
    (((jueves - jueves_semana1).div_euclid(7)) + 1) as u32
}

fn dias_desde_civil(anio: i64, mes: u32, dia: u32) -> i64 {
    let y = if mes <= 2 { anio - 1 } else { anio };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m = mes as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + dia as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
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
