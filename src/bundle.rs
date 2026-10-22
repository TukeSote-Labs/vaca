//! Empaquetado de producción en un único archivo `.vaca`.
//!
//! `vaca construir` recorre el módulo y sus dependencias vendorizadas y genera
//! un archivo autocontenido con formato binario:
//!
//! ```text
//! VACABND1 | u32 nº entradas | (u16 ruta, u64 tamaño, bytes)* | sha256
//! ```
//!
//! El hash cubre todo lo anterior al propio hash. Al ejecutar el archivo,
//! `vaca` verifica la integridad y carga los módulos en memoria, sin tocar el
//! disco para cada importación.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use crate::error::Error;
use crate::paquetes::{self, Manifiesto};

pub const MAGIA: &[u8; 8] = b"VACABND1";
/// Límites defensivos al leer un bundle ajeno.
const MAX_ARCHIVOS: usize = 8192;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
const MAX_RUTA: usize = 512;

/// Código y metadatos cargados en memoria.
#[derive(Debug)]
pub struct Bundle {
    entradas: BTreeMap<String, Arc<str>>,
    hash: String,
    pub nombre: String,
    pub inicio: String,
}

/// Resultado de `vaca construir`.
pub struct Artefacto {
    pub ruta: PathBuf,
    pub bytes: usize,
    pub hash: String,
    pub inicio: String,
    pub modulo: String,
}

impl Bundle {
    pub fn texto(&self, ruta: &str) -> Option<&str> {
        self.entradas.get(ruta).map(|texto| texto.as_ref())
    }

    pub fn contiene(&self, ruta: &str) -> bool {
        self.entradas.contains_key(ruta)
    }

    pub fn hash(&self) -> &str {
        &self.hash
    }

    pub fn archivos(&self) -> usize {
        self.entradas.len()
    }

    /// Manifiesto empaquetado en la raíz (`vaca.mod`).
    pub fn manifiesto(&self) -> Option<Manifiesto> {
        self.texto(paquetes::MANIFIESTO)
            .and_then(|texto| Manifiesto::desde_texto(texto).ok())
    }

    /// Resuelve una importación relativa o de tipo archivo dentro del bundle.
    pub fn resolver_archivo(&self, base: &str, modulo: &str) -> Option<String> {
        let clave = normalizar(base, modulo)?;
        self.buscar_archivo_bajo(&clave)
    }

    /// Resuelve una ruta de módulo (propia o de un paquete vendorizado).
    pub fn resolver_paquete(&self, modulo: &str) -> Option<String> {
        if let Some(manifiesto) = self.manifiesto() {
            if let Some(resto) = modulo.strip_prefix(&format!("{}/", manifiesto.nombre)) {
                if let Some(clave) = self.buscar_archivo_bajo(resto) {
                    return Some(clave);
                }
            }
        }
        self.buscar_paquete_bajo(&format!("paquetes/{}", modulo))
    }

    fn buscar_archivo_bajo(&self, base: &str) -> Option<String> {
        let mut candidatos = Vec::with_capacity(5);
        if base.ends_with(".vaca") {
            candidatos.push(base.to_string());
        } else {
            candidatos.push(base.to_string());
            candidatos.push(format!("{}.vaca", base));
            for entrada in ["paquete.vaca", "index.vaca", "modulo.vaca"] {
                candidatos.push(format!("{}/{}", base, entrada));
            }
        }
        candidatos
            .into_iter()
            .find(|candidato| self.entradas.contains_key(candidato))
    }

    fn buscar_paquete_bajo(&self, base: &str) -> Option<String> {
        for entrada in ["paquete.vaca", "index.vaca", "modulo.vaca"] {
            let candidato = format!("{}/{}", base, entrada);
            if self.entradas.contains_key(&candidato) {
                return Some(candidato);
            }
        }
        let candidato = format!("{}.vaca", base);
        if self.entradas.contains_key(&candidato) {
            Some(candidato)
        } else {
            None
        }
    }
}

/// Indica si un archivo empieza con la cabecera de bundle, sin leerlo entero.
pub fn es_bundle(ruta: &Path) -> bool {
    let Ok(mut archivo) = fs::File::open(ruta) else {
        return false;
    };
    let mut cabecera = [0u8; 8];
    archivo.read_exact(&mut cabecera).is_ok() && &cabecera == MAGIA
}

