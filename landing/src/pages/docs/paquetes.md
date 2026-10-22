---
layout: ../../layouts/Docs.astro
title: "Módulos y paquetes"
description: "Módulos estilo Go con vaca.mod, entrada iniciar.vaca, dependencias desde git, lockfile y bundles de producción verificados con SHA-256."
---

# Módulos y paquetes

Vaca organiza los programas en **módulos**, igual que Go:

- un módulo declara su nombre, su archivo de entrada y sus dependencias en
  `vaca.mod`;
- `vaca` sin argumentos ejecuta la entrada y llama a `principal()`;
- las dependencias se declaran con una **ruta de repositorio** y una **versión
  semver**;
- `vaca instalar` copia el código a `.vaca/paquetes/` y fija versión, commit y
  hash en `vaca.lock`;
- `vaca construir` genera un único archivo de producción verificado.

No hay un registro central: cada ruta apunta a un repositorio git.

## Crear un módulo

```bash
mkdir mi-api
cd mi-api
vaca init ejemplo.com/mi-api
```

Se crean dos archivos:

```text
vaca.mod       # nombre, versión, entrada y dependencias
iniciar.vaca   # punto de entrada
```

`vaca.mod`:

```text
# Módulo de Vaca
modulo ejemplo.com/mi-api
version 0.1.0
inicio iniciar.vaca
```

`iniciar.vaca`:

```vaca
# Punto de entrada del módulo ejemplo.com/mi-api.

fn principal() {
    imprimir("Hola desde ejemplo.com/mi-api")
}
```

Al ejecutar `vaca` en el directorio del módulo (o en cualquier subdirectorio),
Vaca busca la raíz, ejecuta `iniciar.vaca` y llama a `principal()` si existe.

```bash
vaca          # módulo actual
vaca .        # equivalente
vaca run      # equivalente
```

`vaca archivo.vaca` ejecuta un archivo suelto y, si define `principal()`,
también la llama. Es útil para scripts, pruebas y para ejecutar la entrada del
módulo directamente.

Para cambiar la entrada:

```text
inicio cmd/api/servidor.vaca
```

## Importar subpaquetes del propio módulo

```vaca
# util.vaca
fn despedida(nombre) {
    retornar "Adiós, " + nombre
}
```

```vaca
# iniciar.vaca
importar "ejemplo.com/mi-api/util"

fn principal() {
    imprimir(util.despedida("Vaca"))
}
```

La ruta se resuelve dentro de la raíz del módulo. Se busca, en orden:

1. `<ruta>.vaca`
2. `<ruta>/paquete.vaca`
3. `<ruta>/index.vaca`
4. `<ruta>/modulo.vaca`

No se permite `..` ni escapar de la raíz del módulo.

## Añadir una dependencia

```bash
vaca agregar github.com/usuario/util@1.2.0
```

Sin versión, se elige la última etiqueta semver:

```bash
vaca agregar github.com/usuario/util
```

El manifiesto añade una línea:

```text
modulo ejemplo.com/mi-api
version 0.1.0
inicio iniciar.vaca

requiere github.com/usuario/util ^1.2.0
```

Después:

```bash
vaca instalar
```

## Importar el paquete

```vaca
importar "github.com/usuario/util"
imprimir(util.saludo("Vaca"))
```

El alias por defecto es el último segmento de la ruta (`util`). Puedes
cambiarlo:

```vaca
importar "github.com/usuario/util" como herramientas
imprimir(herramientas.saludo("Vaca"))
```

Un paquete debe exponer un archivo de entrada en la raíz del repositorio, en
este orden:

1. `paquete.vaca`
2. `index.vaca`
3. `modulo.vaca`
4. `<ultimo-segmento>.vaca`

Las funciones, constantes y variables de nivel superior de ese archivo forman
la API del paquete. Sus importaciones relativas se cargan desde el propio
paquete.

## Versiones

Las versiones se leen de las etiquetas del repositorio (`v1.2.3` o `1.2.3`).
Las etiquetas que no son semver se ignoran.

| Restricción | Ejemplo | Acepta |
| :--- | :--- | :--- |
| Exacta | `1.2.3` | Solo `1.2.3` |
| Compatible | `^1.2.3` | `>=1.2.3` y `<2.0.0` |
| Aproximada | `~1.2.3` | `>=1.2.3` y `<1.3.0` |
| Mínima | `>=1.2.0` | `1.2.0` o superior |
| Máxima | `<2.0.0` | Anterior a `2.0.0` |
| Cualquiera | `*` | La última versión |

Para `0.x`, `^0.2.3` acepta `>=0.2.3` y `<0.3.0`.

## Dependencias transitivas

Si un paquete incluye su propio `vaca.mod`, sus dependencias también se
instalan. Los manifiestos antiguos en `vaca.paquete` (JSON) se siguen leyendo.
Si dos paquetes piden versiones incompatibles del mismo módulo, `vaca instalar`
falla y muestra las restricciones en conflicto.

