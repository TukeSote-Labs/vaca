//! Módulo `datos`: DataFrames simples sobre listas de diccionarios/columnas.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::{clave_texto, comparar_orden, valor_verdadero, valores_iguales, Value};

pub fn modulo() -> Value {
    crear_modulo(
        "datos",
        vec![
            ("leer_csv", nativa("datos.leer_csv", 1, 1, leer_csv)),
            (
                "guardar_csv",
                nativa("datos.guardar_csv", 2, 2, guardar_csv),
            ),
            (
                "dimensiones",
                nativa("datos.dimensiones", 1, 1, dimensiones),
            ),
            ("nombres", nativa("datos.nombres", 1, 1, nombres)),
            ("columna", nativa("datos.columna", 2, 2, columna)),
            ("filtrar", nativa("datos.filtrar", 4, 4, filtrar)),
            ("ordenar", nativa("datos.ordenar", 2, 3, ordenar)),
            ("mutar", nativa("datos.mutar", 3, 3, mutar)),
            (
                "agrupar_resumir",
                nativa("datos.agrupar_resumir", 4, 4, agrupar_resumir),
            ),
            ("ver", nativa("datos.ver", 1, 1, ver)),
            ("a_json", nativa("datos.a_json", 1, 1, a_json)),
            ("desde_json", nativa("datos.desde_json", 1, 1, desde_json)),
            ("unir", nativa("datos.unir", 2, 2, unir)),
            (
                "seleccionar",
                nativa("datos.seleccionar", 2, 2, seleccionar),
            ),
            (
                "eliminar_columna",
                nativa("datos.eliminar_columna", 2, 2, eliminar_columna),
            ),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

// ----------------------------------------------------------------------
// Estructura interna
// ----------------------------------------------------------------------

struct Marco {
    columnas: Vec<String>,
    filas: Vec<Vec<Value>>,
}

fn leer_marco(valor: &Value, nombre: &str) -> Result<Marco, Error> {
    let dict = match valor {
        Value::Diccionario(d) => d,
        otro => {
            return Err(Error::ejecucion(format!(
                "'{}' espera un DataFrame y recibió {}",
                nombre,
                otro.tipo_nombre()
            )))
        }
    };
    let dict = dict.lock().unwrap();
    let columnas_valor = dict.get("columnas").ok_or_else(|| {
        Error::ejecucion(format!(
            "'{}' recibió un diccionario sin 'columnas'",
            nombre
        ))
    })?;
    let filas_valor = dict.get("filas").ok_or_else(|| {
        Error::ejecucion(format!("'{}' recibió un diccionario sin 'filas'", nombre))
    })?;
    let columnas = match columnas_valor {
        Value::Lista(l) => l
            .lock()
            .unwrap()
            .iter()
            .map(|v| v.a_texto())
            .collect::<Vec<_>>(),
        _ => return Err(Error::ejecucion("'columnas' debe ser una lista de textos")),
    };
    let filas = match filas_valor {
        Value::Lista(l) => l
            .lock()
            .unwrap()
            .iter()
            .map(|fila| match fila {
                Value::Lista(celdas) => Ok(celdas.lock().unwrap().clone()),
                _ => Err(Error::ejecucion("cada fila debe ser una lista")),
            })
            .collect::<Result<Vec<_>, Error>>()?,
        _ => return Err(Error::ejecucion("'filas' debe ser una lista")),
    };
    Ok(Marco { columnas, filas })
}

fn crear_marco(marco: Marco) -> Value {
    Value::diccionario(vec![
        (
            "columnas".to_string(),
            Value::lista(
                marco
                    .columnas
                    .iter()
                    .map(|c| Value::Texto(c.clone()))
                    .collect(),
            ),
        ),
        (
            "filas".to_string(),
            Value::lista(marco.filas.into_iter().map(Value::lista).collect()),
        ),
    ])
}

fn indice_columna(marco: &Marco, nombre: &str) -> Result<usize, Error> {
    marco
        .columnas
        .iter()
        .position(|c| c == nombre)
        .ok_or_else(|| {
            Error::ejecucion(format!(
                "la columna '{}' no existe (columnas: {})",
                nombre,
                marco.columnas.join(", ")
            ))
        })
}

fn celda(fila: &[Value], indice: usize) -> Value {
    fila.get(indice).cloned().unwrap_or(Value::Nulo)
}

fn fila_como_diccionario(marco: &Marco, fila: &[Value]) -> Value {
    let mut pares = Vec::with_capacity(marco.columnas.len());
    for (i, columna) in marco.columnas.iter().enumerate() {
        pares.push((columna.clone(), celda(fila, i)));
    }
    Value::diccionario(pares)
}

// ----------------------------------------------------------------------
// CSV
// ----------------------------------------------------------------------

fn leer_csv(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta = resolver_ruta(interprete, &args[0].a_texto());
    let contenido = fs::read_to_string(&ruta)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e)))?;
    let filas_crudas = parsear_csv(&contenido);
    if filas_crudas.is_empty() {
        return Ok(crear_marco(Marco {
            columnas: Vec::new(),
            filas: Vec::new(),
        }));
    }
    let columnas = filas_crudas[0].clone();
    let filas = filas_crudas[1..]
        .iter()
        .filter(|fila| fila.iter().any(|c| !c.trim().is_empty()))
        .map(|fila| {
            (0..columnas.len())
                .map(|i| valor_desde_csv(fila.get(i).map(|s| s.as_str()).unwrap_or("")))
                .collect::<Vec<_>>()
        })
        .collect();
    Ok(crear_marco(Marco { columnas, filas }))
}

