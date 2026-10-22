//! Sistema de módulos y paquetes al estilo de Go.
//!
//! Un proyecto declara su nombre, su archivo de entrada y sus dependencias en
//! `vaca.mod` (texto, como `go.mod`). Cada dependencia es una ruta de módulo
//! (`ejemplo.com/usuario/lib`) y una restricción de versión semver.
//! `vaca instalar` descarga las etiquetas (`tags`) del repositorio git, elige
//! la mejor versión, copia el código a `.vaca/paquetes/` y fija versión, commit
//! y hash en `vaca.lock`.
//!
//! El código importado se resuelve en tiempo de ejecución desde ese
//! directorio, así que un proyecto instalado funciona sin red. También se
//! admiten importaciones del propio módulo (`importar "ejemplo.com/app/util"`).

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use crate::error::Error;
use crate::value::Value;

pub const MANIFIESTO: &str = "vaca.mod";
/// Formato anterior en JSON. Se sigue leyendo para no romper proyectos.
pub const MANIFIESTO_LEGADO: &str = "vaca.paquete";
pub const BLOQUEO: &str = "vaca.lock";
pub const DIR_VENDOR: &str = ".vaca/paquetes";
/// Archivo que se ejecuta al lanzar el módulo sin argumentos.
pub const ENTRADA_POR_DEFECTO: &str = "iniciar.vaca";
/// Archivos que puede exponer un paquete, en orden de preferencia.
const ENTRADAS: [&str; 3] = ["paquete.vaca", "index.vaca", "modulo.vaca"];

// ----------------------------------------------------------------------
// Versiones y restricciones
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub mayor: u64,
    pub menor: u64,
    pub parche: u64,
}

