//! Pruebas de concurrencia real: hilos, tareas y primitivas atómicas.

use std::time::{Duration, Instant};

use vaca::ejecutar_fuente;

#[test]
fn tareas_async_se_ejecutan_en_paralelo() {
    let inicio = Instant::now();
    let salida = ejecutar_fuente(
        r#"
        importar tiempo

        async fn esperar_ms(ms) {
            tiempo.dormir(ms)
            retornar ms
        }

        a = esperar_ms(200)
        b = esperar_ms(200)
        suma_a = esperar a
        suma_b = esperar b
        imprimir(suma_a + suma_b)
        "#,
    )
    .unwrap();
    let transcurrido = inicio.elapsed();

    assert_eq!(salida, "400\n");
    assert!(
        transcurrido < Duration::from_millis(380),
        "las tareas tardaron {:?}; en serie serían ~400 ms",
        transcurrido
    );
}

#[test]
fn lanzar_una_tarea_no_bloquea() {
    let salida = ejecutar_fuente(
        r#"
        importar tiempo

        async fn lento() {
            tiempo.dormir(300)
            retornar "ok"
        }

        inicio = tiempo.millis()
        t = lento()
        lanzamiento = tiempo.millis() - inicio
        imprimir(lanzamiento < 100)
        resultado = esperar t
        imprimir(resultado)
        "#,
    )
    .unwrap();

    assert_eq!(salida, "verdadero\nok\n");
}

#[test]
fn esperar_dos_veces_reutiliza_el_resultado() {
    let salida = ejecutar_fuente(
        r#"
        importar tiempo

        async fn calcular() {
            tiempo.dormir(10)
            retornar 21
        }

        t = calcular()
        primero = esperar t
        segundo = esperar t
        imprimir(primero + segundo)
        "#,
    )
    .unwrap();

    assert_eq!(salida, "42\n");
}

#[test]
fn error_en_una_tarea_se_propaga_al_esperar() {
    let salida = ejecutar_fuente(
        r#"
        importar tiempo

        async fn fallar() {
            tiempo.dormir(5)
            lanzar "fallo dentro de la tarea"
        }

        intentar {
            esperar fallar()
        } atrapar error {
            imprimir(a"capturado: {error}")
        }
        "#,
    )
    .unwrap();

    assert_eq!(salida, "capturado: fallo dentro de la tarea\n");
}

#[test]
fn contador_atomico_no_pierde_incrementos() {
    let salida = ejecutar_fuente(
        r#"
        importar concurrencia

        contador = concurrencia.contador(0)

        fn trabajar() {
            para _ = 1 hasta 1000 {
                contador.incrementar()
            }
            retornar nulo
        }

        tareas = []
        para _ = 1 hasta 8 {
            tareas.agregar(concurrencia.tarea(trabajar))
        }
        concurrencia.esperar_todas(tareas)
        imprimir(contador.valor())
        "#,
    )
    .unwrap();

    assert_eq!(salida, "8000\n");
}

#[test]
fn diccionario_compartido_con_incremento_atomico() {
    let salida = ejecutar_fuente(
        r#"
        importar concurrencia

        visitas = {}

        fn visitar() {
            para _ = 1 hasta 500 {
                visitas.incrementar("total")
            }
            retornar nulo
        }

        tareas = []
        para _ = 1 hasta 4 {
            tareas.agregar(concurrencia.tarea(visitar))
        }
        concurrencia.esperar_todas(tareas)
        imprimir(visitas.obtener("total", 0))
        "#,
    )
    .unwrap();

    assert_eq!(salida, "2000\n");
}

#[test]
fn paralelo_ejecuta_todas_las_funciones_a_la_vez() {
    let inicio = Instant::now();
    let salida = ejecutar_fuente(
        r#"
        importar concurrencia
        importar tiempo

        fn primera() {
            tiempo.dormir(150)
            retornar "primera"
        }

        fn segunda() {
            tiempo.dormir(150)
            retornar "segunda"
        }

        imprimir(concurrencia.paralelo([primera, segunda]))
        "#,
    )
    .unwrap();
    let transcurrido = inicio.elapsed();

    assert_eq!(salida, "[\"primera\", \"segunda\"]\n");
    assert!(
        transcurrido < Duration::from_millis(280),
        "concurrencia.paralelo tardó {:?}; en serie serían ~300 ms",
        transcurrido
    );
}

#[test]
fn tarea_con_argumentos_y_esperar_todas() {
    let salida = ejecutar_fuente(
        r#"
        importar concurrencia
        importar tiempo

        fn sumar(a, b) {
            tiempo.dormir(5)
            retornar a + b
        }

        tareas = [
            concurrencia.tarea(sumar, 1, 2),
            concurrencia.tarea(sumar, 10, 20)
        ]
        resultados = concurrencia.esperar_todas(tareas)
        imprimir(resultados)
        "#,
    )
    .unwrap();

    assert_eq!(salida, "[3, 30]\n");
}
