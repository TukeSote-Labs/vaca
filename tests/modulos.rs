//! Pruebas del modelo de módulos (`vaca.mod`), la entrada `iniciar.vaca` y el
//! empaquetado de producción en un único archivo.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use vaca::bundle;
use vaca::paquetes::{self, Manifiesto};

struct Directorio {
    raiz: PathBuf,
}

impl Drop for Directorio {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.raiz);
    }
}

fn directorio(nombre: &str) -> Directorio {
    let raiz = std::env::temp_dir().join(format!("vaca-modulos-{}-{}", nombre, std::process::id()));
    let _ = fs::remove_dir_all(&raiz);
    fs::create_dir_all(&raiz).expect("crear directorio temporal");
    Directorio { raiz }
}

fn escribir(raiz: &Path, ruta: &str, contenido: &str) {
    let destino = raiz.join(ruta);
    if let Some(padre) = destino.parent() {
        fs::create_dir_all(padre).expect("crear directorio");
    }
    fs::write(destino, contenido).expect("escribir archivo");
}

fn vaca(raiz: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vaca"))
        .args(args)
        .current_dir(raiz)
        .output()
        .expect("ejecutar vaca")
}

fn salida_texto(salida: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&salida.stdout),
        String::from_utf8_lossy(&salida.stderr)
    )
}

#[test]
fn manifiesto_de_texto_ida_y_vuelta() {
    let proyecto = directorio("manifiesto");
    let original = Manifiesto {
        nombre: "ejemplo.com/app".to_string(),
        version: "1.2.3".to_string(),
        inicio: "servidor.vaca".to_string(),
        dependencias: [("github.com/u/lib".to_string(), "^1.2.0".to_string())].into(),
        reemplazos: [("github.com/u/local".to_string(), "../local".to_string())].into(),
        origenes: [(
            "github.com/u/lib".to_string(),
            "https://git.ejemplo.com/lib.git".to_string(),
        )]
        .into(),
    };
    let ruta = proyecto.raiz.join(paquetes::MANIFIESTO);
    original.escribir(&ruta).expect("escribir manifiesto");

    let texto = fs::read_to_string(&ruta).expect("leer manifiesto");
    assert!(texto.contains("modulo ejemplo.com/app"), "texto: {}", texto);
    assert!(texto.contains("inicio servidor.vaca"), "texto: {}", texto);
    assert!(
        texto.contains("requiere github.com/u/lib ^1.2.0"),
        "texto: {}",
        texto
    );

    let leido = Manifiesto::leer(&ruta).expect("leer");
    assert_eq!(leido.nombre, original.nombre);
    assert_eq!(leido.inicio, original.inicio);
    assert_eq!(leido.dependencias, original.dependencias);
    assert_eq!(leido.reemplazos, original.reemplazos);
    assert_eq!(leido.origenes, original.origenes);
}

#[test]
fn sigue_leyendo_el_manifiesto_json_antiguo() {
    let proyecto = directorio("legado");
    let ruta = proyecto.raiz.join(paquetes::MANIFIESTO_LEGADO);
    fs::write(
        &ruta,
        "{\n  \"nombre\": \"ejemplo.com/viejo\",\n  \"version\": \"0.4.0\",\n  \"dependencias\": {\"ejemplo.com/lib\": \"^1.0.0\"}\n}\n",
    )
    .expect("escribir JSON");

    assert_eq!(
        Manifiesto::ruta_en(&proyecto.raiz),
        Some(ruta.clone()),
        "debe encontrar el manifiesto antiguo"
    );
    let manifiesto = Manifiesto::leer(&ruta).expect("leer JSON");
    assert_eq!(manifiesto.nombre, "ejemplo.com/viejo");
    assert_eq!(manifiesto.entrada(), paquetes::ENTRADA_POR_DEFECTO);
    assert_eq!(manifiesto.dependencias["ejemplo.com/lib"], "^1.0.0");
}

#[test]
fn init_crea_manifiesto_y_punto_de_entrada() {
    let proyecto = directorio("init");
    let creado = paquetes::init(&proyecto.raiz, "ejemplo.com/app").expect("init");
    assert_eq!(creado.nombre, "ejemplo.com/app");

    let manifiesto = fs::read_to_string(&creado.manifiesto).expect("manifiesto");
    assert!(manifiesto.contains("modulo ejemplo.com/app"));
    assert!(manifiesto.contains("inicio iniciar.vaca"));

    let entrada = fs::read_to_string(&creado.entrada).expect("entrada");
    assert!(entrada.contains("fn principal()"), "entrada: {}", entrada);

    // No se sobrescribe un módulo existente.
    assert!(paquetes::init(&proyecto.raiz, "ejemplo.com/otro").is_err());
}

#[test]
fn init_exige_el_nombre_del_modulo() {
    let proyecto = directorio("init-sin-nombre");
    let salida = vaca(&proyecto.raiz, &["init"]);
    assert!(!salida.status.success());
    let texto = salida_texto(&salida);
    assert!(texto.contains("falta el nombre"), "salida: {}", texto);
    assert!(!proyecto.raiz.join(paquetes::MANIFIESTO).exists());
}

#[test]
fn valida_nombres_de_modulo() {
    assert!(paquetes::validar_nombre_modulo("ejemplo.com/app").is_ok());
    assert!(paquetes::validar_nombre_modulo("app").is_ok());
    assert!(paquetes::validar_nombre_modulo("").is_err());
    assert!(paquetes::validar_nombre_modulo("../fuera").is_err());
    assert!(paquetes::validar_nombre_modulo("app con espacio").is_err());
}