impl Version {
    pub fn parse(texto: &str) -> Option<Version> {
        let texto = texto.trim().trim_start_matches('v');
        let mut partes = texto.split('.');
        let mayor = partes.next()?.parse().ok()?;
        let menor = partes.next()?.parse().ok()?;
        let parche = partes.next()?.parse().ok()?;
        if partes.next().is_some() {
            return None;
        }
        Some(Version {
            mayor,
            menor,
            parche,
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.mayor, self.menor, self.parche)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Restriccion {
    Cualquiera,
    Exacta(Version),
    Compatible(Version),
    Aproximada(Version),
    MayorIgual(Version),
    Menor(Version),
}

impl Restriccion {
    pub fn parse(texto: &str) -> Option<Restriccion> {
        let texto = texto.trim();
        if texto.is_empty() || texto == "*" || texto.eq_ignore_ascii_case("latest") {
            return Some(Restriccion::Cualquiera);
        }
        if let Some(resto) = texto.strip_prefix(">=") {
            return Some(Restriccion::MayorIgual(Version::parse(resto)?));
        }
        if let Some(resto) = texto.strip_prefix('<') {
            return Some(Restriccion::Menor(Version::parse(resto)?));
        }
        if let Some(resto) = texto.strip_prefix('^') {
            return Some(Restriccion::Compatible(Version::parse(resto)?));
        }
        if let Some(resto) = texto.strip_prefix('~') {
            return Some(Restriccion::Aproximada(Version::parse(resto)?));
        }
        Some(Restriccion::Exacta(Version::parse(texto)?))
    }

    pub fn cumple(&self, version: &Version) -> bool {
        match self {
            Restriccion::Cualquiera => true,
            Restriccion::Exacta(v) => version == v,
            Restriccion::Compatible(v) => {
                if version < v {
                    return false;
                }
                if v.mayor > 0 {
                    version.mayor == v.mayor
                } else {
                    // 0.x: la compatibilidad se rompe al cambiar el menor.
                    version.mayor == 0 && version.menor == v.menor
                }
            }
            Restriccion::Aproximada(v) => {
                version >= v && version.mayor == v.mayor && version.menor == v.menor
            }
            Restriccion::MayorIgual(v) => version >= v,
            Restriccion::Menor(v) => version < v,
        }
    }

    pub fn texto(&self) -> String {
        match self {
            Restriccion::Cualquiera => "*".to_string(),
            Restriccion::Exacta(v) => v.to_string(),
            Restriccion::Compatible(v) => format!("^{}", v),
            Restriccion::Aproximada(v) => format!("~{}", v),
            Restriccion::MayorIgual(v) => format!(">={}", v),
            Restriccion::Menor(v) => format!("<{}", v),
        }
    }
}

// ----------------------------------------------------------------------
// Manifiesto y lock
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct Manifiesto {
    pub nombre: String,
    pub version: String,
    pub inicio: String,
    pub dependencias: BTreeMap<String, String>,
    pub reemplazos: BTreeMap<String, String>,
    pub origenes: BTreeMap<String, String>,
}

impl Manifiesto {
    /// Busca `vaca.mod` y, si no existe, el antiguo `vaca.paquete`.
    pub fn ruta_en(raiz: &Path) -> Option<PathBuf> {
        for nombre in [MANIFIESTO, MANIFIESTO_LEGADO] {
            let ruta = raiz.join(nombre);
            if ruta.is_file() {
                return Some(ruta);
            }
        }
        None
    }

    /// Archivo de entrada del módulo (`iniciar.vaca` si no se declara otro).
    pub fn entrada(&self) -> String {
        if self.inicio.is_empty() {
            ENTRADA_POR_DEFECTO.to_string()
        } else {
            self.inicio.clone()
        }
    }

    pub fn leer(ruta: &Path) -> Result<Manifiesto, Error> {
        let texto = fs::read_to_string(ruta).map_err(|e| {
            Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e))
        })?;
        let origen = ruta.display().to_string();
        if texto.trim_start().starts_with('{') {
            Manifiesto::leer_json(&texto, ruta)
        } else {
            Manifiesto::leer_texto(&texto, &origen)
        }
    }

    /// Interpreta el contenido de un `vaca.mod` (o del JSON antiguo).
    pub fn desde_texto(texto: &str) -> Result<Manifiesto, Error> {
        if texto.trim_start().starts_with('{') {
            Manifiesto::leer_json(texto, Path::new(MANIFIESTO))
        } else {
            Manifiesto::leer_texto(texto, MANIFIESTO)
        }
    }

    fn leer_texto(texto: &str, origen: &str) -> Result<Manifiesto, Error> {
        let mut manifiesto = Manifiesto::default();
        for (indice, linea) in texto.lines().enumerate() {
            let numero = indice + 1;
            let linea = linea.trim();
            if linea.is_empty() || linea.starts_with('#') || linea.starts_with("//") {
                continue;
            }
            let (clave, resto) = match linea.split_once(char::is_whitespace) {
                Some((clave, resto)) => (clave, resto.trim()),
                None => (linea, ""),
            };
            let error = |detalle: String| {
                Error::ejecucion(format!("'{}': línea {}: {}", origen, numero, detalle))
            };
            match clave {
                "modulo" | "módulo" | "nombre" => manifiesto.nombre = resto.to_string(),
                "version" | "versión" => manifiesto.version = resto.to_string(),
                "inicio" | "entrada" => manifiesto.inicio = resto.to_string(),
                "requiere" => {
                    let mut partes = resto.split_whitespace();
                    let (Some(modulo), Some(version), None) =
                        (partes.next(), partes.next(), partes.next())
                    else {
                        return Err(error(
                            "'requiere' necesita una ruta y una versión: requiere dominio/usuario/repo ^1.2.0"
                                .to_string(),
                        ));
                    };
                    validar_modulo(modulo).map_err(|e| error(e.to_string()))?;
                    Restriccion::parse(version).ok_or_else(|| {
                        error(format!("versión inválida '{}' para '{}'", version, modulo))
                    })?;
                    manifiesto
                        .dependencias
                        .insert(modulo.to_string(), version.to_string());
                }
                "reemplaza" => {
                    let mut partes = resto.split_whitespace();
                    let (Some(modulo), Some(destino), None) =
                        (partes.next(), partes.next(), partes.next())
                    else {
                        return Err(error(
                            "'reemplaza' necesita una ruta de módulo y un directorio"
                                .to_string(),
                        ));
                    };
                    validar_modulo(modulo).map_err(|e| error(e.to_string()))?;
                    manifiesto
                        .reemplazos
                        .insert(modulo.to_string(), destino.to_string());
                }
                "origen" => {
                    let mut partes = resto.split_whitespace();
                    let (Some(modulo), Some(url), None) =
                        (partes.next(), partes.next(), partes.next())
                    else {
                        return Err(error(
                            "'origen' necesita una ruta de módulo y una URL".to_string(),
                        ));
                    };
                    validar_modulo(modulo).map_err(|e| error(e.to_string()))?;
                    validar_origen(url).map_err(|e| error(e.to_string()))?;
                    manifiesto
                        .origenes
                        .insert(modulo.to_string(), url.to_string());
                }
                otra => {
                    return Err(error(format!(
                        "directiva desconocida '{}'; usa modulo, version, inicio, requiere, reemplaza u origen",
                        otra
                    )))
                }
            }
        }
        if !manifiesto.nombre.is_empty() {
            validar_nombre_modulo(&manifiesto.nombre)
                .map_err(|e| Error::ejecucion(format!("'{}': {}", origen, e)))?;
        }
        Ok(manifiesto)
    }

    fn leer_json(texto: &str, ruta: &Path) -> Result<Manifiesto, Error> {
        let valor = crate::modulos::json::parse(texto)
            .map_err(|e| Error::ejecucion(format!("'{}': {}", ruta.display(), e)))?;
        let mapa = match valor {
            Value::Diccionario(mapa) => mapa,
            _ => {
                return Err(Error::ejecucion(format!(
                    "'{}' debe contener un objeto JSON",
                    ruta.display()
                )))
            }
        };
        let mapa = mapa.lock().unwrap().clone();

        let mut manifiesto = Manifiesto::default();
        if let Some(valor) = mapa.get("nombre") {
            manifiesto.nombre = valor.a_texto();
        }
        if let Some(valor) = mapa.get("version") {
            manifiesto.version = valor.a_texto();
        }
        if let Some(valor) = mapa.get("inicio") {
            manifiesto.inicio = valor.a_texto();
        }
        manifiesto.dependencias = leer_mapa(&mapa, "dependencias")?;
        manifiesto.reemplazos = leer_mapa(&mapa, "reemplazos")?;
        manifiesto.origenes = leer_mapa(&mapa, "origenes")?;

        for modulo in manifiesto.dependencias.keys() {
            validar_modulo(modulo)?;
        }
        for modulo in manifiesto.reemplazos.keys() {
            validar_modulo(modulo)?;
        }
        Ok(manifiesto)
    }

    pub fn escribir(&self, ruta: &Path) -> Result<(), Error> {
        let nombre = if self.nombre.is_empty() {
            "proyecto"
        } else {
            self.nombre.as_str()
        };
        let mut texto = String::from("# Módulo de Vaca\n");
        texto.push_str(&format!("modulo {}\n", nombre));
        texto.push_str(&format!(
            "version {}\n",
            if self.version.is_empty() {
                "0.1.0"
            } else {
                &self.version
            }
        ));
        texto.push_str(&format!("inicio {}\n", self.entrada()));
        if !self.dependencias.is_empty() {
            texto.push('\n');
            for (modulo, restriccion) in &self.dependencias {
                texto.push_str(&format!("requiere {} {}\n", modulo, restriccion));
            }
        }
        if !self.reemplazos.is_empty() {
            texto.push('\n');
            for (modulo, destino) in &self.reemplazos {
                texto.push_str(&format!("reemplaza {} {}\n", modulo, destino));
            }
        }
        if !self.origenes.is_empty() {
            texto.push('\n');
            for (modulo, url) in &self.origenes {
                texto.push_str(&format!("origen {} {}\n", modulo, url));
            }
        }
        fs::write(ruta, texto).map_err(|e| {
            Error::ejecucion(format!("no se pudo escribir '{}': {}", ruta.display(), e))
        })
    }

    pub fn url_origen(&self, modulo: &str) -> Result<String, Error> {
        let url = match self.origenes.get(modulo) {
            Some(url) => url.clone(),
            None => format!("https://{}.git", modulo),
        };
        validar_origen(&url)?;
        Ok(url)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Bloqueado {
    pub version: String,
    pub commit: String,
    pub hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct Bloqueo {
    pub paquetes: BTreeMap<String, Bloqueado>,
}

impl Bloqueo {
    pub fn leer(ruta: &Path) -> Result<Bloqueo, Error> {
        if !ruta.exists() {
            return Ok(Bloqueo::default());
        }
        let texto = fs::read_to_string(ruta).map_err(|e| {
            Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e))
        })?;
        let valor = crate::modulos::json::parse(&texto)
            .map_err(|e| Error::ejecucion(format!("'{}': {}", ruta.display(), e)))?;
        let mapa = match valor {
            Value::Diccionario(mapa) => mapa.lock().unwrap().clone(),
            _ => {
                return Err(Error::ejecucion(format!(
                    "'{}' debe contener un objeto JSON",
                    ruta.display()
                )))
            }
        };
        let mut bloqueo = Bloqueo::default();
        if let Some(Value::Diccionario(paquetes)) = mapa.get("paquetes") {
            for (modulo, valor) in paquetes.lock().unwrap().iter() {
                if let Value::Diccionario(datos) = valor {
                    let datos = datos.lock().unwrap();
                    let texto =
                        |clave: &str| datos.get(clave).map(|v| v.a_texto()).unwrap_or_default();
                    bloqueo.paquetes.insert(
                        modulo.clone(),
                        Bloqueado {
                            version: texto("version"),
                            commit: texto("commit"),
                            hash: texto("hash"),
                        },
                    );
                }
            }
        }
        Ok(bloqueo)
    }

    pub fn escribir(&self, ruta: &Path) -> Result<(), Error> {
        let mut texto = String::from("{\n  \"paquetes\": {");
        let mut primero = true;
        for (modulo, datos) in &self.paquetes {
            if !primero {
                texto.push(',');
            }
            primero = false;
            texto.push_str(&format!("\n    {}: {{", comillas(modulo)));
            texto.push_str(&format!(
                "\"version\": {}, \"commit\": {}, \"hash\": {}",
                comillas(&datos.version),
                comillas(&datos.commit),
                comillas(&datos.hash)
            ));
            texto.push('}');
        }
        if !primero {
            texto.push('\n');
        }
        texto.push_str("  }\n}\n");
        fs::write(ruta, texto).map_err(|e| {
            Error::ejecucion(format!("no se pudo escribir '{}': {}", ruta.display(), e))
        })
    }
}

// ----------------------------------------------------------------------
// Rutas del proyecto
// ----------------------------------------------------------------------

pub fn raiz_proyecto(desde: &Path) -> Option<PathBuf> {
    let mut actual = Some(desde);
    let mut con_manifiesto = None;
    while let Some(dir) = actual {
        if dir.join(DIR_VENDOR).is_dir() {
            return Some(dir.to_path_buf());
        }
        let dentro_de_vendor = dir.components().any(|c| c.as_os_str() == ".vaca");
        if !dentro_de_vendor && Manifiesto::ruta_en(dir).is_some() && con_manifiesto.is_none() {
            con_manifiesto = Some(dir.to_path_buf());
        }
        actual = dir.parent();
    }
    con_manifiesto
}

pub fn dir_vendor(raiz: &Path) -> PathBuf {
    raiz.join(DIR_VENDOR)
}

pub fn dir_paquete(raiz: &Path, modulo: &str) -> PathBuf {
    let mut ruta = dir_vendor(raiz);
    for segmento in modulo.split('/') {
        ruta.push(segmento);
    }
    ruta
}

/// Solo se aceptan orígenes seguros. `ext::` de git ejecuta comandos.
fn validar_origen(url: &str) -> Result<(), Error> {
    let permitido = url.starts_with("https://")
        || url.starts_with("ssh://")
        || url.starts_with("file://")
        || url.starts_with("git@");
    if permitido && !url.starts_with('-') {
        return Ok(());
    }
    Err(Error::ejecucion(format!(
        "origen de paquete no permitido '{}': usa https://, ssh:// o file://",
        url
    )))
}

/// Una ruta es de paquete si parece un dominio y no un archivo `.vaca`.
pub fn es_ruta_paquete(modulo: &str) -> bool {
    if modulo.ends_with(".vaca")
        || modulo.starts_with('.')
        || modulo.starts_with('/')
        || modulo.starts_with("file:")
    {
        return false;
    }
    match modulo.split('/').next() {
        Some(primero) => modulo.contains('/') && primero.contains('.') && !primero.contains('@'),
        None => false,
    }
}

pub fn validar_modulo(modulo: &str) -> Result<(), Error> {
    let error = || {
        Error::ejecucion(format!(
            "ruta de paquete inválida '{}': usa 'dominio/usuario/repo' (sin espacios, '..' ni '@')",
            modulo
        ))
    };
    if modulo.is_empty() || modulo.len() > 200 {
        return Err(error());
    }
    let segmentos: Vec<&str> = modulo.split('/').collect();
    if segmentos.len() < 2 {
        return Err(error());
    }
    if !segmentos[0].contains('.') {
        return Err(error());
    }
    for segmento in &segmentos {
        if segmento.is_empty() || *segmento == "." || *segmento == ".." {
            return Err(error());
        }
        if !segmento
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(error());
        }
    }
    Ok(())
}

/// Busca el archivo de entrada de un paquete instalado.
pub fn resolver_importacion(modulo: &str, ruta_base: &Path) -> Result<PathBuf, Error> {
    let raiz = raiz_proyecto(ruta_base).ok_or_else(|| {
        Error::ejecucion(format!(
            "el paquete '{}' necesita un módulo con '{}' y ejecutar `vaca instalar`",
            modulo, MANIFIESTO
        ))
    })?;

    if let Some(ruta) = resolver_modulo_propio(&raiz, modulo)? {
        return Ok(ruta);
    }

    validar_modulo(modulo)?;
    let dir = dir_paquete(&raiz, modulo);
    buscar_entrada(&dir).ok_or_else(|| {
        Error::ejecucion(format!(
            "el paquete '{}' no está instalado o no tiene un archivo de entrada ({}); ejecuta `vaca instalar` en '{}'",
            modulo,
            ENTRADAS.join(", "),
            raiz.display()
        ))
    })
}

/// Resuelve una importación que pertenece al propio módulo.
///
/// `importar "ejemplo.com/app/util"` busca `util.vaca` o `util/paquete.vaca`
/// dentro de la raíz del módulo. Devuelve `None` si la ruta no empieza por el
/// nombre del módulo.
pub fn resolver_modulo_propio(raiz: &Path, modulo: &str) -> Result<Option<PathBuf>, Error> {
    let Some(ruta_manifiesto) = Manifiesto::ruta_en(raiz) else {
        return Ok(None);
    };
    let manifiesto = Manifiesto::leer(&ruta_manifiesto)?;
    if manifiesto.nombre.is_empty() {
        return Ok(None);
    }
    let Some(resto) = modulo.strip_prefix(manifiesto.nombre.as_str()) else {
        return Ok(None);
    };
    if resto.is_empty() {
        return Err(Error::ejecucion(format!(
            "no se puede importar el módulo completo '{}'; importa un archivo o un subpaquete",
            modulo
        )));
    }
    let Some(resto) = resto.strip_prefix('/') else {
        return Ok(None);
    };
    if resto.is_empty() {
        return Err(Error::ejecucion(format!(
            "importación inválida '{}': falta la ruta dentro del módulo",
            modulo
        )));
    }
    validar_ruta_relativa(resto)?;

    let base: PathBuf = resto
        .split('/')
        .fold(raiz.to_path_buf(), |ruta, parte| ruta.join(parte));
    buscar_entrada_propia(&base).map(Some).ok_or_else(|| {
        Error::ejecucion(format!(
            "el módulo '{}' no contiene '{}' (se buscó {}.vaca y {}/paquete.vaca)",
            manifiesto.nombre, resto, resto, resto
        ))
    })
}

fn validar_ruta_relativa(ruta: &str) -> Result<(), Error> {
    if ruta.is_empty() || ruta.len() > 200 {
        return Err(Error::ejecucion(format!(
            "ruta de módulo inválida '{}'",
            ruta
        )));
    }
    for segmento in ruta.split('/') {
        if segmento.is_empty() || segmento == "." || segmento == ".." {
            return Err(Error::ejecucion(format!(
                "ruta de módulo inválida '{}': no se permite '..'",
                ruta
            )));
        }
        if !segmento
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(Error::ejecucion(format!(
                "ruta de módulo inválida '{}'",
                ruta
            )));
        }
    }
    Ok(())
}

