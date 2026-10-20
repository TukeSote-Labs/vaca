//! Calcula la versión que se muestra en la CLI.
//!
//! La versión base viene de `Cargo.toml` y se completa con metadatos del
//! último commit de Git: `0.1.0+<commits>.<hash-corto>`. Si Git no está
//! disponible (por ejemplo, al compilar desde un tarball sin `.git`), se usa
//! solo la versión de Cargo.

use std::process::Command;

fn salida(programa: &str, args: &[&str]) -> Option<String> {
    let salida = Command::new(programa).args(args).output().ok()?;
    if !salida.status.success() {
        return None;
    }
    let texto = String::from_utf8(salida.stdout).ok()?;
    let texto = texto.trim().to_string();
    if texto.is_empty() {
        None
    } else {
        Some(texto)
    }
}

fn main() {
    // Recompilar cuando cambie el commit actual o la rama.
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Some(git_dir) = salida("git", &["rev-parse", "--git-dir"]) {
        println!("cargo:rerun-if-changed={}/HEAD", git_dir);
        if let Some(rama) = salida("git", &["rev-parse", "--abbrev-ref", "HEAD"]) {
            println!("cargo:rerun-if-changed={}/refs/heads/{}", git_dir, rama);
        }
    }

    let base = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
    let cuenta = salida("git", &["rev-list", "--count", "HEAD"]);
    let commit = salida("git", &["rev-parse", "--short=7", "HEAD"]);

    let version = match (cuenta, commit) {
        (Some(cuenta), Some(commit)) => {
            println!("cargo:rustc-env=VACA_COMMIT_COUNT={}", cuenta);
            println!("cargo:rustc-env=VACA_COMMIT={}", commit);
            format!("{}+{}.{}", base, cuenta, commit)
        }
        _ => base,
    };

    println!("cargo:rustc-env=VACA_VERSION={}", version);
}