fn resolver_ruta(interprete: &Interprete, ruta: &str) -> std::path::PathBuf {
    let ruta = Path::new(ruta);
    if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        interprete.ruta_base.join(ruta)
    }
}

fn parsear_csv(contenido: &str) -> Vec<Vec<String>> {
    let mut filas = Vec::new();
    let mut fila = Vec::new();
    let mut campo = String::new();
    let mut en_comillas = false;
    let mut chars = contenido.chars().peekable();

    while let Some(c) = chars.next() {
        if en_comillas {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    campo.push('"');
                    chars.next();
                } else {
                    en_comillas = false;
                }
            } else {
                campo.push(c);
            }
        } else {
            match c {
                '"' => en_comillas = true,
                ',' => fila.push(std::mem::take(&mut campo)),
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    fila.push(std::mem::take(&mut campo));
                    filas.push(std::mem::take(&mut fila));
                }
                '\n' => {
                    fila.push(std::mem::take(&mut campo));
                    filas.push(std::mem::take(&mut fila));
                }
                _ => campo.push(c),
            }
        }
    }
    if !campo.is_empty() || !fila.is_empty() {
        fila.push(campo);
        filas.push(fila);
    }
    filas
}

fn valor_desde_csv(texto: &str) -> Value {
    let t = texto.trim();
    if t.is_empty() {
        return Value::Nulo;
    }
    let minusculas = t.to_lowercase();
    if matches!(minusculas.as_str(), "verdadero" | "true") {
        return Value::Logico(true);
    }
    if matches!(minusculas.as_str(), "falso" | "false") {
        return Value::Logico(false);
    }
    if let Ok(n) = t.parse::<i64>() {
        return Value::Entero(n);
    }
    if let Ok(n) = t.parse::<f64>() {
        return Value::Decimal(n);
    }
    Value::Texto(t.to_string())
}

fn guardar_csv(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "guardar_csv")?;
    let ruta = resolver_ruta(interprete, &args[1].a_texto());
    let mut salida = String::new();
    salida.push_str(
        &marco
            .columnas
            .iter()
            .map(|c| escapar_csv(c))
            .collect::<Vec<_>>()
            .join(","),
    );
    salida.push('\n');
    for fila in &marco.filas {
        let linea = (0..marco.columnas.len())
            .map(|i| escapar_csv(&celda(fila, i).a_texto()))
            .collect::<Vec<_>>()
            .join(",");
        salida.push_str(&linea);
        salida.push('\n');
    }
    fs::write(&ruta, salida)
        .map_err(|e| Error::ejecucion(format!("no se pudo guardar '{}': {}", ruta.display(), e)))?;
    Ok(Value::Nulo)
}

