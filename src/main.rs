//! CLI del lenguaje Vaca.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use vaca::lexer::Lexer;
use vaca::paquetes::Manifiesto;
use vaca::parser::Parser;
use vaca::value::Value;
use vaca::{Error, Interprete};

/// Versión base de Cargo más los metadatos del último commit.
const VERSION: &str = env!("VACA_VERSION");
const TAGLINE: &str = "Vaca: Lenguaje en español para servidores";

fn main() -> ExitCode {
    let argumentos: Vec<String> = env::args().skip(1).collect();
    let codigo = match argumentos.first().map(|s| s.as_str()) {
        None => comando_modulo(None, &[]),
        Some("-h") | Some("--ayuda") | Some("ayuda") => {
            ayuda();
            0
        }
        Some("-v") | Some("--version") | Some("version") => {
            println!("Vaca {}", VERSION);
            0
        }
        Some("repl") => match repl() {
            Ok(()) => 0,
            Err(codigo) => codigo,
        },
        Some("tokens") => match argumentos.get(1) {
            Some(ruta) => mostrar_tokens(ruta),
            None => {
                eprintln!("uso: vaca tokens <archivo.vaca>");
                1
            }
        },
        Some("ast") => match argumentos.get(1) {
            Some(ruta) => mostrar_ast(ruta),
            None => {
                eprintln!("uso: vaca ast <archivo.vaca>");
                1
            }
        },
        Some("run") | Some("ejecutar") => match argumentos.get(1) {
            Some(ruta) if Path::new(ruta).is_dir() => comando_modulo(Some(ruta), &argumentos[2..]),
            Some(ruta) => ejecutar(ruta, &argumentos[2..]),
            None => comando_modulo(None, &[]),
        },
        Some("construir") | Some("build") => comando_construir(),
        Some("init") => comando_init(argumentos.get(1).map(|s| s.as_str())),
        Some("instalar") | Some("install") => comando_instalar(),
        Some("agregar") | Some("get") => match argumentos.get(1) {
            Some(argumento) => comando_agregar(argumento),
            None => {
                eprintln!("uso: vaca agregar <dominio/usuario/repo>[@version]");
                1
            }
        },
        Some("actualizar") | Some("update") => comando_actualizar(),
        Some("listar") | Some("list") => comando_listar(),
        Some("limpiar") | Some("clean") => comando_limpiar(),
        Some(ruta) if Path::new(ruta).is_dir() => comando_modulo(Some(ruta), &argumentos[1..]),
        Some(ruta) => ejecutar(ruta, &argumentos[1..]),
    };
    ExitCode::from(codigo as u8)
}

