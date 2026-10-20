# Landing de Vaca

Sitio estático en [Astro](https://astro.build) para presentar el lenguaje Vaca:
características, ejemplos, checklist de un lenguaje "normal", concurrencia,
instalación, herramientas y estado real del proyecto.

El diseño es **neobrutalista**: bordes negros gruesos, sombras duras, bloques de
color saturado, tipografía pesada y cero esquinas redondeadas.

## Desarrollo

Requiere [Bun](https://bun.sh) (recomendado) o Node 22+.

```bash
cd landing
bun install
bun dev          # http://localhost:4321
```

Servidor en segundo plano:

```bash
bun astro dev --background
bun astro dev status
bun astro dev stop
```

## Producción

```bash
bun run build    # genera dist/
bun run preview  # sirve dist/ localmente
```

`dist/` es completamente estático: se puede publicar en GitHub Pages, Netlify,
Cloudflare Pages o cualquier CDN.

### GitHub Pages

El build usa rutas absolutas (`/_astro/...` y `/docs/...`). Si publicas en un
subdirectorio, por ejemplo `https://usuario.github.io/vaca/`, configura `site`
y `base` en `astro.config.mjs`:

```js
export default defineConfig({
  site: 'https://usuario.github.io',
  base: '/vaca',
});
```

Además, los enlaces internos a `/docs/` deben llevar el prefijo `/vaca/` (o
publicar en la raíz de un dominio propio o en Netlify/Cloudflare Pages). Para
un dominio propio o la raíz del sitio, deja la configuración por defecto.

## Documentación

El manual completo se publica en **`/docs/`** y está formado por siete páginas:

- Introducción e instalación.
- El lenguaje: tipos, control de flujo, funciones, colecciones, POO y errores.
- Módulos nativos: JSON, matemáticas, tiempo, archivos, texto, datos y
  concurrencia.
- HTTP y concurrencia: servidor, cliente, hilos, límites y despliegue.
- Ejemplos de uso con su salida esperada.
- Herramientas: CLI, REPL, editor, empaquetado y diagnóstico.
- Referencia rápida: palabras clave, operadores, builtins, métodos y errores.

Los archivos viven en `src/pages/docs/*.md` y usan bloques de código
`vaca`. El resaltado se genera en tiempo de build con una gramática TextMate y
un tema de Shiki definidos en `astro.config.mjs`, así que no hay JavaScript de
cliente para colorear el código.

## Estructura

```text
src/
  pages/index.astro        Composición de la página
  pages/docs/*.md          Documentación (7 páginas)
  layouts/Layout.astro     Metadatos, estilos globales y botón de copiado
  layouts/Docs.astro       Layout de documentación y estilos de prosa
  components/
    Header.astro           Navegación fija
    Hero.astro             Portada y servidor de ejemplo
    Marquee.astro          Cinta desfilante de características
    Features.astro         Rejilla de características
    Showcase.astro         Ejemplos con pestañas
    Checklist.astro        Lista honesta de lo que falta
    Concurrency.astro      Hilos, benchmark y aviso de límites
    Install.astro          Instalación en tres pasos
    Tooling.astro          CLI, REPL y extensión de editor
    Status.astro           Fases del proyecto
    Footer.astro           Enlaces y licencia
    Codigo.astro           Bloque de código con resaltado
    Ventana.astro          Ventana estilo terminal
    Comando.astro          Comando copiable
    Logo.astro             Marca de Vaca
    DocsNav.astro          Navegación lateral de la documentación
  lib/resaltar.ts          Resaltador de sintaxis de la landing
astro.config.mjs           Gramática y tema de Shiki para Vaca
public/favicon.svg         Icono
```

No hay frameworks de cliente ni dependencias de UI: solo Astro y CSS.