/// Nombre de módulo: admite rutas con o sin dominio (`app`, `ejemplo.com/app`).
pub fn validar_nombre_modulo(nombre: &str) -> Result<(), Error> {
    if nombre.is_empty() || nombre.len() > 200 {
        return Err(Error::ejecucion(
            "el nombre del módulo no puede estar vacío".to_string(),
        ));
    }
    validar_ruta_relativa(nombre)
}

fn buscar_entrada(base: &Path) -> Option<PathBuf> {
    for entrada in ENTRADAS {
        let candidato = base.join(entrada);
        if candidato.is_file() {
            return Some(candidato);
        }
    }
    let candidato = base.with_extension("vaca");
    if candidato.is_file() {
        Some(candidato)
    } else {
        None
    }
}

/// Para subpaquetes propios se prefiere el archivo suelto (`util.vaca`)
/// antes que un directorio (`util/paquete.vaca`).
fn buscar_entrada_propia(base: &Path) -> Option<PathBuf> {
    if base
        .extension()
        .is_some_and(|extension| extension == "vaca")
        && base.is_file()
    {
        return Some(base.to_path_buf());
    }
    let archivo = base.with_extension("vaca");
    if archivo.is_file() {
        return Some(archivo);
    }
    for entrada in ENTRADAS {
        let candidato = base.join(entrada);
        if candidato.is_file() {
            return Some(candidato);
        }
    }
    None
}

