//! Pruebas del sistema de paquetes con repositorios git **locales**.
//!
//! No se usa red: cada "remoto" es un repositorio git en un directorio temporal
//! y el manifiesto lo declara en `origenes` con una URL `file://`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use vaca::paquetes::{self, Manifiesto};

fn git_disponible() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|salida| salida.status.success())
        .unwrap_or(false)
}

fn git(dir: &Path, args: &[&str]) {
    let salida = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Pruebas Vaca")
        .env("GIT_AUTHOR_EMAIL", "pruebas@vaca.local")
        .env("GIT_COMMITTER_NAME", "Pruebas Vaca")
        .env("GIT_COMMITTER_EMAIL", "pruebas@vaca.local")
        .output()
        .expect("ejecutar git");
    assert!(
        salida.status.success(),
        "git {:?} falló: {}",
        args,
        String::from_utf8_lossy(&salida.stderr)
    );
}

fn crear_repo(dir: &Path, versiones: &[(&str, &[(&str, &str)])]) {
    fs::create_dir_all(dir).expect("crear repo");
    git(dir, &["init", "-q"]);
    for (tag, archivos) in versiones {
        for (nombre, contenido) in archivos.iter() {
            let ruta = dir.join(nombre);
            if let Some(padre) = ruta.parent() {
                fs::create_dir_all(padre).expect("crear directorio");
            }
            fs::write(ruta, contenido).expect("escribir archivo");
        }
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", tag]);
        git(dir, &["tag", tag]);
    }
}

struct Proyecto {
    raiz: PathBuf,
}

impl Drop for Proyecto {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.raiz);
    }
}

fn proyecto(nombre: &str) -> Proyecto {
    let raiz =
        std::env::temp_dir().join(format!("vaca-paquetes-{}-{}", nombre, std::process::id()));
    let _ = fs::remove_dir_all(&raiz);
    fs::create_dir_all(&raiz).expect("crear proyecto");
    Proyecto { raiz }
}

fn manifiesto(raiz: &Path, dependencias: &[(&str, &str)], origenes: &[(&str, String)]) {
    let mut manifiesto = Manifiesto {
        nombre: "ejemplo.com/app".to_string(),
        version: "0.1.0".to_string(),
        ..Manifiesto::default()
    };
    for (modulo, restriccion) in dependencias {
        manifiesto
            .dependencias
            .insert(modulo.to_string(), restriccion.to_string());
    }
    for (modulo, url) in origenes {
        manifiesto.origenes.insert(modulo.to_string(), url.clone());
    }
    manifiesto
        .escribir(&raiz.join(paquetes::MANIFIESTO))
        .expect("escribir manifiesto");
}

fn url_local(ruta: &Path) -> String {
    format!("file://{}", ruta.display())
}

#[test]
fn instala_la_mejor_version_y_escribe_el_lock() {
    if !git_disponible() {
        eprintln!("git no está disponible; se omite la prueba");
        return;
    }
    let proyecto = proyecto("version");
    let repo = proyecto.raiz.join("repo-lib");
    crear_repo(
        &repo,
        &[
            (
                "v1.0.0",
                &[("paquete.vaca", "fn version() { retornar \"1.0.0\" }\n")],
            ),
            (
                "v1.1.0",
                &[("paquete.vaca", "fn version() { retornar \"1.1.0\" }\n")],
            ),
            (
                "v2.0.0",
                &[("paquete.vaca", "fn version() { retornar \"2.0.0\" }\n")],
            ),
        ],
    );
    manifiesto(
        &proyecto.raiz,
        &[("ejemplo.com/lib", "^1.0.0")],
        &[("ejemplo.com/lib", url_local(&repo))],
    );

    let resultado = paquetes::instalar(&proyecto.raiz, false).expect("instalar");
    assert_eq!(resultado.instalados.len(), 1);

    let instalado = proyecto
        .raiz
        .join(".vaca/paquetes/ejemplo.com/lib/paquete.vaca");
    let contenido = fs::read_to_string(&instalado).expect("paquete instalado");
    assert!(contenido.contains("1.1.0"), "contenido: {}", contenido);

    let bloqueo = paquetes::Bloqueo::leer(&proyecto.raiz.join(paquetes::BLOQUEO)).expect("lock");
    let datos = bloqueo
        .paquetes
        .get("ejemplo.com/lib")
        .expect("entrada en el lock");
    assert_eq!(datos.version, "1.1.0");
    assert!(!datos.commit.is_empty());
    assert!(datos.hash.starts_with("sha256:"));

    let listado = paquetes::listar(&proyecto.raiz).expect("listar");
    assert!(listado.contains("1.1.0"), "listado: {}", listado);
}