fn escapar_csv(texto: &str) -> String {
    if texto.contains(',') || texto.contains('"') || texto.contains('\n') || texto.contains('\r') {
        format!("\"{}\"", texto.replace('"', "\"\""))
    } else {
        texto.to_string()
    }
}

// ----------------------------------------------------------------------
// Operaciones
// ----------------------------------------------------------------------

fn dimensiones(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "dimensiones")?;
    Ok(Value::lista(vec![
        Value::Entero(marco.filas.len() as i64),
        Value::Entero(marco.columnas.len() as i64),
    ]))
}

fn nombres(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "nombres")?;
    Ok(Value::lista(
        marco
            .columnas
            .iter()
            .map(|c| Value::Texto(c.clone()))
            .collect(),
    ))
}

fn columna(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "columna")?;
    let indice = indice_columna(&marco, &args[1].a_texto())?;
    Ok(Value::lista(
        marco.filas.iter().map(|fila| celda(fila, indice)).collect(),
    ))
}

fn filtrar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "filtrar")?;
    let indice = indice_columna(&marco, &args[1].a_texto())?;
    let operador = args[2].a_texto();
    let referencia = &args[3];
    let filas = marco
        .filas
        .iter()
        .filter(|fila| {
            let valor = celda(fila, indice);
            match operador.as_str() {
                "==" | "=" => valores_iguales(&valor, referencia),
                "!=" | "<>" => !valores_iguales(&valor, referencia),
                ">" => comparar_orden(&valor, referencia)
                    .map(|o| o.is_gt())
                    .unwrap_or(false),
                "<" => comparar_orden(&valor, referencia)
                    .map(|o| o.is_lt())
                    .unwrap_or(false),
                ">=" => comparar_orden(&valor, referencia)
                    .map(|o| o.is_ge())
                    .unwrap_or(false),
                "<=" => comparar_orden(&valor, referencia)
                    .map(|o| o.is_le())
                    .unwrap_or(false),
                "contiene" => valor.a_texto().contains(&referencia.a_texto()),
                _ => false,
            }
        })
        .cloned()
        .collect();
    Ok(crear_marco(Marco {
        columnas: marco.columnas,
        filas,
    }))
}

fn ordenar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let mut marco = leer_marco(&args[0], "ordenar")?;
    let indice = indice_columna(&marco, &args[1].a_texto())?;
    let ascendente = args.get(2).map(valor_verdadero).unwrap_or(true);
    marco.filas.sort_by(|a, b| {
        let orden = comparar_orden(&celda(a, indice), &celda(b, indice))
            .unwrap_or(std::cmp::Ordering::Equal);
        if ascendente {
            orden
        } else {
            orden.reverse()
        }
    });
    Ok(crear_marco(marco))
}

fn mutar(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let mut marco = leer_marco(&args[0], "mutar")?;
    let nombre = args[1].a_texto();
    if marco.columnas.iter().any(|c| c == &nombre) {
        return Err(Error::ejecucion(format!(
            "la columna '{}' ya existe en el DataFrame",
            nombre
        )));
    }
    let generador = args[2].clone();
    let mut nuevas_filas = Vec::with_capacity(marco.filas.len());
    for fila in &marco.filas {
        let valor = if generador.es_llamable() {
            let diccionario = fila_como_diccionario(&marco, fila);
            interprete.llamar(generador.clone(), vec![diccionario])?
        } else {
            generador.clone()
        };
        let mut nueva = fila.clone();
        nueva.push(valor);
        nuevas_filas.push(nueva);
    }
    marco.columnas.push(nombre);
    marco.filas = nuevas_filas;
    Ok(crear_marco(marco))
}

