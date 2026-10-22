use vaca::ejecutar_fuente;

#[test]
fn imprime_y_evalua_aritmetica() {
    let salida = ejecutar_fuente("imprimir(2 + 3 * 4)\nimprimir((2 + 3) * 4)").unwrap();
    assert_eq!(salida, "14\n20\n");
}

#[test]
fn variables_constantes_y_tipos() {
    let salida = ejecutar_fuente(
        r#"
        contador: Entero = 10
        contador = contador + 5
        constante LIMITE = 20
        imprimir(contador)
        imprimir(contador <= LIMITE)
        imprimir(10 == 10.0)
        imprimir(no contador == 15)
        imprimir(no falso y verdadero)
        "#,
    )
    .unwrap();
    assert_eq!(salida, "15\nverdadero\nverdadero\nfalso\nverdadero\n");
}

#[test]
fn interpolacion_de_texto() {
    let salida = ejecutar_fuente(
        r#"
        nombre = "Mundo"
        datos = {"clave": "valor"}
        imprimir(a"Hola, {nombre}! 2+2={2 + 2}")
        imprimir(a"Con comillas simples: {datos.obtener('clave')}")
        imprimir(a"Con comillas escapadas: {datos.obtener(\"clave\")}")
        "#,
    )
    .unwrap();
    assert_eq!(
        salida,
        "Hola, Mundo! 2+2=4\nCon comillas simples: valor\nCon comillas escapadas: valor\n"
    );
}

#[test]
fn condicionales_y_bucles() {
    let salida = ejecutar_fuente(
        r#"
        suma = 0
        para i = 1 hasta 5 {
            si i == 3 { continuar }
            suma = suma + i
        }
        imprimir(suma)
        n = 0
        mientras verdadero {
            n = n + 1
            si n >= 3 { romper }
        }
        imprimir(n)
        "#,
    )
    .unwrap();
    assert_eq!(salida, "12\n3\n");
}

#[test]
fn para_cada_y_rango() {
    let salida = ejecutar_fuente(
        r#"
        total = 0
        para x en [1, 2, 3] { total = total + x }
        imprimir(total)
        imprimir(rango(3))
        imprimir(rango(1, 4))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "6\n[0, 1, 2]\n[1, 2, 3]\n");
}

#[test]
fn funciones_recursividad_y_defecto() {
    let salida = ejecutar_fuente(
        r#"
        fn factorial(n) {
            si n <= 1 { retornar 1 }
            retornar n * factorial(n - 1)
        }
        fn saludar(nombre, idioma = "ES") {
            si idioma == "ES" { retornar a"Hola, {nombre}" }
            retornar a"Hello, {nombre}"
        }
        imprimir(factorial(6))
        imprimir(saludar("Ana"))
        imprimir(saludar("Ana", "EN"))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "720\nHola, Ana\nHello, Ana\n");
}

#[test]
fn callbacks_y_closures() {
    let salida = ejecutar_fuente(
        r#"
        fn ejecutar(accion) { accion() }
        fn crear_contador() {
            n = 0
            retornar fn() {
                n = n + 1
                retornar n
            }
        }
        c = crear_contador()
        ejecutar(fn() { imprimir("callback") })
        imprimir(c())
        imprimir(c())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "callback\n1\n2\n");
}

#[test]
fn listas_y_metodos() {
    let salida = ejecutar_fuente(
        r#"
        l = []
        l.agregar(3)
        l.agregar(1)
        l.agregar(2)
        l.ordenar()
        imprimir(l)
        imprimir(l.longitud())
        imprimir(l.eliminar(0))
        l.insertar(0, 9)
        imprimir(l)
        dobles = l.mapear(fn(x) { retornar x * 2 })
        imprimir(dobles)
        pares = dobles.filtrar(fn(x) { retornar x % 4 == 0 })
        imprimir(pares)
        imprimir([1, 2].unir([3, 4]))
        "#,
    )
    .unwrap();
    assert_eq!(
        salida,
        "[1, 2, 3]\n3\n1\n[9, 2, 3]\n[18, 4, 6]\n[4]\n[1, 2, 3, 4]\n"
    );
}

#[test]
fn diccionarios_y_metodos() {
    let salida = ejecutar_fuente(
        r#"
        d = {"nombre": "Vaca", "edad": 2}
        d.set("pais", "ES")
        imprimir(d.obtener("nombre"))
        imprimir(d.obtener("falta", "defecto"))
        imprimir(d.existe("edad"))
        imprimir(d.longitud())
        imprimir(d.llaves())
        imprimir(d.eliminar("edad"))
        imprimir(d)
        "#,
    )
    .unwrap();
    assert_eq!(
        salida,
        "Vaca\ndefecto\nverdadero\n3\n[\"edad\", \"nombre\", \"pais\"]\n2\n{\"nombre\": \"Vaca\", \"pais\": \"ES\"}\n"
    );
}

#[test]
fn get_en_ingles_ya_no_existe() {
    let error = ejecutar_fuente("d = {\"a\": 1}\nimprimir(d.get(\"a\"))").unwrap_err();
    assert!(error.to_string().contains("'get'"));
}

#[test]
fn cierres_capturan_la_variable_de_cada_iteracion() {
    let salida = ejecutar_fuente(
        r#"
        fn crear() {
            fs = []
            i = 0
            mientras i < 3 {
                n = i
                fs.agregar(fn() { retornar n })
                i = i + 1
            }
            retornar fs
        }
        fs = crear()
        imprimir(fs[0](), fs[1](), fs[2]())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "0 1 2\n");
}