## Orígenes

Por defecto, la URL es `https://<ruta>.git`. Puedes cambiarla por paquete:

```text
origen ejemplo.com/util https://git.ejemplo.com/util.git
origen ejemplo.com/pruebas file:///home/yo/pruebas
```

Se admiten `https://`, `ssh://`, `file://` y `git@`. Para repositorios privados,
usa las credenciales de git que ya tengas configuradas (SSH o un helper de
credenciales); Vaca no guarda tokens. Se rechazan `ext::` y argumentos que
empiecen por `-`.

## Reemplazos locales

Para desarrollar un paquete y una aplicación a la vez:

```text
requiere ejemplo.com/util ^1.2.0
reemplaza ejemplo.com/util ../util
```

`vaca instalar` copia el directorio local a `.vaca/paquetes/` (excluye `.git`,
`.vaca` y `vaca.lock`). Vuelve a ejecutarlo después de cambiar el paquete. Los
reemplazos no se aplican a las dependencias transitivas de tus dependencias.

## Lockfile e integridad

`vaca.lock` fija, para cada paquete:

- la versión elegida;
- el commit exacto (`git rev-parse HEAD`);
- un hash SHA-256 del árbol de archivos.

Si vuelves a instalar y el paquete ya está en `.vaca/paquetes/` con el mismo
hash, no se usa la red. Si el hash o el commit no coinciden con el lock,
`vaca instalar` falla: revisa el cambio antes de ejecutar `vaca actualizar`.

```json
{
  "paquetes": {
    "ejemplo.com/util": {
      "version": "1.2.0",
      "commit": "0f3c…",
      "hash": "sha256:…"
    }
  }
}
```

## Construir para producción

```bash
vaca instalar      # solo si hay dependencias
vaca construir
```

```text
Empaquetado .vaca/dist/mi-api.vaca (12.4 KiB)
  módulo:  ejemplo.com/mi-api
  entrada: iniciar.vaca
  sha256:  d52795c9…
  copia este archivo al servidor y ejecuta: vaca mi-api.vaca
```

El artefacto incluye:

- todos los archivos `.vaca` del módulo;
- `vaca.mod` y `vaca.lock`;
- las dependencias de `.vaca/paquetes/`.

Al ejecutarlo, Vaca verifica el SHA-256 y carga los módulos en memoria. Si un
solo byte cambia:

```text
error de ejecución: 'mi-api.vaca': el bundle está corrupto: el hash no coincide
```

También se escribe `.vaca/dist/mi-api.vaca.sha256` para verificar la descarga:

```bash
sha256sum -c mi-api.vaca.sha256
```

Límites defensivos al leer bundles ajenos: 8192 archivos, 64 MiB en total y
rutas sin `..`. El bundle contiene **código**, no datos: `.env`, CSV,
certificados y demás archivos se leen del sistema de archivos en ejecución.

## Comandos

| Comando | Efecto |
| :--- | :--- |
| `vaca init <modulo>` | Crea `vaca.mod` e `iniciar.vaca` |
| `vaca` | Ejecuta la entrada del módulo y `principal()` |
| `vaca construir` | Genera `.vaca/dist/<nombre>.vaca` |
| `vaca agregar <ruta>[@version]` | Añade la dependencia y ejecuta la instalación |
| `vaca instalar` | Descarga y verifica todas las dependencias |
| `vaca actualizar` | Re-resuelve las versiones ignorando el lock |
| `vaca listar` | Muestra módulo, entrada, dependencias y hashes |
| `vaca limpiar` | Borra `.vaca/paquetes` y `vaca.lock` |

## Seguridad

- **Los paquetes ejecutan código con los permisos del proceso.** Revisa el
  código antes de instalar algo de un tercero.
- Usa siempre `https://` para orígenes remotos; `file://` está pensado para
  desarrollo y pruebas.
- El lockfile y el hash detectan cambios inesperados, pero no sustituyen una
  auditoría: fijan lo que ya decidiste confiar.
- No hay firmas criptográficas de paquetes ni registro central.
- `.vaca/` está en `.gitignore` por defecto. Si quieres distribuir el código
  vendorizado, quita esa línea y añade `.vaca/` al repositorio, o guarda solo
  `vaca.lock`.
- Fija versiones exactas en producción (`1.2.3`) si necesitas control total;
  `^` es cómodo pero puede traer cambios menores.

## Limitaciones actuales

- Una sola versión por módulo en cada proyecto.
- El resolvedor elige la versión más alta que cumpla todas las restricciones
  conocidas; si aparece una restricción nueva que la invalida, vuelve a resolver.
- No hay subpaquetes con manifiesto propio dentro del mismo repositorio: cada
  ruta de dependencia es un repositorio.
- No se soportan etiquetas de preversión (`1.0.0-beta`).
- Requiere `git` instalado para descargar paquetes.