fn agrupar_resumir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "agrupar_resumir")?;
    let indice_grupo = indice_columna(&marco, &args[1].a_texto())?;
    let indice_resumen = indice_columna(&marco, &args[2].a_texto())?;
    let operacion = args[3].a_texto().to_lowercase();

    let mut grupos: HashMap<String, Vec<Value>> = HashMap::new();
    let mut orden: Vec<String> = Vec::new();
    for fila in &marco.filas {
        let clave = clave_texto(&celda(fila, indice_grupo));
        if !grupos.contains_key(&clave) {
            orden.push(clave.clone());
            grupos.insert(clave.clone(), Vec::new());
        }
        grupos
            .get_mut(&clave)
            .unwrap()
            .push(celda(fila, indice_resumen));
    }
    orden.sort();

    let mut filas = Vec::with_capacity(orden.len());
    for clave in orden {
        let valores = grupos.get(&clave).cloned().unwrap_or_default();
        let resumen = resumir(&valores, &operacion)?;
        filas.push(vec![Value::Texto(clave), resumen]);
    }

    Ok(crear_marco(Marco {
        columnas: vec![args[1].a_texto(), args[2].a_texto()],
        filas,
    }))
}

fn resumir(valores: &[Value], operacion: &str) -> Result<Value, Error> {
    match operacion {
        "conteo" | "contar" | "count" => Ok(Value::Entero(valores.len() as i64)),
        "suma" | "sumar" | "sum" => {
            let mut total = 0.0;
            for valor in valores {
                let n = valor.a_decimal().ok_or_else(|| {
                    Error::ejecucion("'suma' solo admite columnas numéricas")
                })?;
                total += n;
            }
            Ok(Value::Decimal(total))
        }
        "promedio" | "media" | "mean" => {
            if valores.is_empty() {
                return Ok(Value::Decimal(0.0));
            }
            let mut total = 0.0;
            for valor in valores {
                let n = valor.a_decimal().ok_or_else(|| {
                    Error::ejecucion("'promedio' solo admite columnas numéricas")
                })?;
                total += n;
            }
            Ok(Value::Decimal(total / valores.len() as f64))
        }
        "min" | "minimo" | "mínimo" => Ok(valores
            .iter()
            .min_by(|a, b| comparar_orden(a, b).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
            .unwrap_or(Value::Nulo)),
        "max" | "maximo" | "máximo" => Ok(valores
            .iter()
            .max_by(|a, b| comparar_orden(a, b).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
            .unwrap_or(Value::Nulo)),
        "primero" => Ok(valores.first().cloned().unwrap_or(Value::Nulo)),
        "ultimo" | "último" => Ok(valores.last().cloned().unwrap_or(Value::Nulo)),
        "unir" => Ok(Value::Texto(
            valores
                .iter()
                .map(|v| v.a_texto())
                .collect::<Vec<_>>()
                .join(","),
        )),
        otra => Err(Error::ejecucion(format!(
            "operación de resumen desconocida '{}' (usa suma, promedio, conteo, min, max, primero, ultimo o unir)",
            otra
        ))),
    }
}

fn ver(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "ver")?;
    let mut anchos: Vec<usize> = marco
        .columnas
        .iter()
        .map(|c| c.chars().count().max(3))
        .collect();
    for fila in &marco.filas {
        for (i, ancho) in anchos.iter_mut().enumerate() {
            let texto = celda(fila, i).a_texto();
            *ancho = (*ancho).max(texto.chars().count());
        }
    }

    let cabecera = marco
        .columnas
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:<width$}", c, width = anchos[i]))
        .collect::<Vec<_>>()
        .join("  ");
    interprete.escribir_linea(&cabecera);
    let separador = anchos
        .iter()
        .map(|w| "-".repeat(*w))
        .collect::<Vec<_>>()
        .join("  ");
    interprete.escribir_linea(&separador);
    for fila in &marco.filas {
        let linea = (0..marco.columnas.len())
            .map(|i| format!("{:<width$}", celda(fila, i).a_texto(), width = anchos[i]))
            .collect::<Vec<_>>()
            .join("  ");
        interprete.escribir_linea(&linea);
    }
    interprete.escribir_linea(&format!(
        "{} fila(s), {} columna(s)",
        marco.filas.len(),
        marco.columnas.len()
    ));
    Ok(Value::Nulo)
}