#[test]
fn poo_herencia_y_metodos() {
    let salida = ejecutar_fuente(
        r#"
        clase Animal {
            fn iniciar(nombre) { este.nombre = nombre }
            fn hablar() { retornar "..." }
            fn describir() { retornar a"{este.nombre} dice {este.hablar()}" }
        }
        clase Perro(Animal) {
            fn iniciar(nombre, energia = 100) {
                este.nombre = nombre
                este.energia = energia
            }
            fn hablar() { retornar "Guau" }
            fn gastar(cantidad) {
                este.energia = este.energia - cantidad
                retornar este.energia
            }
        }
        p = nuevo Perro("Firulais", 50)
        imprimir(p.describir())
        imprimir(p.gastar(20))
        imprimir(p.energia)
        "#,
    )
    .unwrap();
    assert_eq!(salida, "Firulais dice Guau\n30\n30\n");
}

#[test]
fn excepciones_y_afirmaciones() {
    let salida = ejecutar_fuente(
        r#"
        fn dividir(a, b) {
            si b == 0 { lanzar "no se puede dividir entre cero" }
            retornar a / b
        }
        intentar {
            imprimir(dividir(10, 2))
            dividir(1, 0)
        } atrapar error {
            imprimir(a"capturado: {error}")
        }
        afirmar(1 == 1, "uno es uno")
        imprimir("fin")
        "#,
    )
    .unwrap();
    assert_eq!(
        salida,
        "5\ncapturado: no se puede dividir entre cero\nfin\n"
    );
}

#[test]
fn modulos_json_y_mate() {
    let salida = ejecutar_fuente(
        r#"
        importar json
        importar mate
        datos = json.parse("{\"a\": 1, \"b\": [2, 3], \"c\": true}")
        imprimir(datos.a)
        imprimir(datos.b[1])
        imprimir(json.stringify({"x": 1, "y": [verdadero, nulo]}))
        imprimir(mate.promedio([10, 20, 30]))
        imprimir(mate.raiz(81))
        imprimir(mate.redondear(3.14159, 2))
        "#,
    )
    .unwrap();
    assert_eq!(salida, "1\n3\n{\"x\":1,\"y\":[true,null]}\n20\n9\n3.14\n");
}

#[test]
fn metodos_de_texto() {
    let salida = ejecutar_fuente(
        r#"
        t = "  Hola Mundo  "
        imprimir(t.recortar().mayusculas())
        imprimir(t.contiene("Mundo"))
        imprimir("a,b,c".dividir(","))
        imprimir("abc".invertir())
        imprimir("Hola".subtexto(1, 3))
        imprimir("abc".indice_de("b"))
        "#,
    )
    .unwrap();
    assert_eq!(
        salida,
        "HOLA MUNDO\nverdadero\n[\"a\", \"b\", \"c\"]\ncba\nol\n1\n"
    );
}

#[test]
fn dataframes_sin_archivos() {
    let salida = ejecutar_fuente(
        r#"
        importar datos
        df = datos.desde_json([
            {"ciudad": "Madrid", "monto": 10},
            {"ciudad": "Madrid", "monto": 30},
            {"ciudad": "Lima", "monto": 20}
        ])
        imprimir(datos.dimensiones(df))
        imprimir(datos.nombres(df))
        mayores = datos.filtrar(df, "monto", ">", 15)
        imprimir(datos.dimensiones(mayores))
        resumen = datos.agrupar_resumir(df, "ciudad", "monto", "suma")
        datos.ver(resumen)
        "#,
    )
    .unwrap();
    assert!(salida.contains("[3, 2]"));
    assert!(salida.contains("[\"ciudad\", \"monto\"]"));
    assert!(salida.contains("[2, 2]"));
    assert!(salida.contains("Lima"));
    assert!(salida.contains("Madrid"));
}

#[test]
fn diccionario_con_funciones_y_claves_palabra() {
    let salida = ejecutar_fuente(
        r#"
        acciones = {
            "doble": fn(x) { retornar x * 2 },
            "metodo": fn() { retornar "ok" }
        }
        imprimir(acciones.doble(21))
        imprimir(acciones.metodo())
        "#,
    )
    .unwrap();
    assert_eq!(salida, "42\nok\n");
}

#[test]
fn metodos_de_diccionario_y_lista_como_pila() {
    let salida = ejecutar_fuente(
        r#"
        pila = []
        pila.agregar(1)
        pila.agregar(2)
        imprimir(pila.pop())
        imprimir(pila)
        d = {}
        d["clave"] = 42
        imprimir(d["clave"])
        imprimir(d["otra"])
        "#,
    )
    .unwrap();
    assert_eq!(salida, "2\n[1]\n42\nnulo\n");
}

#[test]
fn errores_de_ejecucion_son_informados() {
    let error = ejecutar_fuente("imprimir(no_existe)").unwrap_err();
    let mensaje = error.to_string();
    assert!(mensaje.contains("no_existe"));
}

#[test]
fn recursion_infinita_no_desborda_el_proceso() {
    let error = ejecutar_fuente("fn f() { retornar f() }\nf()").unwrap_err();
    assert!(error.to_string().contains("recursión"));
}

#[test]
fn dividir_global_funciona_como_explode() {
    let salida = ejecutar_fuente(
        r#"
partes = dividir("Ana,30,Madrid", ",")
afirmar(longitud(partes) == 3, "esperaba 3 partes")
afirmar(partes[0] == "Ana", "primera parte incorrecta")
afirmar(partes[2] == "Madrid", "última parte incorrecta")
afirmar(dividir("sin-separador", ",")[0] == "sin-separador", "sin separador")
afirmar(longitud(dividir("a,b,", ",")) == 3, "debe conservar el vacío final")
afirmar(dividir("abc", "")[1] == "b", "separador vacío")
imprimir("ok")
"#,
    )
    .unwrap();
    assert_eq!(salida.trim(), "ok");
}
