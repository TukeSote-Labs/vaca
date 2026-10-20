/**
 * Resaltador mínimo para Vaca.
 *
 * No pretende ser un lexer completo: colorea comentarios, textos, números,
 * palabras clave y llamadas para que los ejemplos se lean bien. El código se
 * escapa a HTML antes de inyectarse con `set:html`.
 */

const PALABRAS = new Set([
  'si',
  'sino',
  'mientras',
  'para',
  'hasta',
  'paso',
  'en',
  'romper',
  'continuar',
  'retornar',
  'fn',
  'clase',
  'nuevo',
  'este',
  'constante',
  'importar',
  'como',
  'async',
  'esperar',
  'await',
  'lanzar',
  'intentar',
  'atrapar',
  'afirmar',
  'y',
  'o',
  'no',
  'verdadero',
  'falso',
  'nulo',
]);

const NATIVAS = new Set([
  'imprimir',
  'leer',
  'entero',
  'decimal',
  'texto',
  'longitud',
  'error',
]);

const PATRON =
  /(?<comentario>#[^\n]*|\/\/[^\n]*|\/\*[\s\S]*?\*\/)|(?<cadena>a?"(?:[^"\\]|\\.)*")|(?<numero>\b\d+(?:\.\d+)?\b)|(?<palabra>[A-Za-zÁÉÍÓÚÜÑáéíóúüñ_][\wÁÉÍÓÚÜÑáéíóúüñ]*)|(?<espacio>\s+)|(?<otro>[\s\S])/g;

function escapar(texto: string): string {
  return texto
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

export function resaltar(codigo: string): string {
  let salida = '';

  for (const coincidencia of codigo.matchAll(PATRON)) {
    const grupos = coincidencia.groups ?? {};
    const token = coincidencia[0];

    if (grupos.comentario) {
      salida += `<span class="tok-com">${escapar(token)}</span>`;
      continue;
    }

    if (grupos.cadena) {
      salida += `<span class="tok-cad">${escapar(token)}</span>`;
      continue;
    }

    if (grupos.numero) {
      salida += `<span class="tok-num">${escapar(token)}</span>`;
      continue;
    }

    if (grupos.palabra) {
      if (PALABRAS.has(token)) {
        salida += `<span class="tok-kw">${escapar(token)}</span>`;
        continue;
      }

      const resto = codigo.slice((coincidencia.index ?? 0) + token.length);
      const esLlamada = /^\s*\(/.test(resto);

      if (esLlamada || NATIVAS.has(token)) {
        salida += `<span class="tok-fn">${escapar(token)}</span>`;
      } else {
        salida += `<span class="tok-id">${escapar(token)}</span>`;
      }
      continue;
    }

    salida += escapar(token);
  }

  return salida;
}
