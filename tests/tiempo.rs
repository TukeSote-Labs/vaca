//! Pruebas del módulo `tiempo`: partes, campos y formatos.

use vaca::ejecutar_fuente;

fn ejecutar(fuente: &str) -> String {
    ejecutar_fuente(fuente).expect("ejecutar programa de tiempo")
}

#[test]
fn partes_y_campos_de_una_fecha() {
    let salida = ejecutar(
        r#"
importar tiempo

afirmar(tiempo.año(0) == 1970, "año de epoch 0")
afirmar(tiempo.mes(0) == 1, "mes de epoch 0")
afirmar(tiempo.dia(0) == 1, "día de epoch 0")
afirmar(tiempo.hora(0) == 0, "hora de epoch 0")
afirmar(tiempo.minuto(0) == 0, "minuto de epoch 0")
afirmar(tiempo.segundo(0) == 0, "segundo de epoch 0")
afirmar(tiempo.dia_semana(0) == 4, "1970-01-01 fue jueves")
afirmar(tiempo.dia_del_año(0) == 1, "primer día del año")
afirmar(tiempo.semana(0) == 1, "semana ISO de epoch 0")

p = tiempo.partes(1709164800)
afirmar(p.año == 2024, "año bisiesto")
afirmar(p.mes == 2, "mes de febrero")
afirmar(p.dia == 29, "29 de febrero")
afirmar(p.dia_del_año == 60, "día 60")
afirmar(p.dia_semana == 4, "jueves")
afirmar(p.semana == 9, "semana 9")
afirmar(p.zona == "UTC", "zona UTC")

afirmar(tiempo.año(-1) == 1969, "epoch negativo")
afirmar(tiempo.semana(1609459200) == 53, "semana 53 de 2020")

imprimir("ok")
"#,
    );
    assert_eq!(salida.trim(), "ok");
}

#[test]
fn formatos_con_codigos() {
    let salida = ejecutar(
        r#"
importar tiempo

afirmar(tiempo.formatear("A-M-D H:m:S", 1700000000) == "2023-11-14 22:13:20", "formato completo")
afirmar(tiempo.fecha_iso("A-M-D", 1700000000) == "2023-11-14", "fecha_iso con formato")
afirmar(tiempo.fecha("H:m", 0) == "00:00", "fecha con hora y minuto")
afirmar(tiempo.iso("A", 0) == "1970", "año con iso")
afirmar(tiempo.iso("M", 0) == "01", "mes con iso")
afirmar(tiempo.iso("D", 0) == "01", "día con iso")
afirmar(tiempo.iso("J", 1709164800) == "060", "día del año")
afirmar(tiempo.formatear("W", 0) == "4", "día de la semana")
afirmar(tiempo.formatear("E", 0) == "0", "epoch")
afirmar(tiempo.formatear("Z", 0) == "UTC", "zona")
afirmar(tiempo.formatear("\\A", 0) == "A", "escape")

imprimir("ok")
"#,
    );
    assert_eq!(salida.trim(), "ok");
}

#[test]
fn totales_desde_epoch() {
    let salida = ejecutar(
        r#"
importar tiempo

afirmar(tiempo.minutos_totales(3600) == 60, "minutos totales")
afirmar(tiempo.horas_totales(7200) == 2, "horas totales")
afirmar(tiempo.dias_totales(172800) == 2, "días totales")
afirmar(tiempo.minutos_totales(-61) == -2, "minutos negativos")

imprimir("ok")
"#,
    );
    assert_eq!(salida.trim(), "ok");
}