#[test]
fn importa_un_paquete_instalado() {
    if !git_disponible() {
        eprintln!("git no está disponible; se omite la prueba");
        return;
    }
    let proyecto = proyecto("importar");
    let repo = proyecto.raiz.join("repo-lib");
    crear_repo(
        &repo,
        &[(
            "v1.0.0",
            &[(
                "paquete.vaca",
                "fn saludo() { retornar \"hola desde el paquete\" }\n",
            )],
        )],
    );
    manifiesto(
        &proyecto.raiz,
        &[("ejemplo.com/lib", "^1.0.0")],
        &[("ejemplo.com/lib", url_local(&repo))],
    );
    paquetes::instalar(&proyecto.raiz, false).expect("instalar");

    let principal = proyecto.raiz.join("main.vaca");
    fs::write(
        &principal,
        "importar \"ejemplo.com/lib\"\nimprimir(lib.saludo())\n",
    )
    .expect("escribir main");

    let salida = vaca::ejecutar_archivo(&principal).expect("ejecutar");
    assert_eq!(salida, "hola desde el paquete\n");
}

#[test]
fn resuelve_dependencias_transitivas() {
    if !git_disponible() {
        eprintln!("git no está disponible; se omite la prueba");
        return;
    }
    let proyecto = proyecto("transitivas");
    let base = proyecto.raiz.join("repo-base");
    crear_repo(
        &base,
        &[(
            "v1.0.0",
            &[("paquete.vaca", "fn texto() { retornar \"base\" }\n")],
        )],
    );

    let lib = proyecto.raiz.join("repo-lib");
    crear_repo(
        &lib,
        &[(
            "v1.0.0",
            &[
                (
                    "vaca.paquete",
                    "{\n  \"nombre\": \"ejemplo.com/lib\",\n  \"version\": \"1.0.0\",\n  \"dependencias\": {\"ejemplo.com/base\": \"^1.0.0\"}\n}\n",
                ),
                (
                    "paquete.vaca",
                    "importar \"ejemplo.com/base\"\nfn saludo() { retornar base.texto() }\n",
                ),
            ],
        )],
    );

    manifiesto(
        &proyecto.raiz,
        &[("ejemplo.com/lib", "^1.0.0")],
        &[
            ("ejemplo.com/lib", url_local(&lib)),
            ("ejemplo.com/base", url_local(&base)),
        ],
    );
    paquetes::instalar(&proyecto.raiz, false).expect("instalar");

    assert!(proyecto
        .raiz
        .join(".vaca/paquetes/ejemplo.com/base/paquete.vaca")
        .is_file());

    let principal = proyecto.raiz.join("main.vaca");
    fs::write(
        &principal,
        "importar \"ejemplo.com/lib\"\nimprimir(lib.saludo())\n",
    )
    .expect("escribir main");
    let salida = vaca::ejecutar_archivo(&principal).expect("ejecutar");
    assert_eq!(salida, "base\n");
}

#[test]
fn detecta_restricciones_incompatibles() {
    if !git_disponible() {
        eprintln!("git no está disponible; se omite la prueba");
        return;
    }
    let proyecto = proyecto("conflicto");
    let base = proyecto.raiz.join("repo-base");
    crear_repo(
        &base,
        &[
            ("v1.0.0", &[("paquete.vaca", "fn x() { retornar 1 }\n")]),
            ("v2.0.0", &[("paquete.vaca", "fn x() { retornar 2 }\n")]),
        ],
    );
    let lib = proyecto.raiz.join("repo-lib");
    crear_repo(
        &lib,
        &[(
            "v1.0.0",
            &[
                (
                    "vaca.paquete",
                    "{\n  \"dependencias\": {\"ejemplo.com/base\": \"^1.0.0\"}\n}\n",
                ),
                ("paquete.vaca", "fn x() { retornar 0 }\n"),
            ],
        )],
    );

    manifiesto(
        &proyecto.raiz,
        &[
            ("ejemplo.com/base", "^2.0.0"),
            ("ejemplo.com/lib", "^1.0.0"),
        ],
        &[
            ("ejemplo.com/base", url_local(&base)),
            ("ejemplo.com/lib", url_local(&lib)),
        ],
    );

    let error = paquetes::instalar(&proyecto.raiz, false).expect_err("debe fallar");
    let mensaje = error.to_string();
    assert!(
        mensaje.contains("no hay una versión"),
        "mensaje inesperado: {}",
        mensaje
    );
}