/// Último segmento de la ruta, útil para nombres de artefactos.
pub fn nombre_corto(modulo: &str) -> String {
    modulo
        .rsplit('/')
        .next()
        .filter(|nombre| !nombre.is_empty())
        .unwrap_or("programa")
        .to_string()
}

// ----------------------------------------------------------------------
// Instalación
// ----------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Resultado {
    pub instalados: Vec<(String, String)>,
    pub reemplazados: Vec<String>,
}

struct Resuelto {
    version: Option<Version>,
    commit: String,
    hash: String,
}

pub fn instalar(raiz: &Path, actualizar: bool) -> Result<Resultado, Error> {
    let ruta_manifiesto = Manifiesto::ruta_en(raiz).ok_or_else(|| {
        Error::ejecucion(format!(
            "no hay '{}' en '{}'; ejecuta `vaca init <modulo>` primero",
            MANIFIESTO,
            raiz.display()
        ))
    })?;
    let manifiesto = Manifiesto::leer(&ruta_manifiesto)?;
    let ruta_bloqueo = raiz.join(BLOQUEO);
    let bloqueo_anterior = if actualizar {
        Bloqueo::default()
    } else {
        Bloqueo::leer(&ruta_bloqueo)?
    };

    // Requisitos acumulados por módulo (incluye dependencias transitivas).
    let mut requisitos: BTreeMap<String, Vec<Restriccion>> = BTreeMap::new();
    for (modulo, restriccion) in &manifiesto.dependencias {
        let restriccion = Restriccion::parse(restriccion).ok_or_else(|| {
            Error::ejecucion(format!(
                "versión inválida '{}' para '{}'",
                restriccion, modulo
            ))
        })?;
        requisitos
            .entry(modulo.clone())
            .or_default()
            .push(restriccion);
    }

    let mut resueltos: BTreeMap<String, Resuelto> = BTreeMap::new();
    let mut resultado = Resultado::default();
    let mut pendientes: VecDeque<String> = requisitos.keys().cloned().collect();
    let mut vueltas = 0;

    loop {
        while let Some(modulo) = pendientes.pop_front() {
            if resueltos.contains_key(&modulo) {
                continue;
            }
            let restricciones = requisitos.get(&modulo).cloned().unwrap_or_default();

            if let Some(ruta_local) = manifiesto.reemplazos.get(&modulo) {
                let origen = resolver_ruta_local(raiz, ruta_local);
                let destino = dir_paquete(raiz, &modulo);
                copiar_paquete(&origen, &destino)?;
                let hash = hash_directorio(&destino)?;
                resueltos.insert(
                    modulo.clone(),
                    Resuelto {
                        version: None,
                        commit: String::new(),
                        hash,
                    },
                );
                resultado.reemplazados.push(modulo.clone());
                continue;
            }

            validar_modulo(&modulo)?;
            let url = manifiesto.url_origen(&modulo)?;
            let destino = dir_paquete(raiz, &modulo);

            // Reutiliza la versión fijada si sigue satisfaciendo los requisitos.
            let fijado = bloqueo_anterior.paquetes.get(&modulo);
            let version_fijada = fijado
                .and_then(|datos| Version::parse(&datos.version))
                .filter(|version| restricciones.iter().all(|r| r.cumple(version)));

            match version_fijada {
                Some(version) => {
                    // Si el paquete ya está copiado y su hash coincide, no hace falta red.
                    let coincide = destino.is_dir()
                        && fijado
                            .map(|datos| {
                                hash_directorio(&destino).ok().as_deref()
                                    == Some(datos.hash.as_str())
                            })
                            .unwrap_or(false);
                    if !coincide {
                        let commit = clonar(&url, &version, &destino)?;
                        if let Some(datos) = fijado {
                            if !datos.commit.is_empty() && datos.commit != commit {
                                return Err(Error::ejecucion(format!(
                                    "el paquete '{}' cambió: el lock fija {} y el tag {} apunta a {}; usa `vaca actualizar` si es intencional",
                                    modulo, datos.commit, version, commit
                                )));
                            }
                        }
                    }
                    let commit = if coincide {
                        fijado.map(|d| d.commit.clone()).unwrap_or_default()
                    } else {
                        git_salida(
                            Command::new("git")
                                .arg("-C")
                                .arg(&destino)
                                .args(["rev-parse", "HEAD"]),
                        )
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                    };
                    let hash = hash_directorio(&destino)?;
                    resueltos.insert(
                        modulo.clone(),
                        Resuelto {
                            version: Some(version),
                            commit,
                            hash,
                        },
                    );
                }
                None => {
                    let disponibles = versiones_git(&url)?;
                    let (version, _) =
                        elegir_version(&restricciones, &disponibles).ok_or_else(|| {
                            let textos: Vec<String> =
                                restricciones.iter().map(|r| r.texto()).collect();
                            Error::ejecucion(format!(
                                "no hay una versión de '{}' que cumpla {} (disponibles: {})",
                                modulo,
                                textos.join(", "),
                                disponibles
                                    .iter()
                                    .map(|(v, _)| v.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ))
                        })?;
                    let commit = clonar(&url, &version, &destino)?;
                    let hash = hash_directorio(&destino)?;
                    resueltos.insert(
                        modulo.clone(),
                        Resuelto {
                            version: Some(version),
                            commit,
                            hash,
                        },
                    );
                }
            }

            // Dependencias transitivas del paquete recién instalado.
            if let Some(manifiesto_paquete) = Manifiesto::ruta_en(&destino) {
                let dependiente = Manifiesto::leer(&manifiesto_paquete)?;
                for (dependencia, restriccion) in dependiente.dependencias {
                    let restriccion = Restriccion::parse(&restriccion).ok_or_else(|| {
                        Error::ejecucion(format!(
                            "versión inválida '{}' en '{}'",
                            restriccion,
                            manifiesto_paquete.display()
                        ))
                    })?;
                    let lista = requisitos.entry(dependencia.clone()).or_default();
                    if !lista.contains(&restriccion) {
                        lista.push(restriccion);
                    }
                    if !resueltos.contains_key(&dependencia) {
                        pendientes.push_back(dependencia);
                    }
                }
            }

            resultado.instalados.push((
                modulo.clone(),
                resueltos
                    .get(&modulo)
                    .map(|r| {
                        r.version
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "local".into())
                    })
                    .unwrap_or_default(),
            ));
        }

        // ¿Alguna restricción nueva invalida una versión ya elegida?
        let mut rehacer = None;
        for (modulo, restricciones) in &requisitos {
            if let Some(resuelto) = resueltos.get(modulo) {
                if let Some(version) = resuelto.version {
                    if !restricciones.iter().all(|r| r.cumple(&version)) {
                        rehacer = Some(modulo.clone());
                        break;
                    }
                }
            }
        }
        match rehacer {
            Some(modulo) => {
                resueltos.remove(&modulo);
                pendientes.push_back(modulo);
            }
            None => break,
        }

        vueltas += 1;
        if vueltas > 50 {
            return Err(Error::ejecucion(
                "no se pudieron estabilizar las versiones: revisa un ciclo de dependencias o restricciones incompatibles",
            ));
        }
    }

    // Aviso de dependencias declaradas pero no instaladas (no debería ocurrir).
    for modulo in requisitos.keys() {
        if !resueltos.contains_key(modulo) {
            return Err(Error::ejecucion(format!(
                "la dependencia '{}' no se pudo resolver",
                modulo
            )));
        }
    }

    let mut bloqueo = Bloqueo::default();
    for (modulo, resuelto) in &resueltos {
        bloqueo.paquetes.insert(
            modulo.clone(),
            Bloqueado {
                version: resuelto
                    .version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "local".to_string()),
                commit: resuelto.commit.clone(),
                hash: resuelto.hash.clone(),
            },
        );
    }
    bloqueo.escribir(&ruta_bloqueo)?;

    resultado.instalados.sort();
    resultado.reemplazados.sort();
    Ok(resultado)
}

