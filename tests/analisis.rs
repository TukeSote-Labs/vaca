use vaca::{ejecutar_fuente, Interprete};

fn mensaje_error(codigo: &str) -> String {
    ejecutar_fuente(codigo)
        .expect_err("el programa debería fallar en el análisis")
        .to_string()
}

#[test]
fn variable_local_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        fn calcular() {
            resultado = 42
            retornar 0
        }
        imprimir(calcular())
        "#,
    );
    assert!(mensaje.contains("error de compilación"));
    assert!(mensaje.contains("resultado"));
    assert!(mensaje.contains("nunca usadas"));
    assert!(mensaje.contains("función 'calcular'"));
}

#[test]
fn variable_local_usada_compila() {
    let salida = ejecutar_fuente(
        r#"
        fn calcular() {
            resultado = 42
            retornar resultado
        }
        imprimir(calcular())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "42\n");
}

#[test]
fn globales_y_parametros_estan_exentos() {
    let salida = ejecutar_fuente(
        r#"
        global_sin_usar = 10

        fn suma(a, b) {
            retornar a
        }

        imprimir(suma(1, 2))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "1\n");
}

#[test]
fn parametros_se_pueden_usar_y_los_globales_tambien() {
    let salida = ejecutar_fuente(
        r#"
        base = 10
        fn sumar(a, b) {
            retornar a + b + base
        }
        imprimir(sumar(1, 2))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "13\n");
}

#[test]
fn variable_de_bucle_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        para i = 1 hasta 3 {
            imprimir("hola")
        }
        "#,
    );
    assert!(mensaje.contains("'i'"));
    assert!(mensaje.contains("bucle 'para i'"));
}

#[test]
fn variable_de_bucle_usada_compila() {
    let salida = ejecutar_fuente(
        r#"
        total = 0
        para i = 1 hasta 3 {
            total = total + i
        }
        imprimir(total)
        "#,
    )
    .unwrap();
    assert_eq!(salida, "6\n");
}

#[test]
fn variable_de_para_cada_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        para elemento en [1, 2, 3] {
            imprimir("x")
        }
        "#,
    );
    assert!(mensaje.contains("elemento"));
}

#[test]
fn variable_de_para_cada_usada_compila() {
    let salida = ejecutar_fuente(
        r#"
        total = 0
        para elemento en [1, 2, 3] {
            total = total + elemento
        }
        imprimir(total)
        "#,
    )
    .unwrap();
    assert_eq!(salida, "6\n");
}

#[test]
fn variable_de_atrapar_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        intentar {
            lanzar "fallo"
        } atrapar error {
            imprimir("recuperado")
        }
        "#,
    );
    assert!(mensaje.contains("'error'"));
    assert!(mensaje.contains("atrapar"));
}

#[test]
fn variable_de_atrapar_usada_compila() {
    let salida = ejecutar_fuente(
        r#"
        intentar {
            lanzar "fallo"
        } atrapar error {
            imprimir(a"recuperado: {error}")
        }
        "#,
    )
    .unwrap();
    assert_eq!(salida, "recuperado: fallo\n");
}

#[test]
fn guion_bajo_descarta_la_comprobacion() {
    let salida = ejecutar_fuente(
        r#"
        fn f() {
            _ = 1
            _ignorada = 3
            retornar 4
        }
        imprimir(f())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "4\n");
}

#[test]
fn una_variable_solo_escrita_cuenta_como_no_usada() {
    let mensaje = mensaje_error(
        r#"
        fn f() {
            x = 1
            x = 2
            retornar 3
        }
        imprimir(f())
        "#,
    );
    assert!(mensaje.contains("'x'"));
}

#[test]
fn constante_local_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        fn f() {
            constante LIMITE = 10
            retornar 0
        }
        imprimir(f())
        "#,
    );
    assert!(mensaje.contains("LIMITE"));
}

#[test]
fn bloque_con_variable_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        si verdadero {
            dentro = 1
        }
        imprimir("fin")
        "#,
    );
    assert!(mensaje.contains("'dentro'"));
    assert!(mensaje.contains("bloque"));
}

#[test]
fn funciones_anidadas_pueden_usar_variables_exteriores() {
    let salida = ejecutar_fuente(
        r#"
        fn exterior() {
            fn incrementar() {
                contador = contador + 1
            }
            contador = 0
            incrementar()
            imprimir(contador)
            contador
        }
        exterior()
        "#,
    )
    .unwrap();
    assert_eq!(salida, "1\n");
}

#[test]
fn cierre_autorreferente_no_se_reporta() {
    let salida = ejecutar_fuente(
        r#"
        fn f() {
            g = fn() { retornar g }
            retornar tipo(g())
        }
        imprimir(f())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "Funcion\n");
}

#[test]
fn importaciones_no_usadas_estan_permitidas() {
    let salida = ejecutar_fuente(
        r#"
        importar json
        importar mate
        imprimir("ok")
        "#,
    )
    .unwrap();
    assert_eq!(salida, "ok\n");
}

#[test]
fn punto_y_coma_sigue_siendo_opcional() {
    let salida = ejecutar_fuente(
        r#"
        fn f() { x = 1; retornar x; }
        imprimir(f());
        "#,
    )
    .unwrap();
    assert_eq!(salida, "1\n");
}

#[test]
fn metodo_con_variable_no_usada_es_error() {
    let mensaje = mensaje_error(
        r#"
        clase Ejemplo {
            fn revisar() {
                temporal = 1
                retornar 2
            }
        }
        e = nuevo Ejemplo()
        imprimir(e.revisar())
        "#,
    );
    assert!(mensaje.contains("temporal"), "mensaje: {}", mensaje);
    assert!(mensaje.contains("Ejemplo.revisar"), "mensaje: {}", mensaje);
}

#[test]
fn el_repl_recuerda_globales_entre_ejecuciones() {
    let mut interprete = Interprete::con_captura();
    interprete.ejecutar_fuente("x = 1").unwrap();
    interprete
        .ejecutar_fuente("fn f() { x = 2 }\nf()\nimprimir(x)")
        .unwrap();
    assert_eq!(interprete.tomar_salida(), "2\n");
}
