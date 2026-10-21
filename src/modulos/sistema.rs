//! Módulo `sistema`: entorno, argumentos, salida y archivos `.env`.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::Error;
use crate::interprete::Interprete;
use crate::modulos::crear_modulo;
use crate::value::{valor_verdadero, Value};

pub fn modulo() -> Value {
    crear_modulo(
        "sistema",
        vec![
            ("env", nativa("sistema.env", 1, 2, env)),
            ("cargar_env", nativa("sistema.cargar_env", 0, 2, cargar_env)),
            ("argumentos", nativa("sistema.argumentos", 0, 0, argumentos)),
            ("salir", nativa("sistema.salir", 0, 1, salir)),
            ("directorio", nativa("sistema.directorio", 0, 0, directorio)),
            ("plataforma", nativa("sistema.plataforma", 0, 0, plataforma)),
        ],
    )
}

fn nativa(nombre: &'static str, min: usize, max: usize, funcion: crate::value::NativaFn) -> Value {
    crate::modulos::nativa(nombre, min, max, funcion)
}

fn resolver(interprete: &Interprete, ruta: &str) -> PathBuf {
    let ruta = Path::new(ruta);
    if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        interprete.ruta_base.join(ruta)
    }
}

fn env(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let nombre = args[0].a_texto();
    match std::env::var(&nombre) {
        Ok(valor) => Ok(Value::Texto(valor)),
        Err(_) => Ok(args.get(1).cloned().unwrap_or(Value::Nulo)),
    }
}

/// Carga un archivo `.env` y devuelve un diccionario.
///
/// Firma: `sistema.cargar_env(ruta = ".env", obligatorio = falso)`.
/// No modifica las variables de entorno del proceso.
fn cargar_env(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let ruta_texto = args
        .first()
        .map(|valor| valor.a_texto())
        .unwrap_or_else(|| ".env".to_string());
    let obligatorio = args.get(1).map(valor_verdadero).unwrap_or(false);
    let ruta = resolver(interprete, &ruta_texto);

    let contenido = match fs::read_to_string(&ruta) {
        Ok(contenido) => contenido,
        Err(error) if error.kind() == ErrorKind::NotFound && !obligatorio => {
            return Ok(Value::diccionario(Vec::new()));
        }
        Err(error) => {
            return Err(Error::ejecucion(format!(
                "no se pudo leer '{}': {}",
                ruta.display(),
                error
            )));
        }
    };

    let pares = parsear_contenido(&contenido)
        .map_err(|mensaje| Error::ejecucion(format!("'{}': {}", ruta.display(), mensaje)))?;

    Ok(Value::diccionario(
        pares
            .into_iter()
            .map(|(clave, valor)| (clave, Value::Texto(valor)))
            .collect(),
    ))
}

/// Parsea el contenido de un `.env`.
///
/// Soporta comentarios `#` (de línea completa o al final), `export`, comillas
/// simples y dobles, escapes en comillas dobles y valores que contienen `=`.
/// En valores sin comillas, `#` solo inicia un comentario si va precedido de
/// un espacio; `abc#def` conserva el `#`.
fn parsear_contenido(contenido: &str) -> Result<Vec<(String, String)>, String> {
    let contenido = contenido.strip_prefix('\u{feff}').unwrap_or(contenido);
    let mut pares = Vec::new();

    for (indice, linea_cruda) in contenido.lines().enumerate() {
        let numero = indice + 1;
        let mut linea = linea_cruda.trim();

        if linea.is_empty() || linea.starts_with('#') || linea == "export" {
            continue;
        }

        if let Some(resto) = linea.strip_prefix("export") {
            if resto.chars().next().is_some_and(char::is_whitespace) {
                linea = resto.trim_start();
                if linea.is_empty() || linea.starts_with('#') {
                    continue;
                }
            }
        }

        let Some(posicion) = linea.find('=') else {
            return Err(format!("línea {}: falta '='", numero));
        };

        let clave = linea[..posicion].trim();
        if clave.is_empty() {
            return Err(format!("línea {}: la clave está vacía", numero));
        }
        if clave.chars().any(char::is_whitespace) {
            return Err(format!(
                "línea {}: la clave '{}' contiene espacios",
                numero, clave
            ));
        }

        let valor = parsear_valor(&linea[posicion + 1..], numero)?;
        pares.push((clave.to_string(), valor));
    }

    Ok(pares)
}