pub fn leer(ruta: &Path) -> Result<Bundle, Error> {
    let datos = fs::read(ruta)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", ruta.display(), e)))?;
    Bundle::desde_bytes(&datos).map_err(|error| match error {
        Error::Ejecucion(mensaje) => Error::ejecucion(format!("'{}': {}", ruta.display(), mensaje)),
        otro => otro,
    })
}

impl Bundle {
    pub fn desde_bytes(datos: &[u8]) -> Result<Bundle, Error> {
        if datos.len() < MAGIA.len() + 4 + 32 {
            return Err(Error::ejecucion("el bundle está truncado".to_string()));
        }
        if &datos[..MAGIA.len()] != MAGIA {
            return Err(Error::ejecucion(
                "no es un bundle de Vaca (falta la cabecera VACABND1)".to_string(),
            ));
        }
        let (cuerpo, hash_guardado) = datos.split_at(datos.len() - 32);
        let hash = crate::paquetes::sha256_hex(cuerpo);
        if !iguales(hash_guardado, &crate::paquetes::sha256_bytes(cuerpo)) {
            return Err(Error::ejecucion(
                "el bundle está corrupto: el hash no coincide".to_string(),
            ));
        }

        let mut cursor = MAGIA.len();
        let cantidad = leer_u32(cuerpo, &mut cursor)? as usize;
        if cantidad > MAX_ARCHIVOS {
            return Err(Error::ejecucion(format!(
                "el bundle declara demasiados archivos ({})",
                cantidad
            )));
        }
        let mut entradas = BTreeMap::new();
        let mut total = 0u64;
        for _ in 0..cantidad {
            let largo_ruta = leer_u16(cuerpo, &mut cursor)? as usize;
            if largo_ruta == 0 || largo_ruta > MAX_RUTA {
                return Err(Error::ejecucion(
                    "el bundle contiene una ruta inválida".to_string(),
                ));
            }
            let ruta = leer_bytes(cuerpo, &mut cursor, largo_ruta)?;
            let ruta = std::str::from_utf8(ruta)
                .map_err(|_| Error::ejecucion("ruta no UTF-8 en el bundle".to_string()))?;
            validar_clave(ruta)?;

            let largo = leer_u64(cuerpo, &mut cursor)?;
            total = total.saturating_add(largo);
            if total > MAX_TOTAL {
                return Err(Error::ejecucion(
                    "el bundle excede el tamaño máximo permitido".to_string(),
                ));
            }
            let contenido = leer_bytes(cuerpo, &mut cursor, largo as usize)?;
            let contenido = std::str::from_utf8(contenido)
                .map_err(|_| Error::ejecucion(format!("'{}' no es UTF-8 en el bundle", ruta)))?;
            entradas.insert(ruta.to_string(), Arc::from(contenido));
        }
        if cursor != cuerpo.len() {
            return Err(Error::ejecucion(
                "el bundle tiene datos sobrantes al final".to_string(),
            ));
        }

        let mut bundle = Bundle {
            entradas,
            hash: format!("sha256:{}", hash),
            nombre: String::new(),
            inicio: paquetes::ENTRADA_POR_DEFECTO.to_string(),
        };
        if let Some(manifiesto) = bundle.manifiesto() {
            bundle.nombre = manifiesto.nombre.clone();
            bundle.inicio = manifiesto.entrada();
        }
        if !bundle.contiene(&bundle.inicio) {
            return Err(Error::ejecucion(format!(
                "el bundle no contiene el archivo de entrada '{}'",
                bundle.inicio
            )));
        }
        Ok(bundle)
    }

    pub fn serializar(&self) -> Vec<u8> {
        let mut datos = Vec::new();
        datos.extend_from_slice(MAGIA);
        datos.extend_from_slice(&(self.entradas.len() as u32).to_le_bytes());
        for (ruta, contenido) in &self.entradas {
            datos.extend_from_slice(&(ruta.len() as u16).to_le_bytes());
            datos.extend_from_slice(ruta.as_bytes());
            datos.extend_from_slice(&(contenido.len() as u64).to_le_bytes());
            datos.extend_from_slice(contenido.as_bytes());
        }
        let hash = crate::paquetes::sha256_bytes(&datos);
        datos.extend_from_slice(&hash);
        datos
    }
}