pub fn agregar(raiz: &Path, modulo: &str, version: Option<&str>) -> Result<String, Error> {
    validar_modulo(modulo)?;
    let ruta_manifiesto = Manifiesto::ruta_en(raiz).ok_or_else(|| {
        Error::ejecucion(format!(
            "no hay '{}' en '{}'; ejecuta `vaca init <modulo>` primero",
            MANIFIESTO,
            raiz.display()
        ))
    })?;
    let mut manifiesto = Manifiesto::leer(&ruta_manifiesto)?;

    let version = match version {
        Some(v) => Version::parse(v)
            .ok_or_else(|| Error::ejecucion(format!("versión inválida '{}': usa X.Y.Z", v)))?,
        None => {
            let url = manifiesto.url_origen(modulo)?;
            let disponibles = versiones_git(&url)?;
            let restricciones = [Restriccion::Cualquiera];
            elegir_version(&restricciones, &disponibles)
                .map(|(v, _)| v)
                .ok_or_else(|| {
                    Error::ejecucion(format!(
                        "'{}' no tiene versiones etiquetadas (tags X.Y.Z)",
                        modulo
                    ))
                })?
        }
    };

    manifiesto
        .dependencias
        .insert(modulo.to_string(), format!("^{}", version));
    // Si el proyecto venía del formato JSON antiguo, se migra a `vaca.mod`.
    let ruta_salida = if ruta_manifiesto
        .extension()
        .is_some_and(|extension| extension == "paquete")
    {
        raiz.join(MANIFIESTO)
    } else {
        ruta_manifiesto
    };
    manifiesto.escribir(&ruta_salida)?;
    instalar(raiz, false)?;
    Ok(version.to_string())
}

