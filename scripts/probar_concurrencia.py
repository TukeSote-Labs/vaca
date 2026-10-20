#!/usr/bin/env python3
"""Prueba de carga concurrente del servidor Vaca.

Levanta ejemplos/servidor_concurrente.vaca y comprueba que:

1. Muchas peticiones lentas simultáneas se atienden en paralelo.
2. Los contadores atómicos no pierden incrementos.
3. Las peticiones POST concurrentes reciben identificadores únicos.
"""

import concurrent.futures
import json
import pathlib
import subprocess
import time
import urllib.error
import urllib.request

RAIZ = pathlib.Path(__file__).resolve().parent.parent
BINARIO = RAIZ / "target" / "debug" / "vaca"
BASE = "http://127.0.0.1:8090"
CONCURRENTES = 50
MS_LENTO = 200


def peticion(ruta, metodo="GET", cuerpo=None, timeout=30, crudo=False):
    if cuerpo is None:
        datos = None
    elif crudo:
        datos = cuerpo.encode("utf-8")
    else:
        datos = json.dumps(cuerpo).encode("utf-8")
    cabeceras = {} if datos is None else {"Content-Type": "application/json"}
    solicitud = urllib.request.Request(
        BASE + ruta, data=datos, method=metodo, headers=cabeceras
    )
    try:
        with urllib.request.urlopen(solicitud, timeout=timeout) as respuesta:
            return respuesta.status, json.loads(respuesta.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read().decode("utf-8"))


def esperar_servidor(intentos=100):
    for _ in range(intentos):
        try:
            estado, cuerpo = peticion("/salud", timeout=1)
            if estado == 200:
                return cuerpo
        except (urllib.error.URLError, ConnectionError, TimeoutError, json.JSONDecodeError):
            pass
        time.sleep(0.05)
    raise SystemExit("El servidor no respondió a tiempo")


def main():
    if not BINARIO.exists():
        raise SystemExit("Compila primero con: cargo build")

    servidor = subprocess.Popen(
        [str(BINARIO), "ejemplos/servidor_concurrente.vaca"],
        cwd=RAIZ,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        inicial = esperar_servidor()
        base = inicial["peticiones"]
        print(f"Servidor listo. Peticiones iniciales: {base}")

        # 1. Peticiones lentas en paralelo.
        inicio = time.monotonic()
        with concurrent.futures.ThreadPoolExecutor(max_workers=CONCURRENTES) as pool:
            resultados = list(
                pool.map(lambda _: peticion(f"/lento?ms={MS_LENTO}"), range(CONCURRENTES))
            )
        duracion = time.monotonic() - inicio

        for estado, cuerpo in resultados:
            assert estado == 200, (estado, cuerpo)
            assert cuerpo["esperado"] == MS_LENTO, cuerpo

        en_serie = CONCURRENTES * MS_LENTO / 1000.0
        assert duracion < en_serie * 0.5, (
            f"{CONCURRENTES} peticiones de {MS_LENTO} ms tardaron {duracion:.2f} s; "
            f"en serie serían {en_serie:.2f} s (¿sin concurrencia?)"
        )
        print(
            f"{CONCURRENTES} peticiones de {MS_LENTO} ms en {duracion:.2f} s "
            f"(en serie: {en_serie:.2f} s)"
        )

        # 2. Contadores atómicos exactos.
        estado, metricas = peticion("/metricas")
        assert estado == 200, (estado, metricas)
        assert metricas["lentas"] == CONCURRENTES, metricas
        assert metricas["peticiones"] == base + CONCURRENTES + 1, metricas

        # 3. POST concurrentes con identificadores únicos.
        total_posts = 40
        cargas = [{"indice": i, "texto": f"valor-{i}"} for i in range(total_posts)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=20) as pool:
            respuestas = list(
                pool.map(lambda carga: peticion("/eco", "POST", carga), cargas)
            )

        identificadores = set()
        for carga, (estado, cuerpo) in zip(cargas, respuestas):
            assert estado == 200, (estado, cuerpo)
            assert cuerpo["recibido"] == carga, cuerpo
            identificadores.add(cuerpo["peticion"])

        assert len(identificadores) == total_posts, (
            "se repitieron identificadores: el contador atómico falló"
        )
        print(f"{total_posts} POST concurrentes con identificadores únicos")

        # 4. Los JSON inválidos y los parámetros fuera de rango se cuentan.
        for _ in range(2):
            estado, cuerpo = peticion("/eco", "POST", '{"roto"', crudo=True)
            assert estado == 400, (estado, cuerpo)
        estado, metricas = peticion("/metricas")
        assert metricas["errores"] == 2, metricas

        estado, _ = peticion("/lento?ms=-1")
        assert estado == 400, estado
        estado, metricas = peticion("/metricas")
        assert metricas["errores"] == 3, metricas

        print("Servidor concurrente de Vaca: OK")
    finally:
        servidor.terminate()
        try:
            servidor.wait(timeout=3)
        except subprocess.TimeoutExpired:
            servidor.kill()


if __name__ == "__main__":
    main()