#[test]
fn copia_reemplazos_locales() {
    let proyecto = proyecto("reemplazo");
    let local = proyecto.raiz.join("lib-local");
    fs::create_dir_all(local.join(".git")).expect("dir");
    fs::write(local.join("paquete.vaca"), "fn dato() { retornar 42 }\n").expect("escribir");
    fs::write(local.join(".git/ignorar"), "no copiar").expect("escribir");

    let mut manifiesto = Manifiesto {
        nombre: "ejemplo.com/app".to_string(),
        version: "0.1.0".to_string(),
        ..Manifiesto::default()
    };
    manifiesto
        .dependencias
        .insert("ejemplo.com/local".to_string(), "*".to_string());
    manifiesto
        .reemplazos
        .insert("ejemplo.com/local".to_string(), "lib-local".to_string());
    manifiesto
        .escribir(&proyecto.raiz.join(paquetes::MANIFIESTO))
        .expect("manifiesto");

    let resultado = paquetes::instalar(&proyecto.raiz, false).expect("instalar");
    assert_eq!(
        resultado.reemplazados,
        vec!["ejemplo.com/local".to_string()]
    );

    let copiado = proyecto.raiz.join(".vaca/paquetes/ejemplo.com/local");
    assert!(copiado.join("paquete.vaca").is_file());
    assert!(!copiado.join(".git").exists());

    let principal = proyecto.raiz.join("main.vaca");
    fs::write(
        &principal,
        "importar \"ejemplo.com/local\"\nimprimir(local.dato())\n",
    )
    .expect("main");
    let salida = vaca::ejecutar_archivo(&principal).expect("ejecutar");
    assert_eq!(salida, "42\n");
}

#[test]
fn reinstala_sin_red_usando_el_lock() {
    if !git_disponible() {
        eprintln!("git no está disponible; se omite la prueba");
        return;
    }
    let proyecto = proyecto("offline");
    let repo = proyecto.raiz.join("repo-lib");
    crear_repo(
        &repo,
        &[("v1.0.0", &[("paquete.vaca", "fn x() { retornar 1 }\n")])],
    );
    manifiesto(
        &proyecto.raiz,
        &[("ejemplo.com/lib", "^1.0.0")],
        &[("ejemplo.com/lib", url_local(&repo))],
    );
    paquetes::instalar(&proyecto.raiz, false).expect("primera instalación");

    // Sin el repositorio original, la segunda instalación debe usar el vendor
    // y el hash del lock, sin tocar la red ni git.
    fs::remove_dir_all(&repo).expect("borrar repo");
    paquetes::instalar(&proyecto.raiz, false).expect("reinstalación offline");
}

#[test]
fn actualizar_ignora_el_lock_y_sube_de_version() {
    if !git_disponible() {
        eprintln!("git no está disponible; se omite la prueba");
        return;
    }
    let proyecto = proyecto("actualizar");
    let repo = proyecto.raiz.join("repo-lib");
    crear_repo(
        &repo,
        &[(
            "v1.0.0",
            &[("paquete.vaca", "fn version() { retornar \"1.0.0\" }\n")],
        )],
    );
    manifiesto(
        &proyecto.raiz,
        &[("ejemplo.com/lib", "^1.0.0")],
        &[("ejemplo.com/lib", url_local(&repo))],
    );
    paquetes::instalar(&proyecto.raiz, false).expect("instalar 1.0.0");

    // Nueva versión publicada después de la primera instalación.
    git(&repo, &["tag", "v1.1.0"]);

    paquetes::instalar(&proyecto.raiz, false).expect("reinstalar con lock");
    let bloqueo =
        paquetes::Bloqueo::leer(&proyecto.raiz.join(paquetes::BLOQUEO)).expect("lock");
    assert_eq!(bloqueo.paquetes["ejemplo.com/lib"].version, "1.0.0");

    paquetes::instalar(&proyecto.raiz, true).expect("actualizar");
    let bloqueo =
        paquetes::Bloqueo::leer(&proyecto.raiz.join(paquetes::BLOQUEO)).expect("lock");
    assert_eq!(bloqueo.paquetes["ejemplo.com/lib"].version, "1.1.0");
}
