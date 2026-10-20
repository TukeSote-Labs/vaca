use std::process::Command;

fn ejecutar(args: &[&str]) -> String {
    let salida = Command::new(env!("CARGO_BIN_EXE_vaca"))
        .args(args)
        .output()
        .expect("no se pudo ejecutar el binario vaca");

    assert!(
        salida.status.success(),
        "vaca falló: {}",
        String::from_utf8_lossy(&salida.stderr)
    );

    String::from_utf8_lossy(&salida.stdout).to_string()
}

#[test]
fn ayuda_muestra_cowsay_y_version() {
    let texto = ejecutar(&["--ayuda"]);

    assert!(texto.contains("Vaca: Lenguaje en español para servidores"));
    assert!(texto.contains("VERSION = "));
    assert!(texto.contains("^__^"));
    assert!(texto.contains("(oo)"));
    assert!(texto.contains("USO:"));
}

#[test]
fn version_incluye_los_metadatos_del_commit() {
    let texto = ejecutar(&["--version"]);

    assert!(texto.starts_with("Vaca "), "salida: {}", texto);
    assert!(
        texto.contains(env!("CARGO_PKG_VERSION")),
        "salida: {}",
        texto
    );

    if let Some(commit) = option_env!("VACA_COMMIT") {
        assert!(texto.contains(commit), "salida: {}", texto);
    }
}
