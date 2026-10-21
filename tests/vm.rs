//! Pruebas de la máquina virtual de bytecode.
//!
//! Cada programa usa construcciones que el compilador puede traducir, para
//! comprobar que la VM produce los mismos resultados que el intérprete de árbol.

use vaca::ejecutar_fuente;

#[test]
fn bucles_con_romper_y_continuar() {
    let salida = ejecutar_fuente(
        r#"
        fn prueba() {
            suma = 0
            i = 0
            mientras i < 20 {
                i = i + 1
                si i % 2 == 0 { continuar }
                si i > 9 { romper }
                suma = suma + i
            }
            retornar suma
        }
        imprimir(prueba())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "25\n");
}

#[test]
fn para_con_paso_negativo_y_romper() {
    let salida = ejecutar_fuente(
        r#"
        fn cuenta() {
            texto_total = ""
            para i = 5 hasta 1 paso -2 {
                texto_total = texto_total + texto(i)
            }
            retornar texto_total
        }
        fn buscar() {
            encontrado = -1
            para i = 0 hasta 100 {
                si i == 7 {
                    encontrado = i
                    romper
                }
            }
            retornar encontrado
        }
        imprimir(cuenta())
        imprimir(buscar())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "531\n7\n");
}

#[test]
fn para_cada_lista_texto_y_diccionario() {
    let salida = ejecutar_fuente(
        r#"
        fn recorre() {
            suma = 0
            para n en [1, 2, 3] { suma = suma + n }
            letras = 0
            para c en "abc" { letras = letras + longitud(c) }
            claves = ""
            para k en {"b": 1, "a": 2} { claves = claves + k }
            retornar texto(suma) + texto(letras) + claves
        }
        imprimir(recorre())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "63ab\n");
}

#[test]
fn ambitos_de_bloque_y_asignacion_externa() {
    let salida = ejecutar_fuente(
        r#"
        fn ambitos() {
            total = 0
            i = 0
            mientras i < 3 {
                si i == 1 { total = total + 10 }
                interno = i * 2
                total = total + interno
                i = i + 1
            }
            retornar total
        }
        imprimir(ambitos())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "16\n");
}

#[test]
fn metodos_y_este_en_la_vm() {
    let salida = ejecutar_fuente(
        r#"
        clase Punto {
            fn iniciar(px, py) {
                este.x = px
                este.y = py
            }
            fn suma() {
                retornar este.x + este.y
            }
            fn mover(dx) {
                este.x = este.x + dx
            }
        }
        p = nuevo Punto(1, 2)
        p.mover(5)
        imprimir(p.suma())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "8\n");
}

#[test]
fn recursividad_y_parametros_por_defecto() {
    let salida = ejecutar_fuente(
        r#"
        fn potencia(base, exponente = 2) {
            si exponente == 0 { retornar 1 }
            retornar base * potencia(base, exponente - 1)
        }
        imprimir(potencia(2), potencia(3, 3))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "4 27\n");
}

#[test]
fn colecciones_e_indices_en_la_vm() {
    let salida = ejecutar_fuente(
        r#"
        fn datos() {
            lista = [3, 1, 2]
            lista[1] = 9
            lista.agregar(4)
            d = {"a": 1}
            d["b"] = 2
            d.c = 3
            retornar texto(lista) + "|" + texto(d.obtener("b")) + texto(d.c)
        }
        imprimir(datos())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "[3, 9, 2, 4]|23\n");
}

#[test]
fn interpolacion_y_texto() {
    let salida = ejecutar_fuente(
        r#"
        fn saluda(nombre) {
            retornar a"Hola, {nombre}! {1 + 2}"
        }
        imprimir(saluda("Vaca"))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "Hola, Vaca! 3\n");
}

#[test]
fn lanzar_desde_codigo_compilado() {
    let salida = ejecutar_fuente(
        r#"
        fn falla() {
            lanzar "boom"
        }
        intentar {
            falla()
        } atrapar error {
            imprimir("atrapado:", error)
        }
        "#,
    )
    .unwrap();
    assert_eq!(salida, "atrapado: boom\n");
}

#[test]
fn cierres_que_mutan_su_entorno() {
    let salida = ejecutar_fuente(
        r#"
        fn contador() {
            n = 0
            retornar fn() {
                n = n + 1
                retornar n
            }
        }
        c = contador()
        imprimir(c(), c(), c())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "1 2 3\n");
}

#[test]
fn funciones_anidadas_y_hoisting() {
    let salida = ejecutar_fuente(
        r#"
        fn exterior(x) {
            fn auxiliar(valor) {
                retornar valor * 2
            }
            retornar auxiliar(x) + 1
        }
        imprimir(exterior(5))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "11\n");
}