/// Construye el artefacto de producción en `.vaca/dist/<nombre>.vaca`.
pub fn construir(raiz: &Path) -> Result<Artefacto, Error> {
    let ruta_manifiesto = Manifiesto::ruta_en(raiz).ok_or_else(|| {
        Error::ejecucion(format!(
            "no hay '{}' en '{}'; ejecuta `vaca init <modulo>` primero",
            paquetes::MANIFIESTO,
            raiz.display()
        ))
    })?;
    let manifiesto = Manifiesto::leer(&ruta_manifiesto)?;
    let inicio = normalizar("", &manifiesto.entrada()).ok_or_else(|| {
        Error::ejecucion(format!(
            "la entrada '{}' del módulo no es una ruta válida",
            manifiesto.entrada()
        ))
    })?;
    if !raiz.join(&inicio).is_file() {
        return Err(Error::ejecucion(format!(
            "no existe el archivo de entrada '{}' declarado en '{}'",
            inicio,
            ruta_manifiesto.display()
        )));
    }

    let mut entradas: BTreeMap<String, Arc<str>> = BTreeMap::new();
    recolectar_codigo(raiz, raiz, &mut entradas)?;

    let vendor = raiz.join(paquetes::DIR_VENDOR);
    if vendor.is_dir() {
        let mut archivos = Vec::new();
        listar_archivos(&vendor, &mut archivos)?;
        for archivo in archivos {
            let relativo = archivo.strip_prefix(&vendor).unwrap_or(&archivo);
            let relativo = relativo.to_string_lossy().replace('\\', "/");
            if !es_codigo(&relativo) {
                continue;
            }
            insertar(&mut entradas, &format!("paquetes/{}", relativo), &archivo)?;
        }
    }

    let contenido_manifiesto = fs::read_to_string(&ruta_manifiesto).map_err(|e| {
        Error::ejecucion(format!(
            "no se pudo leer '{}': {}",
            ruta_manifiesto.display(),
            e
        ))
    })?;
    entradas.insert(
        paquetes::MANIFIESTO.to_string(),
        Arc::from(contenido_manifiesto.as_str()),
    );
    let lock = raiz.join(paquetes::BLOQUEO);
    if lock.is_file() {
        let contenido = fs::read_to_string(&lock).map_err(|e| {
            Error::ejecucion(format!("no se pudo leer '{}': {}", lock.display(), e))
        })?;
        entradas.insert(paquetes::BLOQUEO.to_string(), Arc::from(contenido.as_str()));
    }

    for modulo in manifiesto.dependencias.keys() {
        let prefijo = format!("paquetes/{}", modulo);
        let instalado = entradas
            .keys()
            .any(|clave| clave.starts_with(&format!("{}/", prefijo)));
        if !instalado {
            return Err(Error::ejecucion(format!(
                "falta la dependencia '{}'; ejecuta `vaca instalar` antes de construir",
                modulo
            )));
        }
    }

    if !entradas.contains_key(&inicio) {
        return Err(Error::ejecucion(format!(
            "no se pudo empaquetar la entrada '{}'",
            inicio
        )));
    }

    let bundle = Bundle {
        entradas,
        hash: String::new(),
        nombre: manifiesto.nombre.clone(),
        inicio: inicio.clone(),
    };
    let datos = bundle.serializar();
    let hash = format!("sha256:{}", crate::paquetes::sha256_hex(&datos));

    let dist = raiz.join(".vaca/dist");
    fs::create_dir_all(&dist)
        .map_err(|e| Error::ejecucion(format!("no se pudo crear '{}': {}", dist.display(), e)))?;
    let nombre = paquetes::nombre_corto(&manifiesto.nombre);
    let ruta = dist.join(format!("{}.vaca", nombre));
    fs::write(&ruta, &datos).map_err(|e| {
        Error::ejecucion(format!("no se pudo escribir '{}': {}", ruta.display(), e))
    })?;
    fs::write(
        dist.join(format!("{}.vaca.sha256", nombre)),
        format!("{}  {}.vaca\n", hash.trim_start_matches("sha256:"), nombre),
    )
    .map_err(|e| Error::ejecucion(format!("no se pudo escribir el hash: {}", e)))?;

    Ok(Artefacto {
        ruta,
        bytes: datos.len(),
        hash,
        inicio,
        modulo: manifiesto.nombre,
    })
}