fn ayuda() {
    print!("{}", banner());
    println!();
    println!(
        "USO:
    vaca                       Ejecuta el módulo del directorio actual
    vaca <archivo.vaca>        Ejecuta un programa o un bundle
    vaca <directorio>          Ejecuta el módulo de ese directorio
    vaca run [archivo]         Igual que arriba (sin archivo: el módulo)
    vaca repl                  Abre la consola interactiva
    vaca tokens <archivo>      Muestra los tokens del archivo
    vaca ast <archivo>         Muestra el árbol de sintaxis
    vaca --version             Muestra la versión
    vaca --ayuda               Muestra esta ayuda

MÓDULOS Y PAQUETES:
    vaca init <modulo>         Crea vaca.mod e iniciar.vaca
    vaca construir             Empaqueta el módulo en .vaca/dist/*.vaca
    vaca agregar <ruta>[@v]    Añade una dependencia y la instala
    vaca instalar              Instala las dependencias del manifiesto
    vaca actualizar            Re-resuelve versiones ignorando el lock
    vaca listar                Muestra módulo, entrada y dependencias
    vaca limpiar               Borra .vaca/paquetes y vaca.lock

EJEMPLO:
    vaca init ejemplo.com/mi-api
    vaca"
    );
}

/// Banner estilo cowsay con la versión del build.
fn banner() -> String {
    let version = format!("VERSION = {}", VERSION);
    let ancho = TAGLINE.chars().count().max(version.chars().count());
    let mut texto = String::new();

    texto.push_str(&format!(" {}\n", "_".repeat(ancho + 2)));
    texto.push_str(&format!("/ {:<ancho$} \\\n", TAGLINE, ancho = ancho));
    texto.push_str(&format!("\\ {:<ancho$} /\n", version, ancho = ancho));
    texto.push_str(&format!(" {}\n", "-".repeat(ancho + 2)));
    texto.push_str(
        r"        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
",
    );

    texto
}

fn ejecutar(ruta: &str, extra: &[String]) -> u8 {
    let ruta_path = PathBuf::from(ruta);
    if vaca::bundle::es_bundle(&ruta_path) {
        return ejecutar_bundle(&ruta_path, extra);
    }
    ejecutar_ruta(ruta, extra, true)
}

/// Ejecuta el módulo del directorio actual (o del directorio indicado).
fn comando_modulo(ruta: Option<&str>, extra: &[String]) -> u8 {
    let directorio = match ruta {
        Some(ruta) => match fs::canonicalize(ruta) {
            Ok(directorio) => directorio,
            Err(error) => {
                eprintln!("error: no se pudo abrir '{}': {}", ruta, error);
                return 1;
            }
        },
        None => match directorio_actual() {
            Ok(directorio) => directorio,
            Err(codigo) => return codigo,
        },
    };

    let Some(raiz) = vaca::paquetes::raiz_proyecto(&directorio) else {
        if ruta.is_none() {
            ayuda();
            eprintln!(
                "\nerror: este directorio no es un módulo Vaca; ejecuta `vaca init <modulo>`"
            );
        } else {
            eprintln!(
                "error: '{}' no pertenece a un módulo Vaca (falta vaca.mod)",
                directorio.display()
            );
        }
        return 1;
    };
    let Some(ruta_manifiesto) = Manifiesto::ruta_en(&raiz) else {
        eprintln!("error: no se encontró vaca.mod en '{}'", raiz.display());
        return 1;
    };
    let manifiesto = match Manifiesto::leer(&ruta_manifiesto) {
        Ok(manifiesto) => manifiesto,
        Err(error) => {
            eprintln!("{}", error);
            return 1;
        }
    };
    let entrada = raiz.join(manifiesto.entrada());
    if !entrada.is_file() {
        eprintln!(
            "error: no existe la entrada '{}' declarada en '{}'\n       crea el archivo o cambia la línea 'inicio' del módulo",
            entrada.display(),
            ruta_manifiesto.display()
        );
        return 1;
    }
    ejecutar_ruta(&entrada.to_string_lossy(), extra, true)
}

fn ejecutar_ruta(ruta: &str, extra: &[String], llamar_principal: bool) -> u8 {
    let fuente = match fs::read_to_string(ruta) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {}", ruta, e);
            return 1;
        }
    };
    let mut interprete = Interprete::nuevo();
    // La pila del hilo principal suele ser de 8 MiB.
    interprete.max_profundidad = 256;
    interprete.argumentos = extra.to_vec();
    let ruta_path = PathBuf::from(ruta);
    if let Some(padre) = ruta_path.parent() {
        if !padre.as_os_str().is_empty() {
            interprete.ruta_base = fs::canonicalize(padre).unwrap_or_else(|_| padre.to_path_buf());
        }
    }
    match interprete.ejecutar_fuente(&fuente) {
        Ok(()) => terminar(&mut interprete, llamar_principal),
        Err(Error::Salir(codigo)) => codigo as u8,
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn ejecutar_bundle(ruta: &Path, extra: &[String]) -> u8 {
    let bundle = match vaca::bundle::leer(ruta) {
        Ok(bundle) => Arc::new(bundle),
        Err(error) => {
            eprintln!("{}", error);
            return 1;
        }
    };
    let inicio = bundle.inicio.clone();
    let fuente = match bundle.texto(&inicio) {
        Some(fuente) => fuente.to_string(),
        None => {
            eprintln!("error: el bundle no contiene '{}'", inicio);
            return 1;
        }
    };

    let mut interprete = Interprete::nuevo();
    interprete.max_profundidad = 256;
    interprete.argumentos = extra.to_vec();
    interprete.ruta_base = Path::new(&inicio)
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default();
    interprete.bundle = Some(Arc::clone(&bundle));

    match interprete.ejecutar_fuente(&fuente) {
        Ok(()) => terminar(&mut interprete, true),
        Err(Error::Salir(codigo)) => codigo as u8,
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

/// Tras ejecutar la entrada del módulo, llama a `principal()` si existe.
fn terminar(interprete: &mut Interprete, llamar_principal: bool) -> u8 {
    if llamar_principal {
        let valor = vaca::interprete::Entorno::obtener(&interprete.global, "principal");
        if let Some(valor @ (Value::Funcion(_) | Value::Nativa(_))) = valor {
            if let Err(error) = interprete.llamar(valor, Vec::new()) {
                return match error {
                    Error::Salir(codigo) => codigo as u8,
                    error => {
                        eprintln!("{}", error);
                        1
                    }
                };
            }
        }
    }
    interprete.codigo_salida.unwrap_or(0) as u8
}

fn mostrar_tokens(ruta: &str) -> u8 {
    let fuente = match fs::read_to_string(ruta) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {}", ruta, e);
            return 1;
        }
    };
    match Lexer::nuevo(&fuente).tokenizar() {
        Ok(tokens) => {
            for token in tokens {
                println!("{:>3}:{:<3} {:?}", token.linea, token.columna, token.tipo);
            }
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn mostrar_ast(ruta: &str) -> u8 {
    let fuente = match fs::read_to_string(ruta) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: no se pudo leer '{}': {}", ruta, e);
            return 1;
        }
    };
    match Parser::desde_codigo(&fuente).and_then(|mut p| p.parsear_programa()) {
        Ok(programa) => {
            println!("{:#?}", programa);
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn repl() -> Result<(), u8> {
    println!("Vaca {} — consola interactiva", VERSION);
    println!("Escribe 'salir' para terminar.\n");
    let mut interprete = Interprete::nuevo();
    interprete.max_profundidad = 256;
    let mut buffer = String::new();

    loop {
        if buffer.is_empty() {
            print!("vaca> ");
        } else {
            print!("...   ");
        }
        let _ = io::stdout().flush();

        let mut linea = String::new();
        let leidos = io::stdin().read_line(&mut linea).map_err(|_| 1u8)?;
        if leidos == 0 {
            break;
        }
        let recortada = linea.trim();
        if buffer.is_empty() && (recortada == "salir" || recortada == "exit" || recortada == "quit")
        {
            break;
        }
        buffer.push_str(&linea);

        if !balanceado(&buffer) {
            continue;
        }

        let codigo = std::mem::take(&mut buffer);
        if codigo.trim().is_empty() {
            continue;
        }
        match Parser::desde_codigo(&codigo).and_then(|mut p| p.parsear_programa()) {
            Ok(programa) => match interprete.ejecutar_programa_con_resultado(&programa) {
                Ok(Some(Value::Nulo)) | Ok(None) => {}
                Ok(Some(valor)) => println!("{}", valor.a_repr()),
                Err(Error::Salir(codigo)) => return Err(codigo as u8),
                Err(error) => eprintln!("{}", error),
            },
            Err(error) => eprintln!("{}", error),
        }
    }
    Ok(())
}

/// Cuenta llaves para permitir escribir bloques de varias líneas en el REPL.
fn balanceado(texto: &str) -> bool {
    let mut profundidad = 0i32;
    let mut en_texto = false;
    let mut comilla = '"';
    let mut escape = false;
    for c in texto.chars() {
        if en_texto {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == comilla {
                en_texto = false;
            }
            continue;
        }
        match c {
            '"' | '\'' => {
                en_texto = true;
                comilla = c;
            }
            '{' => profundidad += 1,
            '}' => profundidad -= 1,
            _ => {}
        }
    }
    profundidad <= 0 && !en_texto
}

// ----------------------------------------------------------------------
// Paquetes
// ----------------------------------------------------------------------

fn directorio_actual() -> Result<PathBuf, u8> {
    match env::current_dir() {
        Ok(ruta) => Ok(fs::canonicalize(&ruta).unwrap_or(ruta)),
        Err(error) => {
            eprintln!("error: no se pudo obtener el directorio actual: {}", error);
            Err(1)
        }
    }
}

fn comando_init(nombre: Option<&str>) -> u8 {
    let Some(nombre) = nombre else {
        eprintln!("error: falta el nombre del módulo");
        eprintln!("uso: vaca init <modulo>");
        eprintln!("ejemplo: vaca init ejemplo.com/mi-api");
        return 1;
    };
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::paquetes::init(&raiz, nombre) {
        Ok(modulo) => {
            let manifiesto = modulo
                .manifiesto
                .strip_prefix(&raiz)
                .unwrap_or(&modulo.manifiesto);
            let entrada = modulo
                .entrada
                .strip_prefix(&raiz)
                .unwrap_or(&modulo.entrada);
            println!("Módulo {} creado", modulo.nombre);
            println!("  {}", manifiesto.display());
            println!("  {}", entrada.display());
            println!();
            println!("Ejecuta `vaca` para iniciarlo.");
            println!("Añade dependencias con: vaca agregar dominio/usuario/repo@1.0.0");
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn comando_construir() -> u8 {
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::bundle::construir(&raiz) {
        Ok(artefacto) => {
            let ruta = artefacto
                .ruta
                .strip_prefix(&raiz)
                .unwrap_or(&artefacto.ruta);
            println!(
                "Empaquetado {} ({:.1} KiB)",
                ruta.display(),
                artefacto.bytes as f64 / 1024.0
            );
            println!("  módulo:  {}", artefacto.modulo);
            println!("  entrada: {}", artefacto.inicio);
            println!(
                "  sha256:  {}",
                artefacto.hash.trim_start_matches("sha256:")
            );
            println!(
                "  copia este archivo al servidor y ejecuta: vaca {}",
                artefacto
                    .ruta
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "programa.vaca".to_string())
            );
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn comando_instalar() -> u8 {
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::paquetes::instalar(&raiz, false) {
        Ok(resultado) => {
            if resultado.instalados.is_empty() {
                println!("No hay dependencias que instalar.");
            } else {
                for (modulo, version) in &resultado.instalados {
                    println!("{} {}", modulo, version);
                }
                println!("Instalados {} paquete(s).", resultado.instalados.len());
            }
            for modulo in &resultado.reemplazados {
                println!("{} -> reemplazo local", modulo);
            }
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn comando_agregar(argumento: &str) -> u8 {
    let (modulo, version) = match argumento.split_once('@') {
        Some((modulo, version)) => (modulo, Some(version)),
        None => (argumento, None),
    };
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::paquetes::agregar(&raiz, modulo, version) {
        Ok(version) => {
            println!("Añadido {} ^{}", modulo, version);
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn comando_actualizar() -> u8 {
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::paquetes::instalar(&raiz, true) {
        Ok(resultado) => {
            if resultado.instalados.is_empty() {
                println!("No hay dependencias que actualizar.");
            } else {
                for (modulo, version) in &resultado.instalados {
                    println!("{} {}", modulo, version);
                }
                println!("Actualizados {} paquete(s).", resultado.instalados.len());
            }
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn comando_listar() -> u8 {
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::paquetes::listar(&raiz) {
        Ok(texto) => {
            print!("{}", texto);
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}

fn comando_limpiar() -> u8 {
    let raiz = match directorio_actual() {
        Ok(raiz) => raiz,
        Err(codigo) => return codigo,
    };
    match vaca::paquetes::limpiar(&raiz) {
        Ok(()) => {
            println!("Dependencias y lockfile eliminados.");
            0
        }
        Err(error) => {
            eprintln!("{}", error);
            1
        }
    }
}