#[test]
fn importa_un_subpaquete_del_propio_modulo() {
    let proyecto = directorio("propio");
    escribir(
        &proyecto.raiz,
        "vaca.mod",
        "modulo ejemplo.com/app\nversion 0.1.0\ninicio iniciar.vaca\n",
    );
    escribir(
        &proyecto.raiz,
        "util.vaca",
        "fn saludo(nombre) {\n    retornar \"Hola, \" + nombre\n}\n",
    );
    escribir(
        &proyecto.raiz,
        "iniciar.vaca",
        "importar \"ejemplo.com/app/util\"\nimprimir(util.saludo(\"Vaca\"))\n",
    );

    let salida = vaca::ejecutar_archivo(proyecto.raiz.join("iniciar.vaca")).expect("ejecutar");
    assert_eq!(salida, "Hola, Vaca\n");
}

#[test]
fn vaca_sin_argumentos_inicia_el_modulo() {
    let proyecto = directorio("ejecutar");
    let creado = paquetes::init(&proyecto.raiz, "ejemplo.com/app").expect("init");

    let salida = vaca(&proyecto.raiz, &[]);
    assert!(
        salida.status.success(),
        "vaca falló: {}",
        salida_texto(&salida)
    );
    assert_eq!(
        String::from_utf8_lossy(&salida.stdout).trim(),
        "Hola desde ejemplo.com/app"
    );

    // Ejecutar la entrada de forma explícita también llama a principal().
    let salida = vaca(&proyecto.raiz, &["iniciar.vaca"]);
    assert!(salida.status.success(), "{}", salida_texto(&salida));
    assert_eq!(
        String::from_utf8_lossy(&salida.stdout).trim(),
        "Hola desde ejemplo.com/app"
    );

    // Desde un subdirectorio también se encuentra la raíz del módulo.
    fs::create_dir_all(proyecto.raiz.join("sub")).expect("sub");
    let salida = vaca(&proyecto.raiz.join("sub"), &[]);
    assert!(salida.status.success(), "{}", salida_texto(&salida));
    assert!(String::from_utf8_lossy(&salida.stdout).contains("Hola desde"));

    assert!(creado.entrada.is_file());
}

#[test]
fn construir_genera_un_bundle_verificado() {
    let proyecto = directorio("bundle");
    escribir(
        &proyecto.raiz,
        "vaca.mod",
        "modulo ejemplo.com/app\nversion 0.1.0\ninicio iniciar.vaca\n",
    );
    escribir(
        &proyecto.raiz,
        "iniciar.vaca",
        "fn principal() {\n    imprimir(\"bundle\")\n}\n",
    );

    let artefacto = bundle::construir(&proyecto.raiz).expect("construir");
    assert!(artefacto.ruta.is_file());
    assert_eq!(
        artefacto.ruta.extension().and_then(|e| e.to_str()),
        Some("vaca")
    );
    assert!(artefacto.hash.starts_with("sha256:"));

    let cargado = bundle::leer(&artefacto.ruta).expect("leer bundle");
    assert_eq!(cargado.inicio, "iniciar.vaca");
    assert_eq!(cargado.nombre, "ejemplo.com/app");
    assert!(cargado.hash().starts_with("sha256:"));
    assert!(cargado.texto("iniciar.vaca").is_some());
    assert!(cargado.texto("vaca.mod").is_some());

    // Cualquier byte alterado invalida el hash.
    let mut datos = fs::read(&artefacto.ruta).expect("leer bytes");
    let posicion = datos.len() / 2;
    datos[posicion] ^= 0x01;
    let roto = proyecto.raiz.join("roto.vaca");
    fs::write(&roto, &datos).expect("escribir roto");
    let error = bundle::leer(&roto).expect_err("debe fallar");
    assert!(
        error.to_string().contains("hash no coincide"),
        "error: {}",
        error
    );
}

#[test]
fn el_bundle_incluye_dependencias_y_corre_aislado() {
    let proyecto = directorio("bundle-deps");
    let aislado = directorio("bundle-aislado");
    escribir(
        &proyecto.raiz,
        "lib/paquete.vaca",
        "importar \"util.vaca\"\n\nfn saludo() {\n    retornar util.texto()\n}\n",
    );
    escribir(
        &proyecto.raiz,
        "lib/util.vaca",
        "fn texto() {\n    retornar \"desde la dependencia\"\n}\n",
    );
    escribir(
        &proyecto.raiz,
        "vaca.mod",
        "modulo ejemplo.com/app\nversion 0.1.0\ninicio iniciar.vaca\n\nrequiere ejemplo.com/lib *\nreemplaza ejemplo.com/lib lib\n",
    );
    escribir(
        &proyecto.raiz,
        "iniciar.vaca",
        "importar \"ejemplo.com/lib\"\n\nfn principal() {\n    imprimir(lib.saludo())\n}\n",
    );

    paquetes::instalar(&proyecto.raiz, false).expect("instalar");
    let artefacto = bundle::construir(&proyecto.raiz).expect("construir");

    let destino = aislado.raiz.join("app.vaca");
    fs::copy(&artefacto.ruta, &destino).expect("copiar bundle");
    fs::remove_dir_all(proyecto.raiz.join(".vaca")).expect("borrar vendor");

    let salida = vaca(&aislado.raiz, &["app.vaca"]);
    assert!(
        salida.status.success(),
        "el bundle no arrancó: {}",
        salida_texto(&salida)
    );
    assert_eq!(
        String::from_utf8_lossy(&salida.stdout).trim(),
        "desde la dependencia"
    );
}