fn parsear_valor(resto: &str, numero: usize) -> Result<String, String> {
    let resto = resto.trim_start();

    if let Some(resto) = resto.strip_prefix('"') {
        let mut valor = String::new();
        let mut escape = false;
        let mut cerrado = false;
        let mut fin = 0;

        for (indice, caracter) in resto.char_indices() {
            if escape {
                match caracter {
                    'n' => valor.push('\n'),
                    't' => valor.push('\t'),
                    'r' => valor.push('\r'),
                    '\\' => valor.push('\\'),
                    '"' => valor.push('"'),
                    otro => {
                        valor.push('\\');
                        valor.push(otro);
                    }
                }
                escape = false;
            } else if caracter == '\\' {
                escape = true;
            } else if caracter == '"' {
                cerrado = true;
                fin = indice + 1;
                break;
            } else {
                valor.push(caracter);
            }
        }

        if !cerrado {
            return Err(format!(
                "línea {}: falta la comilla doble de cierre",
                numero
            ));
        }

        validar_resto(&resto[fin..], numero)?;
        return Ok(valor);
    }

    if let Some(resto) = resto.strip_prefix('\'') {
        let Some(fin) = resto.find('\'') else {
            return Err(format!(
                "línea {}: falta la comilla simple de cierre",
                numero
            ));
        };

        let valor = resto[..fin].to_string();
        validar_resto(&resto[fin + 1..], numero)?;
        return Ok(valor);
    }

    let mut anterior_espacio = true;
    let mut corte = resto.len();

    for (indice, caracter) in resto.char_indices() {
        if caracter == '#' && anterior_espacio {
            corte = indice;
            break;
        }
        anterior_espacio = caracter.is_whitespace();
    }

    Ok(resto[..corte].trim().to_string())
}

fn validar_resto(resto: &str, numero: usize) -> Result<(), String> {
    let resto = resto.trim();
    if resto.is_empty() || resto.starts_with('#') {
        Ok(())
    } else {
        Err(format!(
            "línea {}: texto inesperado después del valor entre comillas",
            numero
        ))
    }
}

fn argumentos(interprete: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::lista(
        interprete
            .argumentos
            .iter()
            .map(|a| Value::Texto(a.clone()))
            .collect(),
    ))
}

fn salir(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let codigo = match args.first() {
        Some(v) => v
            .a_entero_exacto()
            .ok_or_else(|| Error::ejecucion("'salir' espera un código entero"))?,
        None => 0,
    };
    Err(Error::Salir(codigo as i32))
}

fn directorio(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    let ruta = std::env::current_dir()
        .map_err(|e| Error::ejecucion(format!("no se pudo obtener el directorio: {}", e)))?;
    Ok(Value::Texto(ruta.to_string_lossy().to_string()))
}

fn plataforma(_: &mut Interprete, _: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(std::env::consts::OS.to_string()))
}

#[cfg(test)]
mod tests {
    use super::parsear_contenido;
    use std::collections::HashMap;

    fn mapa(texto: &str) -> HashMap<String, String> {
        parsear_contenido(texto)
            .expect("el .env debería ser válido")
            .into_iter()
            .collect()
    }

    #[test]
    fn parsea_valores_comentarios_y_comillas() {
        let valores = mapa(
            r#"
            # Configuración
            PUERTO=8080
            NOMBRE = "Vaca servidor" # comentario final
            export DEBUG=true
            URL=http://localhost:8080/api?x=1
            CLAVE='valor # literal'
            MENSAJE="línea\nnueva"
            IGUAL=a=b=c
            ALMOHADILLA=abc#def
            VACIO=
            "#,
        );

        assert_eq!(valores.get("PUERTO").map(String::as_str), Some("8080"));
        assert_eq!(
            valores.get("NOMBRE").map(String::as_str),
            Some("Vaca servidor")
        );
        assert_eq!(valores.get("DEBUG").map(String::as_str), Some("true"));
        assert_eq!(
            valores.get("URL").map(String::as_str),
            Some("http://localhost:8080/api?x=1")
        );
        assert_eq!(
            valores.get("CLAVE").map(String::as_str),
            Some("valor # literal")
        );
        assert_eq!(
            valores.get("MENSAJE").map(String::as_str),
            Some("línea\nnueva")
        );
        assert_eq!(valores.get("IGUAL").map(String::as_str), Some("a=b=c"));
        assert_eq!(
            valores.get("ALMOHADILLA").map(String::as_str),
            Some("abc#def")
        );
        assert_eq!(valores.get("VACIO").map(String::as_str), Some(""));
    }

    #[test]
    fn rechaza_lineas_invalidas() {
        assert!(parsear_contenido("SIN_IGUAL").is_err());
        assert!(parsear_contenido("=valor").is_err());
        assert!(parsear_contenido("CLAVE=\"sin cerrar").is_err());
        assert!(parsear_contenido("CLAVE=\"ok\" sobra").is_err());
        assert!(parsear_contenido("CLAVE='sin cerrar").is_err());
    }
}
