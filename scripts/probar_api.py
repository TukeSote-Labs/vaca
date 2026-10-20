#!/usr/bin/env python3
"""Prueba de humo de la API de ejemplo ejemplos/servidor_api.vaca."""

import json
import pathlib
import subprocess
import time
import urllib.error
import urllib.request

RAIZ = pathlib.Path(__file__).resolve().parent.parent
BINARIO = RAIZ / "target" / "debug" / "vaca"
BASE = "http://127.0.0.1:8081"


def peticion(ruta, metodo="GET", cuerpo=None):
    datos = json.dumps(cuerpo).encode("utf-8") if cuerpo is not None else None
    cabeceras = {"Content-Type": "application/json"} if cuerpo is not None else {}
    solicitud = urllib.request.Request(
        BASE + ruta, data=datos, method=metodo, headers=cabeceras
    )
    try:
        with urllib.request.urlopen(solicitud, timeout=5) as respuesta:
            return respuesta.status, json.loads(respuesta.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        return error.code, json.loads(error.read().decode("utf-8"))


def main():
    if not BINARIO.exists():
        raise SystemExit("Compila primero con: cargo build")

    servidor = subprocess.Popen(
        [str(BINARIO), "ejemplos/servidor_api.vaca"],
        cwd=RAIZ,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    try:
        time.sleep(1.5)
        if servidor.poll() is not None:
            print(servidor.stdout.read())
            raise SystemExit("El servidor terminó inesperadamente")

        estado, cuerpo = peticion("/salud")
        assert estado == 200 and cuerpo["estado"] == "ok", (estado, cuerpo)
        assert cuerpo["productos"] == 2, cuerpo

        estado, cuerpo = peticion("/api/productos")
        assert estado == 200 and cuerpo["total"] == 2, (estado, cuerpo)

        estado, cuerpo = peticion("/api/productos?nombre=tec")
        assert estado == 200 and cuerpo["total"] == 1, (estado, cuerpo)
        assert cuerpo["productos"][0]["nombre"] == "Teclado", cuerpo

        estado, cuerpo = peticion("/api/productos/detalle?id=1")
        assert estado == 200 and cuerpo["producto"]["id"] == 1, (estado, cuerpo)

        estado, cuerpo = peticion("/api/productos/detalle?id=999")
        assert estado == 404, (estado, cuerpo)

        estado, cuerpo = peticion(
            "/api/productos", "POST", {"nombre": "Cable", "precio": 5.5}
        )
        assert estado == 201 and cuerpo["producto"]["id"] == 3, (estado, cuerpo)

        estado, cuerpo = peticion(
            "/api/productos", "POST", {"nombre": "", "precio": 5}
        )
        assert estado == 400, (estado, cuerpo)

        estado, cuerpo = peticion("/api/productos")
        assert estado == 200 and cuerpo["total"] == 3, (estado, cuerpo)
        total_final = cuerpo["total"]

        estado, cuerpo = peticion("/no-existe")
        assert estado == 404, (estado, cuerpo)

        print("API HTTP de Vaca: OK")
        print("Total de productos:", total_final)
    finally:
        servidor.terminate()
        try:
            servidor.wait(timeout=3)
        except subprocess.TimeoutExpired:
            servidor.kill()


if __name__ == "__main__":
    main()