fn a_json(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "a_json")?;
    let filas = marco
        .filas
        .iter()
        .map(|fila| fila_como_diccionario(&marco, fila))
        .collect();
    Ok(Value::lista(filas))
}

fn desde_json(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let lista = match &args[0] {
        Value::Lista(l) => l.lock().unwrap().clone(),
        otro => {
            return Err(Error::ejecucion(format!(
                "'desde_json' espera una lista de diccionarios y recibió {}",
                otro.tipo_nombre()
            )))
        }
    };

    let mut columnas: Vec<String> = Vec::new();
    let mut vistos: HashMap<String, ()> = HashMap::new();
    let mut filas_crudas: Vec<HashMap<String, Value>> = Vec::new();
    for elemento in &lista {
        let dict = match elemento {
            Value::Diccionario(d) => d.lock().unwrap().clone(),
            otro => {
                return Err(Error::ejecucion(format!(
                    "cada elemento debe ser un diccionario, se encontró {}",
                    otro.tipo_nombre()
                )))
            }
        };
        let mut claves: Vec<String> = dict.keys().cloned().collect();
        claves.sort();
        for clave in claves {
            if !vistos.contains_key(&clave) {
                vistos.insert(clave.clone(), ());
                columnas.push(clave);
            }
        }
        filas_crudas.push(dict);
    }

    let filas = filas_crudas
        .iter()
        .map(|dict| {
            columnas
                .iter()
                .map(|c| dict.get(c).cloned().unwrap_or(Value::Nulo))
                .collect::<Vec<_>>()
        })
        .collect();

    Ok(crear_marco(Marco { columnas, filas }))
}

fn unir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let a = leer_marco(&args[0], "unir")?;
    let b = leer_marco(&args[1], "unir")?;
    let mut columnas = a.columnas.clone();
    for columna in &b.columnas {
        if !columnas.contains(columna) {
            columnas.push(columna.clone());
        }
    }
    let reordenar = |marco: &Marco| -> Vec<Vec<Value>> {
        marco
            .filas
            .iter()
            .map(|fila| {
                columnas
                    .iter()
                    .map(|columna| {
                        marco
                            .columnas
                            .iter()
                            .position(|c| c == columna)
                            .map(|i| celda(fila, i))
                            .unwrap_or(Value::Nulo)
                    })
                    .collect()
            })
            .collect()
    };
    let mut filas = reordenar(&a);
    filas.extend(reordenar(&b));
    Ok(crear_marco(Marco { columnas, filas }))
}

fn seleccionar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "seleccionar")?;
    let nombres = match &args[1] {
        Value::Lista(l) => l
            .lock()
            .unwrap()
            .iter()
            .map(|v| v.a_texto())
            .collect::<Vec<_>>(),
        otro => {
            return Err(Error::ejecucion(format!(
                "'seleccionar' espera una lista de nombres y recibió {}",
                otro.tipo_nombre()
            )))
        }
    };
    let indices: Vec<usize> = nombres
        .iter()
        .map(|n| indice_columna(&marco, n))
        .collect::<Result<Vec<_>, Error>>()?;
    let filas = marco
        .filas
        .iter()
        .map(|fila| indices.iter().map(|&i| celda(fila, i)).collect())
        .collect();
    Ok(crear_marco(Marco {
        columnas: nombres,
        filas,
    }))
}

fn eliminar_columna(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let marco = leer_marco(&args[0], "eliminar_columna")?;
    let indice = indice_columna(&marco, &args[1].a_texto())?;
    let mut columnas = marco.columnas.clone();
    columnas.remove(indice);
    let filas = marco
        .filas
        .iter()
        .map(|fila| {
            fila.iter()
                .enumerate()
                .filter(|(i, _)| *i != indice)
                .map(|(_, v)| v.clone())
                .collect()
        })
        .collect();
    Ok(crear_marco(Marco { columnas, filas }))
}
