// @ts-check
import { defineConfig } from 'astro/config';

/**
 * Gramática TextMate mínima para Vaca. Permite resaltar los bloques ```vaca
 * de la documentación con Shiki en tiempo de build.
 */
const gramaticaVaca = {
  name: 'vaca',
  scopeName: 'source.vaca',
  patterns: [
    { include: '#comentarios' },
    { include: '#textos' },
    { include: '#palabras' },
    { include: '#numeros' },
    { include: '#funciones' },
  ],
  repository: {
    comentarios: {
      patterns: [
        { name: 'comment.line.number-sign.vaca', match: '#.*$' },
        { name: 'comment.line.double-slash.vaca', match: '//.*$' },
        { name: 'comment.block.vaca', begin: '/\\*', end: '\\*/' },
      ],
    },
    textos: {
      patterns: [
        {
          name: 'string.quoted.double.vaca',
          begin: 'a?"',
          end: '"',
          patterns: [
            { name: 'constant.character.escape.vaca', match: '\\\\.' },
            {
              name: 'meta.interpolation.vaca',
              begin: '\\{',
              end: '\\}',
              patterns: [{ include: 'source.vaca' }],
            },
          ],
        },
      ],
    },
    palabras: {
      patterns: [
        {
          name: 'keyword.control.vaca',
          match:
            '\\b(si|sino|mientras|para|hasta|paso|en|romper|continuar|retornar|lanzar|intentar|atrapar)\\b',
        },
        {
          name: 'keyword.other.vaca',
          match:
            '\\b(clase|nuevo|este|constante|importar|como|afirmar|async|esperar|await)\\b',
        },
        { name: 'storage.type.function.vaca', match: '\\bfn\\b' },
        { name: 'constant.language.vaca', match: '\\b(verdadero|falso|nulo)\\b' },
        { name: 'keyword.operator.logical.vaca', match: '\\b(y|o|no)\\b' },
      ],
    },
    numeros: {
      patterns: [{ name: 'constant.numeric.vaca', match: '\\b\\d+(\\.\\d+)?\\b' }],
    },
    funciones: {
      patterns: [
        {
          name: 'entity.name.function.vaca',
          match: '\\b[A-Za-zÁÉÍÓÚÜÑáéíóúüñ_][\\wÁÉÍÓÚÜÑáéíóúüñ]*(?=\\s*\\()',
        },
        {
          name: 'variable.other.vaca',
          match: '\\b[A-Za-zÁÉÍÓÚÜÑáéíóúüñ_][\\wÁÉÍÓÚÜÑáéíóúüñ]*\\b',
        },
      ],
    },
  },
};

/** Tema de Shiki alineado con la paleta neobrutalista de la landing. */
const temaVaca = {
  name: 'vaca',
  type: 'dark',
  colors: {
    'editor.background': '#0a0a0a',
    'editor.foreground': '#f5f0e3',
  },
  settings: [
    { settings: { foreground: '#f5f0e3', background: '#0a0a0a' } },
    { scope: ['comment'], settings: { foreground: '#8f8f8f', fontStyle: 'italic' } },
    {
      scope: ['keyword', 'storage', 'constant.language'],
      settings: { foreground: '#ffe600' },
    },
    { scope: ['string'], settings: { foreground: '#b8ff3c' } },
    { scope: ['constant.numeric'], settings: { foreground: '#ff90e8' } },
    { scope: ['entity.name.function', 'support.function'], settings: { foreground: '#00e0ff' } },
    { scope: ['variable', 'meta.interpolation'], settings: { foreground: '#f5f0e3' } },
    { scope: ['constant.character.escape'], settings: { foreground: '#ff6b35' } },
    { scope: ['keyword.operator'], settings: { foreground: '#ff90e8' } },
  ],
};

// https://astro.build/config
export default defineConfig({
  markdown: {
    shikiConfig: {
      langs: [gramaticaVaca],
      theme: temaVaca,
      wrap: false,
    },
  },
});
