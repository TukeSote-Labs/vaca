use std::fs;
use std::path::Path;

use vaca::ejecutar_archivo;

#[test]
fn todos_los_ejemplos_se_ejecutan() {
    let directorio = Path::new(env!("CARGO_MANIFEST_DIR")).join("ejemplos");
    let mut ejemplos: Vec<_> = fs::read_dir(&directorio)
        .expect("directorio de ejemplos")
        .filter_map(|entrada| entrada.ok().map(|e| e.path()))
        .filter(|ruta| ruta.extension().and_then(|e| e.to_str()) == Some("vaca"))
        .filter(|ruta| {
            let nombre = ruta.file_name().and_then(|n| n.to_str()).unwrap_or("");
            nombre != "servidor.vaca" && nombre != "servidor_api.vaca"
        })
        .collect();
    ejemplos.sort();

    assert!(!ejemplos.is_empty(), "no se encontraron ejemplos");

    let mut fallos = Vec::new();
    for ruta in ejemplos {
        if let Err(error) = ejecutar_archivo(&ruta) {
            fallos.push(format!("{}: {}", ruta.display(), error));
        }
    }

    assert!(
        fallos.is_empty(),
        "ejemplos con errores:\n{}",
        fallos.join("\n")
    );
}
