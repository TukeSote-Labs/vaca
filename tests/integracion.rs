use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use vaca::ejecutar_fuente;

fn directorio_temporal(nombre: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut ruta = std::env::temp_dir();
    ruta.push(format!(
        "vaca_prueba_{}_{}_{}",
        nombre,
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&ruta).unwrap();
    ruta
}

#[test]
fn csv_lectura_filtrado_y_guardado() {
    let dir = directorio_temporal("csv");
    let entrada = dir.join("ventas.csv");
    fs::write(&entrada, "ciudad,monto\nMadrid,100\nLima,50\nMadrid,30\n").unwrap();
    let salida_csv = dir.join("mayores.csv");

    let codigo = format!(
        r#"
        importar datos
        df = datos.leer_csv("{entrada}")
        imprimir(datos.dimensiones(df))
        mayores = datos.filtrar(df, "monto", ">=", 60)
        imprimir(datos.dimensiones(mayores))
        datos.guardar_csv(mayores, "{salida}")
        resumen = datos.agrupar_resumir(df, "ciudad", "monto", "suma")
        imprimir(datos.columna(resumen, "monto"))
        "#,
        entrada = entrada.display(),
        salida = salida_csv.display()
    );
    let salida = ejecutar_fuente(&codigo).unwrap();
    assert!(salida.contains("[3, 2]"));
    assert!(salida.contains("[1, 2]"));
    assert!(salida.starts_with("[3, 2]\n[1, 2]\n"));

    let guardado = fs::read_to_string(&salida_csv).unwrap();
    assert!(guardado.contains("Madrid,100"));
    assert!(!guardado.contains("Lima,50"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn importacion_de_archivo_vaca() {
    let dir = directorio_temporal("modulo");
    let modulo = dir.join("util.vaca");
    fs::write(
        &modulo,
        r#"
        constante VERSION = "9.9"
        fn saludar(nombre) { retornar a"Hola, {nombre}" }
        fn doblar(n) { retornar n * 2 }
        "#,
    )
    .unwrap();

    let codigo = format!(
        r#"
        importar "{modulo}" como util
        imprimir(util.VERSION)
        imprimir(util.saludar("Vaca"))
        imprimir(util.doblar(21))
        "#,
        modulo = modulo.display()
    );
    let salida = ejecutar_fuente(&codigo).unwrap();
    assert_eq!(salida, "9.9\nHola, Vaca\n42\n");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cliente_http_contra_servidor_de_prueba() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = listener.local_addr().unwrap().port();

    let servidor = std::thread::spawn(move || {
        let (mut flujo, _) = listener.accept().unwrap();
        let mut buffer = [0u8; 4096];
        let leidos = flujo.read(&mut buffer).unwrap();
        let peticion = String::from_utf8_lossy(&buffer[..leidos]).to_string();
        assert!(peticion.starts_with("GET /datos?x=1 HTTP/1.1"));
        assert!(peticion.contains("Host: 127.0.0.1"));
        let cuerpo = "{\"ok\":true}";
        let respuesta = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            cuerpo.len(),
            cuerpo
        );
        flujo.write_all(respuesta.as_bytes()).unwrap();
    });

    let codigo = format!(
        r#"
        importar http
        respuesta = http.peticion("http://127.0.0.1:{puerto}/datos?x=1")
        imprimir(respuesta.estado)
        imprimir(respuesta.cuerpo)
        "#,
        puerto = puerto
    );
    let salida = ejecutar_fuente(&codigo).unwrap();
    servidor.join().unwrap();
    assert_eq!(salida, "200\n{\"ok\":true}\n");
}

#[test]
fn json_rechaza_anidamiento_excesivo() {
    let texto = format!("{}0{}", "[".repeat(300), "]".repeat(300));
    assert!(vaca::modulos::json::parse(&texto).is_err());
}

#[test]
fn json_roundtrip_complejo() {
    let salida = ejecutar_fuente(
        r#"
        importar json
        original = {"a": [1, 2, 3], "b": {"c": verdadero}, "d": "texto con \"comillas\""}
        serializado = json.stringify(original)
        copia = json.parse(serializado)
        imprimir(copia.a[2])
        imprimir(copia.b.c)
        imprimir(copia.d)
        imprimir(json.valido(serializado))
        imprimir(json.valido("{roto"))
        "#,
    )
    .unwrap();
    assert_eq!(
        salida,
        "3\nverdadero\ntexto con \"comillas\"\nverdadero\nfalso\n"
    );
}

#[test]
fn sistema_cargar_env_lee_comentarios_y_comillas() {
    let dir = directorio_temporal("env");
    let ruta = dir.join(".env");
    fs::write(
        &ruta,
        r#"
# Configuración de prueba
PUERTO=8080
NOMBRE="Vaca servidor" # comentario al final
export DEBUG=true
URL=http://localhost:8080/api?x=1
CLAVE='valor # literal'
"#,
    )
    .unwrap();

    let ruta_texto = ruta.to_string_lossy().replace('\\', "\\\\");
    let codigo = format!(
        r#"
        importar sistema

        config = sistema.cargar_env("{ruta}")
        imprimir(config.get("PUERTO"))
        imprimir(config.get("NOMBRE"))
        imprimir(config.get("DEBUG"))
        imprimir(config.get("URL"))
        imprimir(config.get("CLAVE"))

        vacio = sistema.cargar_env("{ruta}.no-existe")
        imprimir(vacio.longitud())

        intentar {{
            sistema.cargar_env("{ruta}.no-existe", verdadero)
        }} atrapar {{
            imprimir("error esperado")
        }}
        "#,
        ruta = ruta_texto
    );

    let salida = ejecutar_fuente(&codigo).unwrap();
    assert_eq!(
        salida,
        "8080\nVaca servidor\ntrue\nhttp://localhost:8080/api?x=1\nvalor # literal\n0\nerror esperado\n"
    );

    let _ = fs::remove_dir_all(&dir);
}
