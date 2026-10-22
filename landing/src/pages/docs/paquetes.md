---
layout: ../../layouts/Docs.astro
title: "Paquetes"
description: "Gestor de paquetes estilo Go: manifiesto, versiones semver, dependencias desde git, lockfile y reemplazos locales."
---

# Paquetes

Vaca incluye un gestor de paquetes inspirado en los módulos de Go:

- las dependencias se declaran con una **ruta de módulo** y una **versión**;
- las versiones son **etiquetas semver** de un repositorio git;
- `vaca instalar` copia el código a `.vaca/paquetes/` y fija versión, commit y
  hash en `vaca.lock`;
- el código importa paquetes con `importar "dominio/usuario/repo"`.

No hay un registro central: cada ruta apunta a un repositorio.

## Crear un proyecto

```bash
mkdir mi-api
cd mi-api
vaca init ejemplo.com/mi-api
```

Esto crea `vaca.paquete`:

```json
{
  "nombre": "ejemplo.com/mi-api",
  "version": "0.1.0",
  "dependencias": {},
  "reemplazos": {},
  "origenes": {}
}
```

## Añadir una dependencia

```bash
vaca agregar github.com/usuario/util@1.2.0
```

Sin versión, se elige la última etiqueta semver:

```bash
vaca agregar github.com/usuario/util
```

El manifiesto queda así:

```json
{
  "nombre": "ejemplo.com/mi-api",
  "version": "0.1.0",
  "dependencias": {
    "github.com/usuario/util": "^1.2.0"
  },
  "reemplazos": {},
  "origenes": {}
}
```

Después, instala todo con:

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

El paquete debe exponer un archivo de entrada en la raíz del repositorio, en
este orden de preferencia:

1. `paquete.vaca`
2. `index.vaca`
3. `modulo.vaca`
4. `<ultimo-segmento>.vaca`

Las funciones, constantes y variables de nivel superior de ese archivo forman
la API del paquete. Los archivos que importa con rutas relativas se cargan
desde el propio paquete.

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

Si un paquete incluye su propio `vaca.paquete`, sus dependencias también se
instalan. Si dos paquetes piden versiones incompatibles del mismo módulo,
`vaca instalar` falla y muestra las restricciones en conflicto. Para resolverlo,
ajusta las versiones en el manifiesto.

## Orígenes

Por defecto, la URL es `https://<ruta>.git`. Puedes cambiarla por paquete:

```json
{
  "origenes": {
    "ejemplo.com/util": "https://git.ejemplo.com/util.git",
    "ejemplo.com/pruebas": "file:///home/yo/pruebas"
  }
}
```

Se admiten `https://` y `file://`. Para repositorios privados, usa las
credenciales de git que ya tengas configuradas (SSH o un helper de
credenciales); Vaca no guarda tokens.

## Reemplazos locales

Para desarrollar un paquete y una aplicación a la vez:

```json
{
  "dependencias": {
    "ejemplo.com/util": "^1.2.0"
  },
  "reemplazos": {
    "ejemplo.com/util": "../util"
  }
}
```

`vaca instalar` copia el directorio local a `.vaca/paquetes/`. Vuelve a
ejecutarlo después de cambiar el paquete. Los reemplazos no se aplican a las
dependencias transitivas de tus dependencias.

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

## Comandos

| Comando | Efecto |
| :--- | :--- |
| `vaca init [nombre]` | Crea `vaca.paquete` en el directorio actual |
| `vaca agregar <ruta>[@version]` | Añade la dependencia y ejecuta la instalación |
| `vaca instalar` | Descarga y verifica todas las dependencias |
| `vaca actualizar` | Re-resuelve las versiones ignorando el lock |
| `vaca listar` | Muestra dependencias, versiones y hashes |
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
  ruta de módulo es un repositorio.
- No se soportan etiquetas de preversión (`1.0.0-beta`).
- Requiere `git` instalado para descargar paquetes.