pub fn listar(raiz: &Path) -> Result<String, Error> {
    let ruta_manifiesto = Manifiesto::ruta_en(raiz).ok_or_else(|| {
        Error::ejecucion(format!("no hay '{}' en '{}'", MANIFIESTO, raiz.display()))
    })?;
    let manifiesto = Manifiesto::leer(&ruta_manifiesto)?;
    let bloqueo = Bloqueo::leer(&raiz.join(BLOQUEO))?;
    let mut texto = String::new();
    if !manifiesto.nombre.is_empty() {
        texto.push_str(&format!("módulo:   {}\n", manifiesto.nombre));
    }
    texto.push_str(&format!("entrada:  {}\n", manifiesto.entrada()));
    if manifiesto.dependencias.is_empty() {
        texto.push_str("sin dependencias\n");
        return Ok(texto);
    }
    for (modulo, restriccion) in &manifiesto.dependencias {
        let estado = match bloqueo.paquetes.get(modulo) {
            Some(datos) => format!("{} ({})", datos.version, datos.hash),
            None => "no instalado".to_string(),
        };
        texto.push_str(&format!("{} {} -> {}\n", modulo, restriccion, estado));
    }
    Ok(texto)
}

pub fn limpiar(raiz: &Path) -> Result<(), Error> {
    for ruta in [raiz.join(DIR_VENDOR), raiz.join(BLOQUEO)] {
        if ruta.is_dir() {
            fs::remove_dir_all(&ruta).map_err(|e| {
                Error::ejecucion(format!("no se pudo borrar '{}': {}", ruta.display(), e))
            })?;
        } else if ruta.is_file() {
            fs::remove_file(&ruta).map_err(|e| {
                Error::ejecucion(format!("no se pudo borrar '{}': {}", ruta.display(), e))
            })?;
        }
    }
    Ok(())
}

pub struct ModuloCreado {
    pub nombre: String,
    pub manifiesto: PathBuf,
    pub entrada: PathBuf,
}

pub fn init(raiz: &Path, nombre: &str) -> Result<ModuloCreado, Error> {
    let nombre = nombre.trim().trim_end_matches('/');
    validar_nombre_modulo(nombre)?;
    if let Some(ruta) = Manifiesto::ruta_en(raiz) {
        return Err(Error::ejecucion(format!(
            "'{}' ya existe; no se sobrescribe",
            ruta.display()
        )));
    }
    let manifiesto = Manifiesto {
        nombre: nombre.to_string(),
        version: "0.1.0".to_string(),
        inicio: ENTRADA_POR_DEFECTO.to_string(),
        ..Manifiesto::default()
    };
    let ruta = raiz.join(MANIFIESTO);
    manifiesto.escribir(&ruta)?;

    let entrada = raiz.join(ENTRADA_POR_DEFECTO);
    if !entrada.exists() {
        let plantilla = format!(
            "# Punto de entrada del módulo {nombre}.\n# Ejecuta `vaca` en este directorio para iniciarlo.\n\nfn principal() {{\n    imprimir(\"Hola desde {nombre}\")\n}}\n"
        );
        fs::write(&entrada, plantilla).map_err(|e| {
            Error::ejecucion(format!(
                "no se pudo escribir '{}': {}",
                entrada.display(),
                e
            ))
        })?;
    }

    Ok(ModuloCreado {
        nombre: nombre.to_string(),
        manifiesto: ruta,
        entrada,
    })
}

// ----------------------------------------------------------------------
// Git y versiones
// ----------------------------------------------------------------------

