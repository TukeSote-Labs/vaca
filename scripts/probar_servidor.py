#!/usr/bin/env python3
"""Prueba de humo del servidor HTTP de Vaca.

Levanta ejemplos/servidor.vaca, hace varias peticiones y lo detiene.
Uso: python3 scripts/probar_servidor.py
"""

import json
import pathlib
import subprocess
import time
import urllib.error
import urllib.request

RAIZ = pathlib.Path(__file__).resolve().parent.parent
BINARIO = RAIZ / "target" / "debug" / "vaca"
BASE = "http://127.0.0.1:8080"


def peticion(ruta, metodo="GET", cuerpo=None):
    datos = None
    cabeceras = {}
    if cuerpo is not None:
        datos = cuerpo.encode("utf-8")
        cabeceras["Content-Type"] = "application/json"
    solicitud = urllib.request.Request(
        BASE + ruta, data=datos, method=metodo, headers=cabeceras
    )
    try:
        with urllib.request.urlopen(solicitud, timeout=5) as respuesta:
            return respuesta.status, respuesta.read().decode("utf-8")
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode("utf-8")


def main():
    if not BINARIO.exists():
        raise SystemExit("Compila primero con: cargo build")

    servidor = subprocess.Popen(
        [str(BINARIO), "ejemplos/servidor.vaca"],
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

        casos = [
            ("GET", "/salud", None),
            ("GET", "/usuarios?nombre=Ana", None),
            ("POST", "/eco", '{"hola":"mundo"}'),
            ("GET", "/no-existe", None),
        ]
        for metodo, ruta, cuerpo in casos:
            estado, respuesta = peticion(ruta, metodo, cuerpo)
            print(f"{metodo} {ruta} -> {estado}")
            print(respuesta)
            if estado >= 500:
                raise SystemExit("El servidor devolvió un error interno")
            json.loads(respuesta)
        print("\nServidor HTTP de Vaca: OK")
    finally:
        servidor.terminate()
        try:
            servidor.wait(timeout=3)
        except subprocess.TimeoutExpired:
            servidor.kill()


if __name__ == "__main__":
    main()