fn recolectar_codigo(
    raiz: &Path,
    dir: &Path,
    salida: &mut BTreeMap<String, Arc<str>>,
) -> Result<(), Error> {
    for entrada in fs::read_dir(dir)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", dir.display(), e)))?
    {
        let entrada =
            entrada.map_err(|e| Error::ejecucion(format!("directorio inválido: {}", e)))?;
        let nombre = entrada.file_name();
        let nombre = nombre.to_string_lossy();
        if matches!(
            nombre.as_ref(),
            ".git" | ".vaca" | "dist" | "node_modules" | "target"
        ) {
            continue;
        }
        let ruta = entrada.path();
        if ruta.is_dir() {
            recolectar_codigo(raiz, &ruta, salida)?;
        } else if ruta.is_file() && es_codigo(&nombre) {
            let relativo = ruta.strip_prefix(raiz).unwrap_or(&ruta);
            let relativo = relativo.to_string_lossy().replace('\\', "/");
            insertar(salida, &relativo, &ruta)?;
        }
    }
    Ok(())
}

fn insertar(
    salida: &mut BTreeMap<String, Arc<str>>,
    clave: &str,
    ruta: &Path,
) -> Result<(), Error> {
    let contenido = fs::read_to_string(ruta).map_err(|e| {
        Error::ejecucion(format!(
            "'{}' no es un archivo de texto UTF-8: {}",
            ruta.display(),
            e
        ))
    })?;
    salida.insert(clave.to_string(), Arc::from(contenido.as_str()));
    Ok(())
}

fn es_codigo(nombre: &str) -> bool {
    let nombre = nombre.rsplit('/').next().unwrap_or(nombre);
    nombre.ends_with(".vaca")
        || nombre == paquetes::MANIFIESTO
        || nombre == paquetes::MANIFIESTO_LEGADO
}

fn listar_archivos(dir: &Path, salida: &mut Vec<PathBuf>) -> Result<(), Error> {
    for entrada in fs::read_dir(dir)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer '{}': {}", dir.display(), e)))?
    {
        let entrada =
            entrada.map_err(|e| Error::ejecucion(format!("directorio inválido: {}", e)))?;
        let ruta = entrada.path();
        if ruta.is_dir() {
            listar_archivos(&ruta, salida)?;
        } else if ruta.is_file() {
            salida.push(ruta);
        }
    }
    Ok(())
}

/// Une `base` y `relativa` y elimina `.` y `..`, sin permitir escapar de la raíz.
pub fn normalizar(base: &str, relativa: &str) -> Option<String> {
    if relativa.starts_with('/') || relativa.starts_with('\\') {
        return None;
    }
    let unida = Path::new(base).join(relativa);
    let mut partes: Vec<String> = Vec::new();
    for componente in unida.components() {
        match componente {
            Component::CurDir => {}
            Component::Normal(parte) => partes.push(parte.to_string_lossy().to_string()),
            Component::ParentDir => {
                partes.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if partes.is_empty() {
        return None;
    }
    Some(partes.join("/"))
}

fn validar_clave(ruta: &str) -> Result<(), Error> {
    if ruta.is_empty() || ruta.starts_with('/') || ruta.contains('\\') {
        return Err(Error::ejecucion(format!(
            "ruta inválida en el bundle: '{}'",
            ruta
        )));
    }
    for segmento in ruta.split('/') {
        if segmento.is_empty() || segmento == "." || segmento == ".." {
            return Err(Error::ejecucion(format!(
                "ruta inválida en el bundle: '{}'",
                ruta
            )));
        }
    }
    Ok(())
}

fn iguales(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diferencia = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diferencia |= x ^ y;
    }
    diferencia == 0
}

fn leer_u16(datos: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    let bytes = leer_bytes(datos, cursor, 2)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn leer_u32(datos: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    let bytes = leer_bytes(datos, cursor, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn leer_u64(datos: &[u8], cursor: &mut usize) -> Result<u64, Error> {
    let bytes = leer_bytes(datos, cursor, 8)?;
    let mut arreglo = [0u8; 8];
    arreglo.copy_from_slice(bytes);
    Ok(u64::from_le_bytes(arreglo))
}

fn leer_bytes<'a>(datos: &'a [u8], cursor: &mut usize, largo: usize) -> Result<&'a [u8], Error> {
    let fin = cursor
        .checked_add(largo)
        .ok_or_else(|| Error::ejecucion("bundle truncado".to_string()))?;
    if fin > datos.len() {
        return Err(Error::ejecucion("bundle truncado".to_string()));
    }
    let bytes = &datos[*cursor..fin];
    *cursor = fin;
    Ok(bytes)
}