fn versiones_git(url: &str) -> Result<Vec<(Version, String)>, Error> {
    let salida = git_salida(Command::new("git").args(["ls-remote", "--tags", "--refs", url]))?;
    let mut versiones = Vec::new();
    for linea in salida.lines() {
        let mut partes = linea.split_whitespace();
        let _hash = partes.next();
        let Some(referencia) = partes.next() else {
            continue;
        };
        let Some(tag) = referencia.strip_prefix("refs/tags/") else {
            continue;
        };
        if let Some(version) = Version::parse(tag) {
            versiones.push((version, tag.to_string()));
        }
    }
    versiones.sort_by_key(|(version, _)| std::cmp::Reverse(*version));
    versiones.dedup_by(|a, b| a.0 == b.0);
    Ok(versiones)
}

fn elegir_version(
    restricciones: &[Restriccion],
    disponibles: &[(Version, String)],
) -> Option<(Version, String)> {
    disponibles
        .iter()
        .find(|(version, _)| restricciones.iter().all(|r| r.cumple(version)))
        .cloned()
}

fn clonar(url: &str, version: &Version, destino: &Path) -> Result<String, Error> {
    if destino.exists() {
        fs::remove_dir_all(destino).map_err(|e| {
            Error::ejecucion(format!("no se pudo limpiar '{}': {}", destino.display(), e))
        })?;
    }
    let padre = destino
        .parent()
        .ok_or_else(|| Error::ejecucion("ruta de paquete inválida".to_string()))?;
    fs::create_dir_all(padre)
        .map_err(|e| Error::ejecucion(format!("no se pudo crear '{}': {}", padre.display(), e)))?;
    let temporal = padre.join(format!(".tmp-{}-{}", std::process::id(), nombre_temporal()));
    let _ = fs::remove_dir_all(&temporal);

    let mut ultimo_error = String::new();
    let mut clonado = false;
    for etiqueta in [format!("v{}", version), version.to_string()] {
        let salida = Command::new("git")
            .args([
                "clone", "--quiet", "--depth", "1", "--branch", &etiqueta, url,
            ])
            .arg(&temporal)
            .output()
            .map_err(|e| {
                Error::ejecucion(format!("no se pudo ejecutar git: {} (¿está instalado?)", e))
            })?;
        if salida.status.success() {
            clonado = true;
            break;
        }
        ultimo_error = String::from_utf8_lossy(&salida.stderr).trim().to_string();
        let _ = fs::remove_dir_all(&temporal);
    }
    if !clonado {
        return Err(Error::ejecucion(format!(
            "no se pudo descargar '{}' en la versión {}: {}",
            url, version, ultimo_error
        )));
    }

    let commit = git_salida(
        Command::new("git")
            .arg("-C")
            .arg(&temporal)
            .args(["rev-parse", "HEAD"]),
    )?
    .trim()
    .to_string();

    let _ = fs::remove_dir_all(temporal.join(".git"));
    fs::rename(&temporal, destino).map_err(|e| {
        Error::ejecucion(format!(
            "no se pudo mover '{}' a '{}': {}",
            temporal.display(),
            destino.display(),
            e
        ))
    })?;
    Ok(commit)
}

fn resolver_ruta_local(raiz: &Path, ruta: &str) -> PathBuf {
    let ruta = Path::new(ruta);
    if ruta.is_absolute() {
        ruta.to_path_buf()
    } else {
        raiz.join(ruta)
    }
}

fn copiar_paquete(origen: &Path, destino: &Path) -> Result<(), Error> {
    if !origen.is_dir() {
        return Err(Error::ejecucion(format!(
            "el reemplazo '{}' no es un directorio",
            origen.display()
        )));
    }
    if destino.exists() {
        fs::remove_dir_all(destino).map_err(|e| {
            Error::ejecucion(format!("no se pudo limpiar '{}': {}", destino.display(), e))
        })?;
    }
    copiar_recursivo(origen, destino).map_err(|e| {
        Error::ejecucion(format!(
            "no se pudo copiar '{}' a '{}': {}",
            origen.display(),
            destino.display(),
            e
        ))
    })
}

fn copiar_recursivo(origen: &Path, destino: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destino)?;
    for entrada in fs::read_dir(origen)? {
        let entrada = entrada?;
        let nombre = entrada.file_name();
        if nombre == ".git" || nombre == ".vaca" || nombre == "vaca.lock" {
            continue;
        }
        let ruta_origen = entrada.path();
        let ruta_destino = destino.join(&nombre);
        if ruta_origen.is_dir() {
            copiar_recursivo(&ruta_origen, &ruta_destino)?;
        } else {
            fs::copy(&ruta_origen, &ruta_destino)?;
        }
    }
    Ok(())
}

fn git_salida(comando: &mut Command) -> Result<String, Error> {
    comando.env("GIT_TERMINAL_PROMPT", "0");
    let salida = comando.output().map_err(|e| {
        Error::ejecucion(format!(
            "no se pudo ejecutar git: {} (instala git para usar paquetes)",
            e
        ))
    })?;
    if !salida.status.success() {
        let error = String::from_utf8_lossy(&salida.stderr).trim().to_string();
        return Err(Error::ejecucion(format!(
            "git falló: {}",
            if error.is_empty() {
                "sin detalles".to_string()
            } else {
                error
            }
        )));
    }
    Ok(String::from_utf8_lossy(&salida.stdout).to_string())
}

fn nombre_temporal() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{}", nanos)
}

// ----------------------------------------------------------------------
// Integridad
// ----------------------------------------------------------------------

fn hash_directorio(dir: &Path) -> Result<String, Error> {
    let mut archivos = Vec::new();
    listar_archivos(dir, &mut archivos)?;
    archivos.sort();

    let mut contexto = ring::digest::Context::new(&ring::digest::SHA256);
    for ruta in archivos {
        let relativa = ruta.strip_prefix(dir).unwrap_or(&ruta);
        let nombre = relativa.to_string_lossy().replace('\\', "/");
        contexto.update(nombre.as_bytes());
        contexto.update(&[0]);
        let contenido = fs::read(&ruta).map_err(|e| {
            Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e))
        })?;
        contexto.update(&contenido);
        contexto.update(&[0]);
    }
    let digesto = contexto.finish();
    Ok(format!("sha256:{}", hex(digesto.as_ref())))
}

