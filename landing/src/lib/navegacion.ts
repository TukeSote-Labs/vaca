export type Seccion = {
  texto: string;
  id: string;
  /** 2 = sección principal, 3 = subsección. */
  nivel?: 2 | 3;
};

export type Pagina = {
  titulo: string;
  href: string;
  descripcion: string;
  secciones: Seccion[];
};

export type Grupo = {
  titulo: string;
  paginas: Pagina[];
};

const s = (texto: string, id: string, nivel: 2 | 3 = 2): Seccion => ({
  texto,
  id,
  nivel,
});

export const navegacion: Grupo[] = [
  {
    titulo: 'Empezar',
    paginas: [
      {
        titulo: 'Introducción',
        href: '/docs/',
        descripcion: 'Qué es Vaca, instalación y primer programa.',
        secciones: [
          s('Ficha técnica', 'ficha-técnica'),
          s('Instalación', 'instalación'),
          s('Tu primer programa', 'tu-primer-programa'),
          s('Convenciones', 'convenciones'),
          s('Estado del proyecto', 'estado-del-proyecto'),
        ],
      },
    ],
  },
  {
    titulo: 'El lenguaje',
    paginas: [
      {
        titulo: 'El lenguaje',
        href: '/docs/lenguaje/',
        descripcion: 'Sintaxis, tipos, control de flujo, funciones, colecciones y POO.',
        secciones: [
          s('Variables y constantes', 'variables-y-constantes'),
          s('Tipos', 'tipos'),
          s('Coerción numérica', 'coerción-numérica', 3),
          s('Texto e interpolación', 'texto-e-interpolación'),
          s('Métodos de texto', 'métodos-de-texto', 3),
          s('Métodos de número', 'métodos-de-número', 3),
          s('Operadores', 'operadores'),
          s('Verdad y falsedad', 'verdad-y-falsedad', 3),
          s('Conversiones y builtins', 'conversiones-y-builtins'),
          s('Condicionales', 'condicionales'),
          s('Bucles', 'bucles'),
          s('mientras', 'mientras', 3),
          s('para ... hasta', 'para--hasta', 3),
          s('para ... en', 'para--en', 3),
          s('romper y continuar', 'romper-y-continuar', 3),
          s('Funciones', 'funciones'),
          s('Parámetros por defecto', 'parámetros-por-defecto', 3),
          s('Closures', 'funciones-anónimas-y-closures', 3),
          s('Callbacks', 'callbacks', 3),
          s('Ámbito', 'ámbito', 3),
          s('Funciones async', 'funciones-async', 3),
          s('Listas', 'listas'),
          s('Métodos de lista', 'métodos-de-lista', 3),
          s('Diccionarios', 'diccionarios'),
          s('Métodos de diccionario', 'métodos-de-diccionario', 3),
          s('Clases y POO', 'clases-y-poo'),
          s('Excepciones', 'excepciones'),
          s('Importaciones', 'importaciones'),
          s('Comentarios y estilo', 'comentarios-y-estilo'),
        ],
      },
    ],
  },
  {
    titulo: 'Biblioteca',
    paginas: [
      {
        titulo: 'Módulos nativos',
        href: '/docs/modulos/',
        descripcion: 'JSON, SQLite, matemáticas, tiempo, sistema, archivos, datos, HTTP y más.',
        secciones: [
          s('json', 'json'),
          s('bd', 'bd'),
          s('mate', 'mate'),
          s('tiempo', 'tiempo'),
          s('sistema', 'sistema'),
          s('Archivos .env', 'archivos-env', 3),
          s('archivo', 'archivo'),
          s('cadenas', 'cadenas'),
          s('datos', 'datos'),
          s('Leer, ver y guardar', 'leer-ver-y-guardar', 3),
          s('Consultar', 'consultar', 3),
          s('Filtrar y ordenar', 'filtrar-y-ordenar', 3),
          s('Mutar y agrupar', 'mutar-y-agrupar', 3),
          s('concurrencia', 'concurrencia'),
          s('http', 'http'),
          s('Módulos de archivo', 'módulos-de-archivo'),
        ],
      },
      {
        titulo: 'HTTP y concurrencia',
        href: '/docs/servidor/',
        descripcion: 'Servidor, peticiones, respuesta, hilos, TLS y despliegue.',
        secciones: [
          s('Servidor HTTP', 'servidor-http'),
          s('Firma', 'firma', 3),
          s('Petición', 'petición', 3),
          s('Respuesta', 'respuesta', 3),
          s('API completa', 'api-completa', 3),
          s('Concurrencia', 'concurrencia'),
          s('Tareas async', 'tareas-async', 3),
          s('Módulo concurrencia', 'módulo-concurrencia', 3),
          s('Estado compartido', 'estado-compartido', 3),
          s('Cliente HTTP', 'cliente-http'),
          s('Laboratorio de estrés', 'laboratorio-de-estrés'),
          s('Despliegue', 'despliegue'),
          s('systemd', 'systemd', 3),
          s('Docker', 'docker', 3),
          s('TLS', 'tls', 3),
          s('Seguridad', 'seguridad-para-un-laboratorio'),
        ],
      },
    ],
  },
  {
    titulo: 'Práctica',
    paginas: [
      {
        titulo: 'Ejemplos de uso',
        href: '/docs/ejemplos/',
        descripcion: 'Programas completos con su salida esperada.',
        secciones: [
          s('Hola, mundo', 'hola-mundo'),
          s('FizzBuzz', 'fizzbuzz'),
          s('Factorial y Fibonacci', 'factorial-y-fibonacci'),
          s('Two Sum', 'two-sum-con-diccionario'),
          s('Búsqueda binaria', 'búsqueda-binaria'),
          s('POO con herencia', 'poo-con-herencia'),
          s('JSON y archivos', 'json-y-archivos'),
          s('DataFrames con CSV', 'dataframes-con-csv'),
          s('Servidor HTTP', 'servidor-http'),
          s('Patrón Strategy', 'patrón-strategy'),
          s('Rate limiter', 'rate-limiter-por-minuto'),
        ],
      },
      {
        titulo: 'Módulos y paquetes',
        href: '/docs/paquetes/',
        descripcion: 'vaca.mod, iniciar.vaca, dependencias, lockfile y bundles.',
        secciones: [
          s('Crear un módulo', 'crear-un-módulo'),
          s('Subpaquetes propios', 'importar-subpaquetes-del-propio-módulo'),
          s('Añadir una dependencia', 'añadir-una-dependencia'),
          s('Importar el paquete', 'importar-el-paquete'),
          s('Versiones', 'versiones'),
          s('Dependencias transitivas', 'dependencias-transitivas'),
          s('Orígenes', 'orígenes'),
          s('Reemplazos locales', 'reemplazos-locales'),
          s('Lockfile e integridad', 'lockfile-e-integridad'),
          s('Construir para producción', 'construir-para-producción'),
          s('Comandos', 'comandos'),
          s('Seguridad', 'seguridad'),
        ],
      },
      {
        titulo: 'Herramientas',
        href: '/docs/herramientas/',
        descripcion: 'CLI, REPL, editor, empaquetado, pruebas y diagnóstico.',
        secciones: [
          s('CLI', 'cli'),
          s('Módulos y bundles', 'módulos-y-bundles'),
          s('REPL', 'repl'),
          s('Variables no usadas', 'variables-no-usadas'),
          s('Tokens y AST', 'tokens-y-ast'),
          s('Editor de Zed', 'editor-de-zed'),
          s('Empaquetado', 'empaquetado'),
          s('Pruebas', 'pruebas'),
          s('VM de bytecode', 'vm-de-bytecode'),
          s('Diagnóstico de errores', 'diagnóstico-de-errores'),
          s('Límites conocidos', 'límites-conocidos'),
        ],
      },
      {
        titulo: 'Referencia rápida',
        href: '/docs/referencia/',
        descripcion: 'Palabras clave, operadores, builtins, métodos y errores.',
        secciones: [
          s('Palabras clave', 'palabras-clave'),
          s('Operadores', 'operadores'),
          s('Tipos y verdad', 'tipos-y-verdad'),
          s('Builtins', 'builtins'),
          s('Métodos esenciales', 'métodos-esenciales'),
          s('Módulos', 'módulos'),
          s('Formas de importar', 'formas-de-importar'),
          s('Chuleta', 'chuleta'),
          s('Errores frecuentes', 'errores-frecuentes'),
          s('Limitaciones', 'limitaciones'),
        ],
      },
    ],
  },
];

export const paginas = navegacion.flatMap((grupo) => grupo.paginas);

export const normalizar = (ruta: string) => (ruta.length > 1 ? ruta.replace(/\/$/, '') : ruta);