fn listar_archivos(dir: &Path, salida: &mut Vec<PathBuf>) -> Result<(), Error> {
    for entrada in fs::read_dir(dir)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", dir.display(), e)))?
    {
        let entrada =
            entrada.map_err(|e| Error::ejecucion(format!("directorio inválido: {}", e)))?;
        let nombre = entrada.file_name();
        if nombre == ".git" || nombre == ".vaca" {
            continue;
        }
        let ruta = entrada.path();
        if ruta.is_dir() {
            listar_archivos(&ruta, salida)?;
        } else if ruta.is_file() {
            salida.push(ruta);
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut texto = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        texto.push_str(&format!("{:02x}", byte));
    }
    texto
}

/// SHA-256 de un bloque de bytes.
pub fn sha256_bytes(datos: &[u8]) -> [u8; 32] {
    let digesto = ring::digest::digest(&ring::digest::SHA256, datos);
    let mut salida = [0u8; 32];
    salida.copy_from_slice(digesto.as_ref());
    salida
}

/// SHA-256 en hexadecimal, para artefactos y mensajes.
pub fn sha256_hex(datos: &[u8]) -> String {
    hex(&sha256_bytes(datos))
}

// ----------------------------------------------------------------------
// JSON mínimo
// ----------------------------------------------------------------------

fn leer_mapa(
    mapa: &crate::value::MapaValores,
    clave: &str,
) -> Result<BTreeMap<String, String>, Error> {
    let mut resultado = BTreeMap::new();
    match mapa.get(clave) {
        None => {}
        Some(Value::Diccionario(interior)) => {
            for (k, v) in interior.lock().unwrap().iter() {
                resultado.insert(k.clone(), v.a_texto());
            }
        }
        Some(_) => {
            return Err(Error::ejecucion(format!(
                "'{}' debe ser un objeto JSON",
                clave
            )))
        }
    }
    Ok(resultado)
}

fn comillas(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len() + 2);
    salida.push('"');
    for c in texto.chars() {
        match c {
            '"' => salida.push_str("\\\""),
            '\\' => salida.push_str("\\\\"),
            '\n' => salida.push_str("\\n"),
            '\r' => salida.push_str("\\r"),
            '\t' => salida.push_str("\\t"),
            c if (c as u32) < 0x20 => salida.push_str(&format!("\\u{:04x}", c as u32)),
            c => salida.push(c),
        }
    }
    salida.push('"');
    salida
}

/// Normaliza una ruta para mensajes y para comprobar que no escapa del proyecto.
pub fn ruta_limpia(ruta: &Path) -> PathBuf {
    ruta.components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn versiones_y_restricciones() {
        let v = Version::parse("v1.2.3").expect("versión");
        assert_eq!(v.to_string(), "1.2.3");
        assert!(Restriccion::parse("^1.2.0").unwrap().cumple(&v));
        assert!(!Restriccion::parse("^2.0.0").unwrap().cumple(&v));
        assert!(Restriccion::parse("~1.2.0").unwrap().cumple(&v));
        assert!(!Restriccion::parse("~1.3.0").unwrap().cumple(&v));
        assert!(Restriccion::parse(">=1.0.0").unwrap().cumple(&v));
        assert!(!Restriccion::parse("<1.2.3").unwrap().cumple(&v));
        // 0.x: ^0.2.0 no acepta 0.3.0
        let cero = Version::parse("0.3.0").unwrap();
        assert!(!Restriccion::parse("^0.2.0").unwrap().cumple(&cero));
    }

    #[test]
    fn rechaza_origenes_peligrosos() {
        assert!(validar_origen("https://github.com/u/r.git").is_ok());
        assert!(validar_origen("file:///tmp/repo").is_ok());
        assert!(validar_origen("ssh://git@host/repo.git").is_ok());
        assert!(validar_origen("ext::sh -c whoami").is_err());
        assert!(validar_origen("http://inseguro/repo.git").is_err());
        assert!(validar_origen("-oProxyCommand=evil").is_err());
    }

    #[test]
    fn distingue_paquetes_de_archivos() {
        assert!(es_ruta_paquete("github.com/usuario/repo"));
        assert!(es_ruta_paquete("ejemplo.com/lib/sub"));
        assert!(!es_ruta_paquete("util.vaca"));
        assert!(!es_ruta_paquete("sub/util.vaca"));
        assert!(!es_ruta_paquete("./lib"));
        assert!(!es_ruta_paquete("http"));
    }

    #[test]
    fn valida_rutas_de_paquete() {
        assert!(validar_modulo("github.com/usuario/repo").is_ok());
        assert!(validar_modulo("ejemplo.com/lib").is_ok());
        assert!(validar_modulo("../lib").is_err());
        assert!(validar_modulo("github.com/../../etc").is_err());
        assert!(validar_modulo("github.com/usuario con espacio/repo").is_err());
        assert!(validar_modulo("sinpunto/repo").is_err());
    }

    #[test]
    fn manifiesto_ida_y_vuelta() {
        let manifiesto = Manifiesto {
            nombre: "ejemplo.com/app".to_string(),
            version: "1.0.0".to_string(),
            dependencias: BTreeMap::from([("ejemplo.com/lib".to_string(), "^1.2.0".to_string())]),
            origenes: BTreeMap::from([(
                "ejemplo.com/lib".to_string(),
                "file:///tmp/lib".to_string(),
            )]),
            ..Manifiesto::default()
        };

        let dir = std::env::temp_dir().join(format!("vaca-manifiesto-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join(MANIFIESTO);
        manifiesto.escribir(&ruta).unwrap();

        let leido = Manifiesto::leer(&ruta).unwrap();
        assert_eq!(leido.nombre, manifiesto.nombre);
        assert_eq!(leido.dependencias, manifiesto.dependencias);
        assert_eq!(leido.origenes, manifiesto.origenes);
        let _ = fs::remove_dir_all(&dir);
    }
}
