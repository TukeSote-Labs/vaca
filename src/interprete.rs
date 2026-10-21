//! Intérprete de Árbol (tree-walking) de Vaca.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicI64;
use std::sync::{Arc, Mutex};

use crate::ast::{Expr, MetodoClase, OpBinario, OpUnario, Stmt};
use crate::error::Error;
use crate::modulos;
use crate::parser::Parser;
use crate::rapido::{ConjuntoRapido, MapaRapido};
use crate::value::{
    clave_texto, comparar_orden, valor_verdadero, valores_iguales, Clase, Funcion, Instancia,
    MapaValores, Modulo, Tarea, Value,
};

// ----------------------------------------------------------------------
// Entornos (scopes)
// ----------------------------------------------------------------------

pub struct Entorno {
    pub valores: MapaValores,
    constantes: ConjuntoRapido<String>,
    pub padre: Option<Arc<Mutex<Entorno>>>,
}

/// Resultado de intentar asignar a una variable ya existente.
pub enum Asignacion {
    Asignado,
    Constante,
    /// La variable no existe en ningún ámbito; devuelve el valor intacto.
    NoExiste(Value),
}

impl Entorno {
    pub fn nuevo(padre: Option<Arc<Mutex<Entorno>>>) -> Arc<Mutex<Entorno>> {
        Arc::new(Mutex::new(Entorno {
            valores: MapaValores::default(),
            constantes: ConjuntoRapido::default(),
            padre,
        }))
    }

    pub fn definir(&mut self, nombre: impl Into<String>, valor: Value) {
        self.valores.insert(nombre.into(), valor);
    }

    /// Actualiza una variable local sin asignar una clave nueva.
    #[inline]
    pub fn fijar(&mut self, nombre: &str, valor: Value) {
        if let Some(hueco) = self.valores.get_mut(nombre) {
            *hueco = valor;
        } else {
            self.valores.insert(nombre.to_string(), valor);
        }
    }

    pub fn definir_constante(&mut self, nombre: impl Into<String>, valor: Value) {
        let nombre = nombre.into();
        self.constantes.insert(nombre.clone());
        self.valores.insert(nombre, valor);
    }

    #[inline]
    pub fn obtener(entorno: &Arc<Mutex<Entorno>>, nombre: &str) -> Option<Value> {
        let padre = {
            let e = entorno.lock().unwrap();
            // Camino caliente: la variable suele estar en el ámbito actual.
            // Evitamos clonar el `Arc` del padre (operación atómica) al acertar.
            if let Some(valor) = e.valores.get(nombre) {
                return Some(valor.clone());
            }
            e.padre.clone()
        };
        padre.and_then(|p| Entorno::obtener(&p, nombre))
    }

    /// Comprueba si la variable existe sin clonar su valor.
    #[inline]
    pub fn existe(entorno: &Arc<Mutex<Entorno>>, nombre: &str) -> bool {
        let padre = {
            let e = entorno.lock().unwrap();
            if e.valores.contains_key(nombre) {
                return true;
            }
            e.padre.clone()
        };
        padre.is_some_and(|p| Entorno::existe(&p, nombre))
    }

    /// Busca la variable y la actualiza en un solo recorrido.
    #[inline]
    pub fn asignar(entorno: &Arc<Mutex<Entorno>>, nombre: &str, valor: Value) -> Asignacion {
        let (padre, valor) = {
            let mut e = entorno.lock().unwrap();
            if !e.constantes.is_empty() && e.constantes.contains(nombre) {
                return Asignacion::Constante;
            }
            if let Some(hueco) = e.valores.get_mut(nombre) {
                *hueco = valor;
                return Asignacion::Asignado;
            }
            (e.padre.clone(), valor)
        };
        match padre {
            Some(p) => Entorno::asignar(&p, nombre, valor),
            None => Asignacion::NoExiste(valor),
        }
    }

    pub fn es_constante(entorno: &Arc<Mutex<Entorno>>, nombre: &str) -> bool {
        let padre = {
            let e = entorno.lock().unwrap();
            if !e.constantes.is_empty() && e.constantes.contains(nombre) {
                return true;
            }
            e.padre.clone()
        };
        padre.is_some_and(|p| Entorno::es_constante(&p, nombre))
    }
}

#[derive(Debug)]
enum Flujo {
    Normal,
    Retornar(Value),
    Romper,
    Continuar,
}

// ----------------------------------------------------------------------
// Intérprete
// ----------------------------------------------------------------------

pub struct Salida {
    pub capturar: bool,
    pub buffer: String,
}

pub struct Interprete {
    pub global: Arc<Mutex<Entorno>>,
    pub max_profundidad: usize,
    profundidad: usize,
    pub salida: Arc<Mutex<Salida>>,
    pub ruta_base: PathBuf,
    pub argumentos: Vec<String>,
    pub codigo_salida: Option<i32>,
    modulos_archivo: HashMap<PathBuf, Value>,
    importando: HashSet<PathBuf>,
    /// Marcos de llamada reciclables (sin capturas de cierres).
    marcos_libres: Vec<Arc<Mutex<Entorno>>>,
    /// Vectores de argumentos reciclables.
    args_libres: Vec<Vec<Value>>,
}

/// Indica si un bloque puede ejecutarse en el ámbito padre sin crear uno nuevo.
///
/// Solo es seguro cuando el bloque no declara funciones, clases ni constantes y
/// todas sus asignaciones a variables apuntan a nombres ya existentes. Los
/// bloques anidados gestionan su propio ámbito, así que no influyen aquí.
fn puede_reusar_ambito(entorno: &Arc<Mutex<Entorno>>, sentencias: &[Stmt]) -> bool {
    sentencias.iter().all(|stmt| match stmt {
        Stmt::Funcion { .. } | Stmt::Clase { .. } | Stmt::Constante { .. } => false,
        Stmt::Asignar {
            objetivo: Expr::Variable(nombre),
            ..
        } => Entorno::existe(entorno, nombre),
        _ => true,
    })
}

impl Interprete {
    pub fn nuevo() -> Self {
        let global = Entorno::nuevo(None);
        let mut interprete = Interprete {
            global,
            max_profundidad: 64,
            profundidad: 0,
            salida: Arc::new(Mutex::new(Salida {
                capturar: false,
                buffer: String::new(),
            })),
            ruta_base: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            argumentos: Vec::new(),
            codigo_salida: None,
            modulos_archivo: HashMap::new(),
            importando: HashSet::new(),
            marcos_libres: Vec::new(),
            args_libres: Vec::new(),
        };
        interprete.registrar_globales();
        interprete
    }

    pub fn con_captura() -> Self {
        let interprete = Interprete::nuevo();
        interprete.salida.lock().unwrap().capturar = true;
        interprete
    }

    /// Crea un intérprete para un hilo de trabajo que comparte el entorno
    /// global, la salida y la configuración, pero con su propia pila.
    pub fn crear_tarea(&self) -> Interprete {
        Interprete {
            global: Arc::clone(&self.global),
            max_profundidad: self.max_profundidad,
            profundidad: 0,
            salida: Arc::clone(&self.salida),
            ruta_base: self.ruta_base.clone(),
            argumentos: self.argumentos.clone(),
            codigo_salida: None,
            modulos_archivo: HashMap::new(),
            importando: HashSet::new(),
            marcos_libres: Vec::new(),
            args_libres: Vec::new(),
        }
    }

    pub fn escribir(&mut self, texto: &str) {
        let mut salida = self.salida.lock().unwrap();
        if salida.capturar {
            salida.buffer.push_str(texto);
        } else {
            print!("{}", texto);
            let _ = io::stdout().flush();
        }
    }

    pub fn escribir_linea(&mut self, texto: &str) {
        let mut salida = self.salida.lock().unwrap();
        if salida.capturar {
            salida.buffer.push_str(texto);
            salida.buffer.push('\n');
        } else {
            println!("{}", texto);
        }
    }

    pub fn tomar_salida(&mut self) -> String {
        let mut salida = self.salida.lock().unwrap();
        std::mem::take(&mut salida.buffer)
    }

    fn registrar_globales(&mut self) {
        let nativas: Vec<(&str, usize, usize, crate::value::NativaFn)> = vec![
            ("imprimir", 0, usize::MAX, nativa_imprimir),
            ("leer", 0, 1, nativa_leer),
            ("entero", 1, 1, nativa_entero),
            ("decimal", 1, 1, nativa_decimal),
            ("texto", 1, 1, nativa_texto),
            ("logico", 1, 1, nativa_logico),
            ("longitud", 1, 1, nativa_longitud),
            ("tipo", 1, 1, nativa_tipo),
            ("afirmar", 1, 2, nativa_afirmar),
            ("rango", 1, 3, nativa_rango),
            ("error", 1, 1, nativa_error),
            ("copiar", 1, 1, nativa_copiar),
        ];
        for (nombre, min, max, funcion) in nativas {
            self.global
                .lock()
                .unwrap()
                .definir(nombre, Value::nativa(nombre, min, max, funcion));
        }
    }

    // ------------------------------------------------------------------
    // Ejecución de programas
    // ------------------------------------------------------------------

    pub fn ejecutar_fuente(&mut self, fuente: &str) -> Result<(), Error> {
        let programa = Parser::desde_codigo(fuente)?.parsear_programa()?;
        self.ejecutar_programa(&programa)
    }

    pub fn ejecutar_programa(&mut self, programa: &[Stmt]) -> Result<(), Error> {
        self.analizar(programa)?;
        let entorno = self.global.clone();
        self.hoistar(programa, &entorno)?;
        match self.ejecutar_lista(programa, &entorno)? {
            Flujo::Normal => Ok(()),
            Flujo::Retornar(_) => Err(Error::ejecucion("'retornar' fuera de una función")),
            Flujo::Romper => Err(Error::ejecucion("'romper' fuera de un bucle")),
            Flujo::Continuar => Err(Error::ejecucion("'continuar' fuera de un bucle")),
        }
    }

    /// Ejecuta un programa y devuelve el valor de la última expresión, útil para el REPL.
    pub fn ejecutar_programa_con_resultado(
        &mut self,
        programa: &[Stmt],
    ) -> Result<Option<Value>, Error> {
        self.analizar(programa)?;
        let entorno = self.global.clone();
        self.hoistar(programa, &entorno)?;
        let mut resultado = None;
        for (i, stmt) in programa.iter().enumerate() {
            if i + 1 == programa.len() {
                if let Stmt::Expresion(expr) = stmt {
                    resultado = Some(self.evaluar(expr, &entorno)?);
                    continue;
                }
            }
            match self.ejecutar_stmt(stmt, &entorno)? {
                Flujo::Normal => {}
                Flujo::Retornar(_) => {
                    return Err(Error::ejecucion("'retornar' fuera de una función"))
                }
                Flujo::Romper => return Err(Error::ejecucion("'romper' fuera de un bucle")),
                Flujo::Continuar => return Err(Error::ejecucion("'continuar' fuera de un bucle")),
            }
        }
        Ok(resultado)
    }

    pub fn evaluar_expresion(&mut self, expr: &Expr) -> Result<Value, Error> {
        let entorno = self.global.clone();
        self.evaluar(expr, &entorno)
    }

    /// Ejecuta el análisis semántico usando los nombres ya definidos en el
    /// entorno global (builtins, REPL y ejecuciones anteriores).
    fn analizar(&self, programa: &[Stmt]) -> Result<(), Error> {
        let conocidos: Vec<String> = self
            .global
            .lock()
            .unwrap()
            .valores
            .keys()
            .cloned()
            .collect();
        crate::analizador::analizar(programa, &conocidos)
    }

    fn ejecutar_lista(
        &mut self,
        sentencias: &[Stmt],
        entorno: &Arc<Mutex<Entorno>>,
    ) -> Result<Flujo, Error> {
        for stmt in sentencias {
            match self.ejecutar_stmt(stmt, entorno)? {
                Flujo::Normal => {}
                otro => return Ok(otro),
            }
        }
        Ok(Flujo::Normal)
    }

    fn ejecutar_bloque(
        &mut self,
        sentencias: &[Stmt],
        padre: &Arc<Mutex<Entorno>>,
    ) -> Result<Flujo, Error> {
        self.ejecutar_en_ambito(sentencias, padre, None)
    }

    fn ejecutar_en_ambito(
        &mut self,
        sentencias: &[Stmt],
        padre: &Arc<Mutex<Entorno>>,
        predefinir: Option<(&str, Value)>,
    ) -> Result<Flujo, Error> {
        // Si el bloque no declara nada nuevo, ejecutarlo en el padre ahorra
        // un nivel de búsqueda en cada acceso a variable.
        if predefinir.is_none() && puede_reusar_ambito(padre, sentencias) {
            return self.ejecutar_lista(sentencias, padre);
        }
        let entorno = Entorno::nuevo(Some(padre.clone()));
        if let Some((nombre, valor)) = predefinir {
            entorno.lock().unwrap().definir(nombre, valor);
        }
        self.hoistar(sentencias, &entorno)?;
        self.ejecutar_lista(sentencias, &entorno)
    }

    fn hoistar(&mut self, sentencias: &[Stmt], entorno: &Arc<Mutex<Entorno>>) -> Result<(), Error> {
        // La mayoría de bloques no declara nada: evitamos el recorrido completo.
        if !sentencias
            .iter()
            .any(|stmt| matches!(stmt, Stmt::Funcion { .. } | Stmt::Clase { .. }))
        {
            return Ok(());
        }

        // Primero funciones, para permitir llamadas mutuas.
        for stmt in sentencias {
            if let Stmt::Funcion {
                nombre,
                parametros,
                cuerpo,
                es_async,
            } = stmt
            {
                let funcion = Funcion {
                    nombre: nombre.clone(),
                    parametros: parametros.clone(),
                    cuerpo: cuerpo.clone(),
                    entorno: entorno.clone(),
                    es_async: *es_async,
                };
                entorno
                    .lock()
                    .unwrap()
                    .definir(nombre.clone(), Value::Funcion(Arc::new(funcion)));
            }
        }

        // Después clases, para que las clases padre ya declaradas antes existan.
        for stmt in sentencias {
            if let Stmt::Clase {
                nombre,
                padre,
                metodos,
            } = stmt
            {
                let padre_valor = match padre {
                    Some(p) => match Entorno::obtener(entorno, p) {
                        Some(Value::Clase(c)) => Some(c),
                        Some(_) => {
                            return Err(Error::ejecucion(format!(
                                "'{}' no es una clase y no puede ser padre de '{}'",
                                p, nombre
                            )))
                        }
                        None => {
                            return Err(Error::ejecucion(format!(
                                "la clase padre '{}' no está definida",
                                p
                            )))
                        }
                    },
                    None => None,
                };
                let clase = self.construir_clase(nombre, padre_valor, metodos, entorno);
                entorno
                    .lock()
                    .unwrap()
                    .definir(nombre.clone(), Value::Clase(Arc::new(clase)));
            }
        }
        Ok(())
    }

    fn construir_clase(
        &mut self,
        nombre: &str,
        padre: Option<Arc<Clase>>,
        metodos: &[MetodoClase],
        entorno: &Arc<Mutex<Entorno>>,
    ) -> Clase {
        let mut mapa = MapaRapido::default();
        for metodo in metodos {
            let funcion = Funcion {
                nombre: format!("{}.{}", nombre, metodo.nombre),
                parametros: metodo.parametros.clone(),
                cuerpo: metodo.cuerpo.clone(),
                entorno: entorno.clone(),
                es_async: metodo.es_async,
            };
            mapa.insert(metodo.nombre.clone(), Arc::new(funcion));
        }
        Clase {
            nombre: nombre.to_string(),
            padre,
            metodos: mapa,
        }
    }

    fn ejecutar_stmt(
        &mut self,
        stmt: &Stmt,
        entorno: &Arc<Mutex<Entorno>>,
    ) -> Result<Flujo, Error> {
        match stmt {
            Stmt::Expresion(expr) => {
                self.evaluar(expr, entorno)?;
                Ok(Flujo::Normal)
            }
            Stmt::Asignar { objetivo, valor } => {
                let valor = self.evaluar(valor, entorno)?;
                self.asignar(objetivo, valor, entorno)?;
                Ok(Flujo::Normal)
            }
            Stmt::Constante { nombre, valor } => {
                if Entorno::es_constante(entorno, nombre) {
                    return Err(Error::ejecucion(format!(
                        "la constante '{}' ya está definida",
                        nombre
                    )));
                }
                let valor = self.evaluar(valor, entorno)?;
                entorno
                    .lock()
                    .unwrap()
                    .definir_constante(nombre.clone(), valor);
                Ok(Flujo::Normal)
            }
            Stmt::Si {
                condicion,
                entonces,
                sino_si,
                sino,
            } => {
                if valor_verdadero(&self.evaluar(condicion, entorno)?) {
                    return self.ejecutar_bloque(entonces, entorno);
                }
                for (cond, cuerpo) in sino_si {
                    if valor_verdadero(&self.evaluar(cond, entorno)?) {
                        return self.ejecutar_bloque(cuerpo, entorno);
                    }
                }
                if let Some(cuerpo) = sino {
                    return self.ejecutar_bloque(cuerpo, entorno);
                }
                Ok(Flujo::Normal)
            }
            Stmt::Mientras { condicion, cuerpo } => {
                // Si el bloque no declara nada nuevo, se ejecuta directamente en
                // el ámbito padre y se evita un nivel de búsqueda por acceso.
                if puede_reusar_ambito(entorno, cuerpo) {
                    loop {
                        if !valor_verdadero(&self.evaluar(condicion, entorno)?) {
                            break;
                        }
                        match self.ejecutar_lista(cuerpo, entorno)? {
                            Flujo::Normal | Flujo::Continuar => {}
                            Flujo::Romper => break,
                            Flujo::Retornar(v) => return Ok(Flujo::Retornar(v)),
                        }
                    }
                    return Ok(Flujo::Normal);
                }
                // Ámbito propio por iteración: conserva la semántica de captura
                // de cierres y de variables declaradas dentro del cuerpo.
                loop {
                    if !valor_verdadero(&self.evaluar(condicion, entorno)?) {
                        break;
                    }
                    match self.ejecutar_bloque(cuerpo, entorno)? {
                        Flujo::Normal | Flujo::Continuar => {}
                        Flujo::Romper => break,
                        Flujo::Retornar(v) => return Ok(Flujo::Retornar(v)),
                    }
                }
                Ok(Flujo::Normal)
            }
            Stmt::ParaRango {
                variable,
                inicio,
                fin,
                paso,
                cuerpo,
            } => {
                let inicio = self
                    .evaluar(inicio, entorno)?
                    .a_entero_exacto()
                    .ok_or_else(|| {
                        Error::ejecucion("el inicio del bucle 'para' debe ser un entero")
                    })?;
                let fin = self
                    .evaluar(fin, entorno)?
                    .a_entero_exacto()
                    .ok_or_else(|| {
                        Error::ejecucion("el límite del bucle 'para' debe ser un entero")
                    })?;
                let paso = match paso {
                    Some(expr) => {
                        self.evaluar(expr, entorno)?
                            .a_entero_exacto()
                            .ok_or_else(|| {
                                Error::ejecucion("el paso del bucle 'para' debe ser un entero")
                            })?
                    }
                    None => 1,
                };
                if paso == 0 {
                    return Err(Error::ejecucion("el paso del bucle 'para' no puede ser 0"));
                }
                let entorno_bucle = Entorno::nuevo(Some(entorno.clone()));
                {
                    let mut e = entorno_bucle.lock().unwrap();
                    e.definir(variable.clone(), Value::Entero(inicio));
                }
                let reusar = puede_reusar_ambito(&entorno_bucle, cuerpo);
                let mut i = inicio;
                loop {
                    if (paso > 0 && i > fin) || (paso < 0 && i < fin) {
                        break;
                    }
                    entorno_bucle
                        .lock()
                        .unwrap()
                        .fijar(variable, Value::Entero(i));
                    let flujo = if reusar {
                        self.ejecutar_lista(cuerpo, &entorno_bucle)?
                    } else {
                        self.ejecutar_bloque(cuerpo, &entorno_bucle)?
                    };
                    match flujo {
                        Flujo::Normal | Flujo::Continuar => {}
                        Flujo::Romper => break,
                        Flujo::Retornar(v) => return Ok(Flujo::Retornar(v)),
                    }
                    match i.checked_add(paso) {
                        Some(siguiente) => i = siguiente,
                        None => break,
                    }
                }
                Ok(Flujo::Normal)
            }
            Stmt::ParaCada {
                variable,
                iterable,
                cuerpo,
            } => {
                let iterable = self.evaluar(iterable, entorno)?;
                let elementos: Vec<Value> = match &iterable {
                    Value::Lista(lista) => lista.lock().unwrap().clone(),
                    Value::Texto(texto) => {
                        texto.chars().map(|c| Value::Texto(c.to_string())).collect()
                    }
                    Value::Diccionario(dict) => {
                        let mut claves: Vec<String> =
                            dict.lock().unwrap().keys().cloned().collect();
                        claves.sort();
                        claves.into_iter().map(Value::Texto).collect()
                    }
                    otro => {
                        return Err(Error::ejecucion(format!(
                            "no se puede iterar sobre un valor de tipo {}",
                            otro.tipo_nombre()
                        )))
                    }
                };
                let entorno_bucle = Entorno::nuevo(Some(entorno.clone()));
                {
                    let mut e = entorno_bucle.lock().unwrap();
                    e.definir(variable.clone(), Value::Nulo);
                }
                let reusar = puede_reusar_ambito(&entorno_bucle, cuerpo);
                for elemento in elementos {
                    entorno_bucle.lock().unwrap().fijar(variable, elemento);
                    let flujo = if reusar {
                        self.ejecutar_lista(cuerpo, &entorno_bucle)?
                    } else {
                        self.ejecutar_bloque(cuerpo, &entorno_bucle)?
                    };
                    match flujo {
                        Flujo::Normal | Flujo::Continuar => {}
                        Flujo::Romper => break,
                        Flujo::Retornar(v) => return Ok(Flujo::Retornar(v)),
                    }
                }
                Ok(Flujo::Normal)
            }
            Stmt::Funcion { .. } | Stmt::Clase { .. } => Ok(Flujo::Normal), // ya izadas
            Stmt::Retornar(expr) => {
                let valor = match expr {
                    Some(e) => self.evaluar(e, entorno)?,
                    None => Value::Nulo,
                };
                Ok(Flujo::Retornar(valor))
            }
            Stmt::Romper => Ok(Flujo::Romper),
            Stmt::Continuar => Ok(Flujo::Continuar),
            Stmt::Lanzar(expr) => {
                let valor = self.evaluar(expr, entorno)?;
                Err(Error::Lanzado(valor))
            }
            Stmt::Intentar {
                cuerpo,
                variable_error,
                atrapar,
            } => match self.ejecutar_bloque(cuerpo, entorno) {
                Ok(flujo) => Ok(flujo),
                Err(Error::Salir(codigo)) => Err(Error::Salir(codigo)),
                Err(error) => {
                    let valor = match error {
                        Error::Lanzado(v) => v,
                        otro => Value::Texto(otro.to_string()),
                    };
                    let predefinir = variable_error
                        .as_ref()
                        .map(|nombre| (nombre.as_str(), valor));
                    self.ejecutar_en_ambito(atrapar, entorno, predefinir)
                }
            },
            Stmt::Importar { modulo, alias } => {
                self.ejecutar_importar(modulo, alias.clone(), entorno)?;
                Ok(Flujo::Normal)
            }
            Stmt::Bloque(sentencias) => self.ejecutar_bloque(sentencias, entorno),
        }
    }

    fn asignar(
        &mut self,
        objetivo: &Expr,
        valor: Value,
        entorno: &Arc<Mutex<Entorno>>,
    ) -> Result<(), Error> {
        match objetivo {
            Expr::Variable(nombre) => match Entorno::asignar(entorno, nombre, valor) {
                Asignacion::Asignado => Ok(()),
                Asignacion::Constante => Err(Error::ejecucion(format!(
                    "no se puede reasignar la constante '{}'",
                    nombre
                ))),
                Asignacion::NoExiste(valor) => {
                    entorno.lock().unwrap().definir(nombre.clone(), valor);
                    Ok(())
                }
            },
            Expr::Miembro { objeto, nombre } => {
                let obj = self.evaluar(objeto, entorno)?;
                match obj {
                    Value::Instancia(instancia) => {
                        instancia
                            .lock()
                            .unwrap()
                            .campos
                            .insert(nombre.clone(), valor);
                        Ok(())
                    }
                    Value::Diccionario(dict) => {
                        dict.lock().unwrap().insert(nombre.clone(), valor);
                        Ok(())
                    }
                    otro => Err(Error::ejecucion(format!(
                        "no se puede asignar el campo '{}' sobre un valor de tipo {}",
                        nombre,
                        otro.tipo_nombre()
                    ))),
                }
            }
            Expr::Indice { objeto, indice } => {
                let obj = self.evaluar(objeto, entorno)?;
                let idx = self.evaluar(indice, entorno)?;
                match obj {
                    Value::Lista(lista) => {
                        let posicion = indice_de_lista(&idx, lista.lock().unwrap().len())?;
                        lista.lock().unwrap()[posicion] = valor;
                        Ok(())
                    }
                    Value::Diccionario(dict) => {
                        dict.lock().unwrap().insert(clave_texto(&idx), valor);
                        Ok(())
                    }
                    otro => Err(Error::ejecucion(format!(
                        "no se puede indexar un valor de tipo {}",
                        otro.tipo_nombre()
                    ))),
                }
            }
            _ => Err(Error::ejecucion(
                "el lado izquierdo de una asignación no es válido",
            )),
        }
    }

    // ------------------------------------------------------------------
    // Evaluación de expresiones
    // ------------------------------------------------------------------

    fn evaluar(&mut self, expr: &Expr, entorno: &Arc<Mutex<Entorno>>) -> Result<Value, Error> {
        match expr {
            Expr::Entero(n) => Ok(Value::Entero(*n)),
            Expr::Decimal(n) => Ok(Value::Decimal(*n)),
            Expr::Texto(t) => Ok(Value::Texto(t.clone())),
            Expr::Logico(b) => Ok(Value::Logico(*b)),
            Expr::Nulo => Ok(Value::Nulo),
            Expr::Interpolado(partes) => {
                let mut texto = String::new();
                for parte in partes {
                    let valor = self.evaluar(parte, entorno)?;
                    texto.push_str(&valor.a_texto());
                }
                Ok(Value::Texto(texto))
            }
            Expr::Variable(nombre) => Entorno::obtener(entorno, nombre).ok_or_else(|| {
                Error::ejecucion(format!("la variable '{}' no está definida", nombre))
            }),
            Expr::Este => Entorno::obtener(entorno, "este")
                .ok_or_else(|| Error::ejecucion("'este' solo puede usarse dentro de un método")),
            Expr::Lista(elementos) => {
                let mut valores = Vec::with_capacity(elementos.len());
                for elemento in elementos {
                    valores.push(self.evaluar(elemento, entorno)?);
                }
                Ok(Value::lista(valores))
            }
            Expr::Diccionario(pares) => {
                let mut valores = Vec::with_capacity(pares.len());
                for (clave, expr) in pares {
                    valores.push((clave.clone(), self.evaluar(expr, entorno)?));
                }
                Ok(Value::diccionario(valores))
            }
            Expr::Binaria { op, izq, der } => {
                if *op == OpBinario::Y {
                    let izquierda = self.evaluar(izq, entorno)?;
                    if !valor_verdadero(&izquierda) {
                        return Ok(Value::Logico(false));
                    }
                    let derecha = self.evaluar(der, entorno)?;
                    return Ok(Value::Logico(valor_verdadero(&derecha)));
                }
                if *op == OpBinario::O {
                    let izquierda = self.evaluar(izq, entorno)?;
                    if valor_verdadero(&izquierda) {
                        return Ok(Value::Logico(true));
                    }
                    let derecha = self.evaluar(der, entorno)?;
                    return Ok(Value::Logico(valor_verdadero(&derecha)));
                }
                let izquierda = self.evaluar(izq, entorno)?;
                let derecha = self.evaluar(der, entorno)?;
                self.operar(*op, izquierda, derecha)
            }
            Expr::Unaria { op, expr } => {
                let valor = self.evaluar(expr, entorno)?;
                match op {
                    OpUnario::No => Ok(Value::Logico(!valor_verdadero(&valor))),
                    OpUnario::Negativo => match valor {
                        Value::Entero(n) => Ok(Value::Entero(-n)),
                        Value::Decimal(n) => Ok(Value::Decimal(-n)),
                        otro => Err(Error::ejecucion(format!(
                            "el operador '-' necesita un número, se recibió {}",
                            otro.tipo_nombre()
                        ))),
                    },
                    OpUnario::Positivo => match valor {
                        Value::Entero(_) | Value::Decimal(_) => Ok(valor),
                        otro => Err(Error::ejecucion(format!(
                            "el operador '+' necesita un número, se recibió {}",
                            otro.tipo_nombre()
                        ))),
                    },
                }
            }
            Expr::Llamada { callee, args } => {
                if let Expr::Miembro { objeto, nombre } = callee.as_ref() {
                    let receptor = self.evaluar(objeto, entorno)?;
                    let mut valores = self.tomar_args(args.len());
                    for arg in args {
                        valores.push(self.evaluar(arg, entorno)?);
                    }
                    return self.llamar_metodo(receptor, nombre, valores);
                }
                let callee = self.evaluar(callee, entorno)?;
                let mut valores = self.tomar_args(args.len());
                for arg in args {
                    valores.push(self.evaluar(arg, entorno)?);
                }
                self.llamar(callee, valores)
            }
            Expr::Miembro { objeto, nombre } => {
                let objeto = self.evaluar(objeto, entorno)?;
                self.obtener_miembro(objeto, nombre)
            }
            Expr::Indice { objeto, indice } => {
                let objeto = self.evaluar(objeto, entorno)?;
                let indice = self.evaluar(indice, entorno)?;
                match objeto {
                    Value::Lista(lista) => {
                        let posicion = indice_de_lista(&indice, lista.lock().unwrap().len())?;
                        Ok(lista.lock().unwrap()[posicion].clone())
                    }
                    Value::Diccionario(dict) => {
                        let clave = clave_texto(&indice);
                        let valor = dict.lock().unwrap().get(&clave).cloned();
                        Ok(valor.unwrap_or(Value::Nulo))
                    }
                    Value::Texto(texto) => {
                        let caracteres: Vec<char> = texto.chars().collect();
                        let posicion = indice_de_lista(&indice, caracteres.len())?;
                        Ok(Value::Texto(caracteres[posicion].to_string()))
                    }
                    otro => Err(Error::ejecucion(format!(
                        "no se puede indexar un valor de tipo {}",
                        otro.tipo_nombre()
                    ))),
                }
            }
            Expr::Nuevo { clase, args } => {
                let valor_clase = Entorno::obtener(entorno, clase).ok_or_else(|| {
                    Error::ejecucion(format!("la clase '{}' no está definida", clase))
                })?;
                let clase = match valor_clase {
                    Value::Clase(c) => c,
                    _ => return Err(Error::ejecucion(format!("'{}' no es una clase", clase))),
                };
                let mut valores = Vec::with_capacity(args.len());
                for arg in args {
                    valores.push(self.evaluar(arg, entorno)?);
                }
                self.instanciar(clase, valores)
            }
            Expr::Funcion {
                nombre,
                parametros,
                cuerpo,
                es_async,
            } => Ok(Value::Funcion(Arc::new(Funcion {
                nombre: nombre.clone().unwrap_or_default(),
                parametros: parametros.clone(),
                cuerpo: cuerpo.clone(),
                entorno: entorno.clone(),
                es_async: *es_async,
            }))),
            Expr::Esperar(expr) => {
                let valor = self.evaluar(expr, entorno)?;
                match valor {
                    Value::Tarea(tarea) => tarea.esperar(),
                    otro => Ok(otro),
                }
            }
        }
    }

    fn operar(&mut self, op: OpBinario, izq: Value, der: Value) -> Result<Value, Error> {
        match op {
            OpBinario::Suma => match (&izq, &der) {
                (Value::Entero(a), Value::Entero(b)) => a
                    .checked_add(*b)
                    .map(Value::Entero)
                    .ok_or_else(|| Error::ejecucion("desbordamiento al sumar enteros")),
                (Value::Lista(a), Value::Lista(b)) => {
                    let mut resultado = a.lock().unwrap().clone();
                    resultado.extend(b.lock().unwrap().iter().cloned());
                    Ok(Value::lista(resultado))
                }
                (Value::Texto(_), _) | (_, Value::Texto(_)) => {
                    Ok(Value::Texto(format!("{}{}", izq.a_texto(), der.a_texto())))
                }
                _ => {
                    let a = izq.a_decimal().ok_or_else(|| {
                        Error::ejecucion(format!(
                            "no se puede sumar {} y {}",
                            izq.tipo_nombre(),
                            der.tipo_nombre()
                        ))
                    })?;
                    let b = der.a_decimal().ok_or_else(|| {
                        Error::ejecucion(format!(
                            "no se puede sumar {} y {}",
                            izq.tipo_nombre(),
                            der.tipo_nombre()
                        ))
                    })?;
                    Ok(Value::Decimal(a + b))
                }
            },
            OpBinario::Resta | OpBinario::Multiplicacion => {
                if let (Value::Entero(a), Value::Entero(b)) = (&izq, &der) {
                    let resultado = if op == OpBinario::Resta {
                        a.checked_sub(*b)
                    } else {
                        a.checked_mul(*b)
                    };
                    return resultado
                        .map(Value::Entero)
                        .ok_or_else(|| Error::ejecucion("desbordamiento de enteros"));
                }
                let a = numero(&izq, op)?;
                let b = numero(&der, op)?;
                Ok(Value::Decimal(if op == OpBinario::Resta {
                    a - b
                } else {
                    a * b
                }))
            }
            OpBinario::Division => {
                if let (Value::Entero(a), Value::Entero(b)) = (&izq, &der) {
                    if *b == 0 {
                        return Err(Error::ejecucion("división entre cero"));
                    }
                    return Ok(Value::Entero(a / b));
                }
                let a = numero(&izq, op)?;
                let b = numero(&der, op)?;
                if b == 0.0 {
                    return Err(Error::ejecucion("división entre cero"));
                }
                Ok(Value::Decimal(a / b))
            }
            OpBinario::Modulo => {
                if let (Value::Entero(a), Value::Entero(b)) = (&izq, &der) {
                    if *b == 0 {
                        return Err(Error::ejecucion("módulo entre cero"));
                    }
                    return Ok(Value::Entero(a % b));
                }
                let a = numero(&izq, op)?;
                let b = numero(&der, op)?;
                if b == 0.0 {
                    return Err(Error::ejecucion("módulo entre cero"));
                }
                Ok(Value::Decimal(a % b))
            }
            OpBinario::Igual => Ok(Value::Logico(valores_iguales(&izq, &der))),
            OpBinario::Distinto => Ok(Value::Logico(!valores_iguales(&izq, &der))),
            OpBinario::Menor | OpBinario::Mayor | OpBinario::MenorIgual | OpBinario::MayorIgual => {
                let orden = comparar_orden(&izq, &der).ok_or_else(|| {
                    Error::ejecucion(format!(
                        "no se pueden comparar {} y {} con '{}'",
                        izq.tipo_nombre(),
                        der.tipo_nombre(),
                        op.simbolo()
                    ))
                })?;
                let resultado = match op {
                    OpBinario::Menor => orden.is_lt(),
                    OpBinario::Mayor => orden.is_gt(),
                    OpBinario::MenorIgual => orden.is_le(),
                    OpBinario::MayorIgual => orden.is_ge(),
                    _ => unreachable!(),
                };
                Ok(Value::Logico(resultado))
            }
            OpBinario::Y | OpBinario::O => unreachable!("se manejan con cortocircuito"),
        }
    }

    // ------------------------------------------------------------------
    // Llamadas y miembros
    // ------------------------------------------------------------------

    pub fn llamar(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, Error> {
        match callee {
            Value::Funcion(funcion) => self.llamar_funcion(&funcion, args),
            Value::Nativa(nativa) => {
                verificar_aridad(&nativa.nombre, nativa.min_args, nativa.max_args, &args)?;
                (nativa.funcion)(self, args)
            }
            Value::Clase(clase) => self.instanciar(clase, args),
            Value::MetodoNativo(metodo) => {
                let (receptor, nombre) = (*metodo).clone();
                self.despachar_metodo_nativo(receptor, &nombre, args)
            }
            otro => Err(Error::ejecucion(format!(
                "no se puede llamar un valor de tipo {}",
                otro.tipo_nombre()
            ))),
        }
    }

    fn llamar_funcion(&mut self, funcion: &Arc<Funcion>, args: Vec<Value>) -> Result<Value, Error> {
        if funcion.es_async {
            return self.iniciar_tarea(funcion.clone(), args);
        }
        self.llamar_funcion_sincrona(funcion, args)
    }

    fn iniciar_tarea(&mut self, funcion: Arc<Funcion>, args: Vec<Value>) -> Result<Value, Error> {
        let mut trabajador = self.crear_tarea();
        let handle = std::thread::Builder::new()
            .name("vaca-tarea".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || trabajador.llamar_funcion_sincrona(&funcion, args))
            .map_err(|error| Error::ejecucion(format!("no se pudo iniciar la tarea: {}", error)))?;
        Ok(Value::Tarea(Arc::new(Tarea::nueva(handle))))
    }

    /// Ejecuta cualquier valor llamable en un hilo nuevo y devuelve una tarea.
    pub fn crear_tarea_llamada(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, Error> {
        let mut trabajador = self.crear_tarea();
        let handle = std::thread::Builder::new()
            .name("vaca-tarea".to_string())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || match &callee {
                Value::Funcion(funcion) => trabajador.llamar_funcion_sincrona(funcion, args),
                otro => trabajador.llamar(otro.clone(), args),
            })
            .map_err(|error| Error::ejecucion(format!("no se pudo iniciar la tarea: {}", error)))?;
        Ok(Value::Tarea(Arc::new(Tarea::nueva(handle))))
    }

    /// Toma un vector de argumentos del reciclador.
    fn tomar_args(&mut self, n: usize) -> Vec<Value> {
        let mut valores = self.args_libres.pop().unwrap_or_default();
        valores.clear();
        if valores.capacity() < n {
            valores.reserve(n);
        }
        valores
    }

    /// Devuelve un vector de argumentos al reciclador.
    fn devolver_args(&mut self, mut valores: Vec<Value>) {
        if self.args_libres.len() < 64 {
            valores.clear();
            self.args_libres.push(valores);
        }
    }

    /// Obtiene un marco para una llamada, reutilizando uno libre si existe.
    fn tomar_marco(&mut self, padre: &Arc<Mutex<Entorno>>) -> Arc<Mutex<Entorno>> {
        if let Some(marco) = self.marcos_libres.pop() {
            {
                let mut e = marco.lock().unwrap();
                e.valores.clear();
                e.constantes.clear();
                e.padre = Some(padre.clone());
            }
            marco
        } else {
            Entorno::nuevo(Some(padre.clone()))
        }
    }

    /// Devuelve el marco al reciclador si ningún cierre lo capturó.
    fn devolver_marco(&mut self, marco: Arc<Mutex<Entorno>>) {
        if Arc::strong_count(&marco) != 1 {
            return;
        }
        {
            let mut e = marco.lock().unwrap();
            e.valores.clear();
            e.constantes.clear();
            e.padre = None;
        }
        if self.marcos_libres.len() < 64 {
            self.marcos_libres.push(marco);
        }
    }

    pub(crate) fn llamar_funcion_sincrona(
        &mut self,
        funcion: &Arc<Funcion>,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        let requeridos = funcion
            .parametros
            .iter()
            .filter(|p| p.defecto.is_none())
            .count();
        if args.len() < requeridos || args.len() > funcion.parametros.len() {
            let esperado = if requeridos == funcion.parametros.len() {
                format!("{}", requeridos)
            } else {
                format!("entre {} y {}", requeridos, funcion.parametros.len())
            };
            let nombre = if funcion.nombre.is_empty() {
                "anónima".to_string()
            } else {
                funcion.nombre.clone()
            };
            return Err(Error::ejecucion(format!(
                "la función '{}' espera {} argumento(s), pero recibió {}",
                nombre,
                esperado,
                args.len()
            )));
        }
        if self.profundidad >= self.max_profundidad {
            return Err(Error::ejecucion(
                "desbordamiento de pila: demasiada recursión (¿falta un caso base?)",
            ));
        }

        self.profundidad += 1;
        let entorno = self.tomar_marco(&funcion.entorno);
        let mut args = args;
        let resultado = (|| -> Result<Flujo, Error> {
            let mut indice = 0;
            for parametro in &funcion.parametros {
                let valor = if indice < args.len() {
                    let valor = std::mem::replace(&mut args[indice], Value::Nulo);
                    indice += 1;
                    valor
                } else {
                    match &parametro.defecto {
                        Some(expr) => self.evaluar(expr, &entorno)?,
                        None => Value::Nulo,
                    }
                };
                entorno
                    .lock()
                    .unwrap()
                    .definir(parametro.nombre.clone(), valor);
            }
            self.hoistar(&funcion.cuerpo, &entorno)?;
            self.ejecutar_lista(&funcion.cuerpo, &entorno)
        })();
        self.profundidad -= 1;
        self.devolver_args(args);
        self.devolver_marco(entorno);

        match resultado? {
            Flujo::Retornar(valor) => Ok(valor),
            Flujo::Normal => Ok(Value::Nulo),
            Flujo::Romper => Err(Error::ejecucion("'romper' fuera de un bucle")),
            Flujo::Continuar => Err(Error::ejecucion("'continuar' fuera de un bucle")),
        }
    }

    fn instanciar(&mut self, clase: Arc<Clase>, args: Vec<Value>) -> Result<Value, Error> {
        let instancia = Arc::new(Mutex::new(Instancia {
            clase: clase.clone(),
            campos: MapaValores::default(),
        }));
        let valor = Value::Instancia(instancia.clone());
        if let Some(iniciar) = buscar_metodo(&clase, "iniciar") {
            if iniciar.es_async {
                return Err(Error::ejecucion(
                    "el constructor 'iniciar' no puede ser async",
                ));
            }
            self.ejecutar_metodo(instancia.clone(), &iniciar, args)?;
        } else if !args.is_empty() {
            return Err(Error::ejecucion(format!(
                "la clase '{}' no define 'iniciar' y recibió argumentos",
                clase.nombre
            )));
        }
        Ok(valor)
    }

    fn ejecutar_metodo(
        &mut self,
        instancia: Arc<Mutex<Instancia>>,
        metodo: &Arc<Funcion>,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        let entorno = Entorno::nuevo(Some(metodo.entorno.clone()));
        entorno
            .lock()
            .unwrap()
            .definir("este", Value::Instancia(instancia));
        let vinculada = Arc::new(Funcion {
            nombre: metodo.nombre.clone(),
            parametros: metodo.parametros.clone(),
            cuerpo: metodo.cuerpo.clone(),
            entorno,
            es_async: metodo.es_async,
        });
        self.llamar_funcion(&vinculada, args)
    }

    fn obtener_miembro(&mut self, objeto: Value, nombre: &str) -> Result<Value, Error> {
        match objeto {
            Value::Instancia(instancia) => {
                if let Some(valor) = instancia.lock().unwrap().campos.get(nombre).cloned() {
                    return Ok(valor);
                }
                let clase = instancia.lock().unwrap().clase.clone();
                if let Some(metodo) = buscar_metodo(&clase, nombre) {
                    let entorno = Entorno::nuevo(Some(metodo.entorno.clone()));
                    entorno
                        .lock()
                        .unwrap()
                        .definir("este", Value::Instancia(instancia));
                    return Ok(Value::Funcion(Arc::new(Funcion {
                        nombre: metodo.nombre.clone(),
                        parametros: metodo.parametros.clone(),
                        cuerpo: metodo.cuerpo.clone(),
                        entorno,
                        es_async: metodo.es_async,
                    })));
                }
                Err(Error::ejecucion(format!(
                    "la instancia de '{}' no tiene el campo ni el método '{}'",
                    clase.nombre, nombre
                )))
            }
            Value::Diccionario(dict) => {
                let valor = dict.lock().unwrap().get(nombre).cloned();
                Ok(valor.unwrap_or(Value::Nulo))
            }
            Value::Modulo(modulo) => modulo.miembros.get(nombre).cloned().ok_or_else(|| {
                Error::ejecucion(format!(
                    "el módulo '{}' no tiene el miembro '{}'",
                    modulo.nombre, nombre
                ))
            }),
            otro @ (Value::Lista(_)
            | Value::Texto(_)
            | Value::Entero(_)
            | Value::Decimal(_)
            | Value::Logico(_)
            | Value::Tarea(_)
            | Value::Contador(_)) => Ok(Value::MetodoNativo(Arc::new((otro, nombre.to_string())))),
            otro => Err(Error::ejecucion(format!(
                "un valor de tipo {} no tiene miembros",
                otro.tipo_nombre()
            ))),
        }
    }

    pub fn llamar_metodo(
        &mut self,
        receptor: Value,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match receptor {
            Value::Instancia(instancia) => {
                if let Some(campo) = instancia.lock().unwrap().campos.get(nombre).cloned() {
                    if campo.es_llamable() {
                        return self.llamar(campo, args);
                    }
                    return Err(Error::ejecucion(format!(
                        "el campo '{}' no es una función",
                        nombre
                    )));
                }
                let clase = instancia.lock().unwrap().clase.clone();
                if let Some(metodo) = buscar_metodo(&clase, nombre) {
                    return self.ejecutar_metodo(instancia, &metodo, args);
                }
                Err(Error::ejecucion(format!(
                    "la instancia de '{}' no tiene el método '{}'",
                    clase.nombre, nombre
                )))
            }
            Value::Modulo(modulo) => {
                let miembro = modulo.miembros.get(nombre).cloned().ok_or_else(|| {
                    Error::ejecucion(format!(
                        "el módulo '{}' no tiene el miembro '{}'",
                        modulo.nombre, nombre
                    ))
                })?;
                self.llamar(miembro, args)
            }
            Value::Diccionario(dict) => {
                if let Some(campo) = dict.lock().unwrap().get(nombre).cloned() {
                    if campo.es_llamable() {
                        return self.llamar(campo, args);
                    }
                    return Err(Error::ejecucion(format!(
                        "el campo '{}' del diccionario no es una función",
                        nombre
                    )));
                }
                self.despachar_metodo_nativo(Value::Diccionario(dict), nombre, args)
            }
            Value::MetodoNativo(metodo) => {
                let (receptor, nombre) = (*metodo).clone();
                self.despachar_metodo_nativo(receptor, &nombre, args)
            }
            otro => self.despachar_metodo_nativo(otro, nombre, args),
        }
    }

    fn despachar_metodo_nativo(
        &mut self,
        receptor: Value,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match receptor {
            Value::Lista(lista) => self.metodo_lista(lista, nombre, args),
            Value::Diccionario(dict) => self.metodo_diccionario(dict, nombre, args),
            Value::Texto(texto) => self.metodo_texto(texto, nombre, args),
            Value::Entero(n) => self.metodo_numero(Value::Entero(n), nombre, args),
            Value::Decimal(n) => self.metodo_numero(Value::Decimal(n), nombre, args),
            Value::Logico(b) => self.metodo_numero(Value::Logico(b), nombre, args),
            Value::Tarea(tarea) => self.metodo_tarea(tarea, nombre, args),
            Value::Contador(contador) => self.metodo_contador(contador, nombre, args),
            otro => Err(Error::ejecucion(format!(
                "un valor de tipo {} no tiene el método '{}'",
                otro.tipo_nombre(),
                nombre
            ))),
        }
    }

    fn metodo_tarea(
        &mut self,
        tarea: Arc<Tarea>,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match nombre {
            "esperar" | "aguardar" => {
                verificar_aridad("esperar", 0, 0, &args)?;
                tarea.esperar()
            }
            otro => Err(Error::ejecucion(format!(
                "una tarea no tiene el método '{}'",
                otro
            ))),
        }
    }

    fn metodo_contador(
        &mut self,
        contador: Arc<AtomicI64>,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match nombre {
            "incrementar" | "sumar" => {
                verificar_aridad("incrementar", 0, 1, &args)?;
                let cantidad = match args.first() {
                    Some(valor) => valor.a_entero_exacto().ok_or_else(|| {
                        Error::ejecucion("la cantidad del contador debe ser un entero")
                    })?,
                    None => 1,
                };
                let nuevo =
                    contador.fetch_add(cantidad, std::sync::atomic::Ordering::SeqCst) + cantidad;
                Ok(Value::Entero(nuevo))
            }
            "valor" | "actual" => {
                verificar_aridad("valor", 0, 0, &args)?;
                Ok(Value::Entero(
                    contador.load(std::sync::atomic::Ordering::SeqCst),
                ))
            }
            "reiniciar" => {
                verificar_aridad("reiniciar", 0, 1, &args)?;
                let valor = match args.first() {
                    Some(valor) => valor.a_entero_exacto().ok_or_else(|| {
                        Error::ejecucion("el valor del contador debe ser un entero")
                    })?,
                    None => 0,
                };
                contador.store(valor, std::sync::atomic::Ordering::SeqCst);
                Ok(Value::Entero(valor))
            }
            "a_texto" => {
                verificar_aridad("a_texto", 0, 0, &args)?;
                Ok(Value::Texto(
                    contador
                        .load(std::sync::atomic::Ordering::SeqCst)
                        .to_string(),
                ))
            }
            otro => Err(Error::ejecucion(format!(
                "un contador no tiene el método '{}'",
                otro
            ))),
        }
    }

    fn metodo_lista(
        &mut self,
        lista: Arc<Mutex<Vec<Value>>>,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match nombre {
            "agregar" | "añadir" | "push" => {
                verificar_aridad("agregar", 1, 1, &args)?;
                lista.lock().unwrap().push(args[0].clone());
                Ok(Value::Nulo)
            }
            "insertar" => {
                verificar_aridad("insertar", 2, 2, &args)?;
                let indice = args[0].a_entero_exacto().ok_or_else(|| {
                    Error::ejecucion("el índice de 'insertar' debe ser un entero")
                })?;
                let len = lista.lock().unwrap().len();
                if indice < 0 || indice as usize > len {
                    return Err(Error::ejecucion(format!(
                        "índice {} fuera de rango para insertar (longitud {})",
                        indice, len
                    )));
                }
                lista
                    .lock()
                    .unwrap()
                    .insert(indice as usize, args[1].clone());
                Ok(Value::Nulo)
            }
            "eliminar" => {
                verificar_aridad("eliminar", 1, 1, &args)?;
                let posicion = indice_de_lista(&args[0], lista.lock().unwrap().len())?;
                let valor = lista.lock().unwrap().remove(posicion);
                Ok(valor)
            }
            "pop" => {
                verificar_aridad("pop", 0, 0, &args)?;
                let valor = lista.lock().unwrap().pop().unwrap_or(Value::Nulo);
                Ok(valor)
            }
            "limpiar" => {
                verificar_aridad("limpiar", 0, 0, &args)?;
                lista.lock().unwrap().clear();
                Ok(Value::Nulo)
            }
            "longitud" | "tamano" | "tamaño" => {
                verificar_aridad("longitud", 0, 0, &args)?;
                Ok(Value::Entero(lista.lock().unwrap().len() as i64))
            }
            "vacia" | "vacía" => {
                verificar_aridad("vacia", 0, 0, &args)?;
                Ok(Value::Logico(lista.lock().unwrap().is_empty()))
            }
            "unir" | "concatenar" => {
                verificar_aridad("unir", 1, 1, &args)?;
                let otra = match &args[0] {
                    Value::Lista(l) => l.lock().unwrap().clone(),
                    otro => {
                        return Err(Error::ejecucion(format!(
                            "'unir' espera una lista, se recibió {}",
                            otro.tipo_nombre()
                        )))
                    }
                };
                let mut resultado = lista.lock().unwrap().clone();
                resultado.extend(otra);
                Ok(Value::lista(resultado))
            }
            "sublista" | "rebanada" => {
                verificar_aridad("sublista", 1, 2, &args)?;
                let elementos = lista.lock().unwrap();
                let len = elementos.len() as i64;
                let inicio = args[0]
                    .a_entero_exacto()
                    .ok_or_else(|| Error::ejecucion("'sublista' espera índices enteros"))?;
                let fin = if args.len() == 2 {
                    args[1]
                        .a_entero_exacto()
                        .ok_or_else(|| Error::ejecucion("'sublista' espera índices enteros"))?
                } else {
                    len
                };
                let inicio = normalizar_indice(inicio, len);
                let fin = normalizar_indice(fin, len);
                if inicio >= fin {
                    Ok(Value::lista(Vec::new()))
                } else {
                    Ok(Value::lista(
                        elementos[inicio as usize..fin as usize].to_vec(),
                    ))
                }
            }
            "invertir" => {
                verificar_aridad("invertir", 0, 0, &args)?;
                lista.lock().unwrap().reverse();
                Ok(Value::Nulo)
            }
            "ordenar" => {
                verificar_aridad("ordenar", 0, 1, &args)?;
                let mut elementos = lista.lock().unwrap().clone();
                if args.is_empty() {
                    elementos
                        .sort_by(|a, b| comparar_orden(a, b).unwrap_or(std::cmp::Ordering::Equal));
                } else {
                    let comparador = args[0].clone();
                    if !comparador.es_llamable() {
                        return Err(Error::ejecucion(
                            "'ordenar' espera una función comparadora opcional",
                        ));
                    }
                    let mut i = 1;
                    while i < elementos.len() {
                        let mut j = i;
                        while j > 0 {
                            let resultado = self.llamar(
                                comparador.clone(),
                                vec![elementos[j - 1].clone(), elementos[j].clone()],
                            )?;
                            let debe_intercambiar = match resultado {
                                Value::Entero(n) => n > 0,
                                Value::Decimal(n) => n > 0.0,
                                Value::Logico(b) => b,
                                otro => {
                                    return Err(Error::ejecucion(format!(
                                        "el comparador de 'ordenar' debe devolver un número o lógico, devolvió {}",
                                        otro.tipo_nombre()
                                    )))
                                }
                            };
                            if debe_intercambiar {
                                elementos.swap(j - 1, j);
                                j -= 1;
                            } else {
                                break;
                            }
                        }
                        i += 1;
                    }
                }
                *lista.lock().unwrap() = elementos;
                Ok(Value::Nulo)
            }
            "contiene" => {
                verificar_aridad("contiene", 1, 1, &args)?;
                let contiene = lista
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|v| valores_iguales(v, &args[0]));
                Ok(Value::Logico(contiene))
            }
            "indice_de" | "índice_de" => {
                verificar_aridad("indice_de", 1, 1, &args)?;
                let indice = lista
                    .lock()
                    .unwrap()
                    .iter()
                    .position(|v| valores_iguales(v, &args[0]))
                    .map(|i| i as i64)
                    .unwrap_or(-1);
                Ok(Value::Entero(indice))
            }
            "copiar" => {
                verificar_aridad("copiar", 0, 0, &args)?;
                Ok(Value::lista(lista.lock().unwrap().clone()))
            }
            "mapear" => {
                verificar_aridad("mapear", 1, 1, &args)?;
                let funcion = args[0].clone();
                let elementos = lista.lock().unwrap().clone();
                let mut resultado = Vec::with_capacity(elementos.len());
                for elemento in elementos {
                    resultado.push(self.llamar(funcion.clone(), vec![elemento])?);
                }
                Ok(Value::lista(resultado))
            }
            "filtrar" => {
                verificar_aridad("filtrar", 1, 1, &args)?;
                let funcion = args[0].clone();
                let elementos = lista.lock().unwrap().clone();
                let mut resultado = Vec::new();
                for elemento in elementos {
                    let mantener = self.llamar(funcion.clone(), vec![elemento.clone()])?;
                    if valor_verdadero(&mantener) {
                        resultado.push(elemento);
                    }
                }
                Ok(Value::lista(resultado))
            }
            "reducir" => {
                verificar_aridad("reducir", 2, 2, &args)?;
                let funcion = args[0].clone();
                let mut acumulador = args[1].clone();
                let elementos = lista.lock().unwrap().clone();
                for elemento in elementos {
                    acumulador = self.llamar(funcion.clone(), vec![acumulador, elemento])?;
                }
                Ok(acumulador)
            }
            "para_cada" => {
                verificar_aridad("para_cada", 1, 1, &args)?;
                let funcion = args[0].clone();
                let elementos = lista.lock().unwrap().clone();
                for elemento in elementos {
                    self.llamar(funcion.clone(), vec![elemento])?;
                }
                Ok(Value::Nulo)
            }
            "buscar" => {
                verificar_aridad("buscar", 1, 1, &args)?;
                let funcion = args[0].clone();
                let elementos = lista.lock().unwrap().clone();
                for elemento in elementos {
                    if valor_verdadero(&self.llamar(funcion.clone(), vec![elemento.clone()])?) {
                        return Ok(elemento);
                    }
                }
                Ok(Value::Nulo)
            }
            "unir_texto" => {
                verificar_aridad("unir_texto", 1, 1, &args)?;
                let separador = args[0].a_texto();
                let texto = lista
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|v| v.a_texto())
                    .collect::<Vec<_>>()
                    .join(&separador);
                Ok(Value::Texto(texto))
            }
            "suma" => {
                verificar_aridad("suma", 0, 0, &args)?;
                let mut total = 0.0;
                let mut todo_enteros = true;
                let mut entero = 0i64;
                for valor in lista.lock().unwrap().iter() {
                    match valor {
                        Value::Entero(n) => {
                            entero = entero.checked_add(*n).ok_or_else(|| {
                                Error::ejecucion("desbordamiento al sumar la lista")
                            })?;
                            total += *n as f64;
                        }
                        Value::Decimal(n) => {
                            todo_enteros = false;
                            total += *n;
                        }
                        otro => {
                            return Err(Error::ejecucion(format!(
                                "'suma' solo admite números, se encontró {}",
                                otro.tipo_nombre()
                            )))
                        }
                    }
                }
                Ok(if todo_enteros {
                    Value::Entero(entero)
                } else {
                    Value::Decimal(total)
                })
            }
            "promedio" => {
                verificar_aridad("promedio", 0, 0, &args)?;
                let elementos = lista.lock().unwrap();
                if elementos.is_empty() {
                    return Ok(Value::Decimal(0.0));
                }
                let mut total = 0.0;
                for valor in elementos.iter() {
                    total += valor
                        .a_decimal()
                        .ok_or_else(|| Error::ejecucion("'promedio' solo admite números"))?;
                }
                Ok(Value::Decimal(total / elementos.len() as f64))
            }
            otro => Err(Error::ejecucion(format!(
                "la lista no tiene el método '{}'",
                otro
            ))),
        }
    }

    fn metodo_diccionario(
        &mut self,
        dict: Arc<Mutex<MapaValores>>,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match nombre {
            "obtener" => {
                verificar_aridad("obtener", 1, 2, &args)?;
                let clave = clave_texto(&args[0]);
                let valor = dict.lock().unwrap().get(&clave).cloned();
                Ok(valor.unwrap_or_else(|| {
                    if args.len() == 2 {
                        args[1].clone()
                    } else {
                        Value::Nulo
                    }
                }))
            }
            "set" | "poner" => {
                verificar_aridad("set", 2, 2, &args)?;
                let clave = clave_texto(&args[0]);
                dict.lock().unwrap().insert(clave, args[1].clone());
                Ok(Value::Nulo)
            }
            "incrementar" => {
                verificar_aridad("incrementar", 1, 2, &args)?;
                let clave = clave_texto(&args[0]);
                let cantidad = match args.get(1) {
                    Some(valor) => valor.a_entero_exacto().ok_or_else(|| {
                        Error::ejecucion("la cantidad de 'incrementar' debe ser un entero")
                    })?,
                    None => 1,
                };
                let mut mapa = dict.lock().unwrap();
                let actual = mapa
                    .get(&clave)
                    .and_then(|valor| valor.a_entero_exacto())
                    .unwrap_or(0);
                let nuevo = actual
                    .checked_add(cantidad)
                    .ok_or_else(|| Error::ejecucion("desbordamiento al incrementar el contador"))?;
                mapa.insert(clave, Value::Entero(nuevo));
                Ok(Value::Entero(nuevo))
            }
            "existe" | "contiene" => {
                verificar_aridad("existe", 1, 1, &args)?;
                let clave = clave_texto(&args[0]);
                Ok(Value::Logico(dict.lock().unwrap().contains_key(&clave)))
            }
            "eliminar" => {
                verificar_aridad("eliminar", 1, 1, &args)?;
                let clave = clave_texto(&args[0]);
                let valor = dict.lock().unwrap().remove(&clave);
                Ok(valor.unwrap_or(Value::Nulo))
            }
            "longitud" | "tamano" | "tamaño" => {
                verificar_aridad("longitud", 0, 0, &args)?;
                Ok(Value::Entero(dict.lock().unwrap().len() as i64))
            }
            "llaves" | "claves" => {
                verificar_aridad("llaves", 0, 0, &args)?;
                let mut claves: Vec<String> = dict.lock().unwrap().keys().cloned().collect();
                claves.sort();
                Ok(Value::lista(claves.into_iter().map(Value::Texto).collect()))
            }
            "valores" => {
                verificar_aridad("valores", 0, 0, &args)?;
                let mut claves: Vec<String> = dict.lock().unwrap().keys().cloned().collect();
                claves.sort();
                let valores = claves
                    .into_iter()
                    .filter_map(|k| dict.lock().unwrap().get(&k).cloned())
                    .collect();
                Ok(Value::lista(valores))
            }
            "items" | "elementos" => {
                verificar_aridad("items", 0, 0, &args)?;
                let mut claves: Vec<String> = dict.lock().unwrap().keys().cloned().collect();
                claves.sort();
                let mut resultado = Vec::new();
                for clave in claves {
                    if let Some(valor) = dict.lock().unwrap().get(&clave).cloned() {
                        resultado.push(Value::lista(vec![Value::Texto(clave), valor]));
                    }
                }
                Ok(Value::lista(resultado))
            }
            "copiar" => {
                verificar_aridad("copiar", 0, 0, &args)?;
                let copia = dict.lock().unwrap().clone();
                Ok(Value::Diccionario(Arc::new(Mutex::new(copia))))
            }
            "unir" => {
                verificar_aridad("unir", 1, 1, &args)?;
                let otro = match &args[0] {
                    Value::Diccionario(d) => d.lock().unwrap().clone(),
                    v => {
                        return Err(Error::ejecucion(format!(
                            "'unir' espera un diccionario, se recibió {}",
                            v.tipo_nombre()
                        )))
                    }
                };
                let mut resultado = dict.lock().unwrap().clone();
                for (clave, valor) in otro {
                    resultado.insert(clave, valor);
                }
                Ok(Value::Diccionario(Arc::new(Mutex::new(resultado))))
            }
            "limpiar" => {
                verificar_aridad("limpiar", 0, 0, &args)?;
                dict.lock().unwrap().clear();
                Ok(Value::Nulo)
            }
            otro => Err(Error::ejecucion(format!(
                "el diccionario no tiene el método '{}'",
                otro
            ))),
        }
    }

    fn metodo_texto(
        &mut self,
        texto: String,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match nombre {
            "longitud" | "tamano" | "tamaño" => {
                verificar_aridad("longitud", 0, 0, &args)?;
                Ok(Value::Entero(texto.chars().count() as i64))
            }
            "mayusculas" | "mayúsculas" => {
                verificar_aridad("mayusculas", 0, 0, &args)?;
                Ok(Value::Texto(texto.to_uppercase()))
            }
            "minusculas" | "minúsculas" => {
                verificar_aridad("minusculas", 0, 0, &args)?;
                Ok(Value::Texto(texto.to_lowercase()))
            }
            "contiene" => {
                verificar_aridad("contiene", 1, 1, &args)?;
                Ok(Value::Logico(texto.contains(&args[0].a_texto())))
            }
            "empieza_con" => {
                verificar_aridad("empieza_con", 1, 1, &args)?;
                Ok(Value::Logico(texto.starts_with(&args[0].a_texto())))
            }
            "termina_con" => {
                verificar_aridad("termina_con", 1, 1, &args)?;
                Ok(Value::Logico(texto.ends_with(&args[0].a_texto())))
            }
            "dividir" | "separar" => {
                verificar_aridad("dividir", 1, 1, &args)?;
                let separador = args[0].a_texto();
                let partes: Vec<Value> = if separador.is_empty() {
                    texto.chars().map(|c| Value::Texto(c.to_string())).collect()
                } else {
                    texto
                        .split(&separador)
                        .map(|p| Value::Texto(p.to_string()))
                        .collect()
                };
                Ok(Value::lista(partes))
            }
            "separar_lineas" => {
                verificar_aridad("separar_lineas", 0, 0, &args)?;
                Ok(Value::lista(
                    texto.lines().map(|p| Value::Texto(p.to_string())).collect(),
                ))
            }
            "reemplazar" => {
                verificar_aridad("reemplazar", 2, 2, &args)?;
                Ok(Value::Texto(
                    texto.replace(&args[0].a_texto(), &args[1].a_texto()),
                ))
            }
            "recortar" => {
                verificar_aridad("recortar", 0, 0, &args)?;
                Ok(Value::Texto(texto.trim().to_string()))
            }
            "subtexto" => {
                verificar_aridad("subtexto", 1, 2, &args)?;
                let caracteres: Vec<char> = texto.chars().collect();
                let len = caracteres.len() as i64;
                let inicio = args[0]
                    .a_entero_exacto()
                    .ok_or_else(|| Error::ejecucion("'subtexto' espera índices enteros"))?;
                let fin = if args.len() == 2 {
                    args[1]
                        .a_entero_exacto()
                        .ok_or_else(|| Error::ejecucion("'subtexto' espera índices enteros"))?
                } else {
                    len
                };
                let inicio = normalizar_indice(inicio, len);
                let fin = normalizar_indice(fin, len);
                if inicio >= fin {
                    Ok(Value::Texto(String::new()))
                } else {
                    Ok(Value::Texto(
                        caracteres[inicio as usize..fin as usize].iter().collect(),
                    ))
                }
            }
            "indice_de" | "índice_de" => {
                verificar_aridad("indice_de", 1, 1, &args)?;
                let buscado = args[0].a_texto();
                let indice = match texto.find(&buscado) {
                    Some(byte) => texto[..byte].chars().count() as i64,
                    None => -1,
                };
                Ok(Value::Entero(indice))
            }
            "repetir" => {
                verificar_aridad("repetir", 1, 1, &args)?;
                let veces = args[0].a_entero_exacto().unwrap_or(0).max(0) as usize;
                Ok(Value::Texto(texto.repeat(veces)))
            }
            "invertir" => {
                verificar_aridad("invertir", 0, 0, &args)?;
                Ok(Value::Texto(texto.chars().rev().collect()))
            }
            "caracteres" => {
                verificar_aridad("caracteres", 0, 0, &args)?;
                Ok(Value::lista(
                    texto.chars().map(|c| Value::Texto(c.to_string())).collect(),
                ))
            }
            "a_entero" => {
                verificar_aridad("a_entero", 0, 0, &args)?;
                texto
                    .trim()
                    .parse::<i64>()
                    .map(Value::Entero)
                    .map_err(|_| Error::ejecucion(format!("'{}' no es un entero válido", texto)))
            }
            "a_decimal" => {
                verificar_aridad("a_decimal", 0, 0, &args)?;
                texto
                    .trim()
                    .parse::<f64>()
                    .map(Value::Decimal)
                    .map_err(|_| Error::ejecucion(format!("'{}' no es un decimal válido", texto)))
            }
            "a_logico" => {
                verificar_aridad("a_logico", 0, 0, &args)?;
                let minusculas = texto.trim().to_lowercase();
                let valor = matches!(
                    minusculas.as_str(),
                    "verdadero" | "true" | "1" | "si" | "sí"
                );
                Ok(Value::Logico(valor))
            }
            otro => Err(Error::ejecucion(format!(
                "el texto no tiene el método '{}'",
                otro
            ))),
        }
    }

    fn metodo_numero(
        &mut self,
        valor: Value,
        nombre: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match nombre {
            "absoluto" | "abs" => {
                verificar_aridad("absoluto", 0, 0, &args)?;
                match valor {
                    Value::Entero(n) => Ok(Value::Entero(n.abs())),
                    Value::Decimal(n) => Ok(Value::Decimal(n.abs())),
                    _ => Ok(valor),
                }
            }
            "redondear" => {
                verificar_aridad("redondear", 0, 1, &args)?;
                let decimales = if args.is_empty() {
                    0i32
                } else {
                    args[0].a_entero_exacto().unwrap_or(0) as i32
                };
                let n = valor.a_decimal().unwrap_or(0.0);
                let factor = 10f64.powi(decimales);
                Ok(Value::Decimal((n * factor).round() / factor))
            }
            "piso" => {
                verificar_aridad("piso", 0, 0, &args)?;
                match valor {
                    Value::Entero(_) => Ok(valor),
                    Value::Decimal(n) => Ok(Value::Entero(n.floor() as i64)),
                    _ => Ok(valor),
                }
            }
            "techo" => {
                verificar_aridad("techo", 0, 0, &args)?;
                match valor {
                    Value::Entero(_) => Ok(valor),
                    Value::Decimal(n) => Ok(Value::Entero(n.ceil() as i64)),
                    _ => Ok(valor),
                }
            }
            "a_texto" => {
                verificar_aridad("a_texto", 0, 0, &args)?;
                Ok(Value::Texto(valor.a_texto()))
            }
            otro => Err(Error::ejecucion(format!(
                "un número no tiene el método '{}'",
                otro
            ))),
        }
    }

    // ------------------------------------------------------------------
    // Importaciones
    // ------------------------------------------------------------------

    fn ejecutar_importar(
        &mut self,
        modulo: &str,
        alias: Option<String>,
        entorno: &Arc<Mutex<Entorno>>,
    ) -> Result<(), Error> {
        if let Some(valor) = modulos::obtener_modulo(modulo) {
            let nombre = alias.unwrap_or_else(|| modulo_nombre(modulo));
            entorno.lock().unwrap().definir(nombre, valor);
            return Ok(());
        }

        let valor = self.importar_archivo(modulo)?;
        let nombre = alias.unwrap_or_else(|| {
            Path::new(modulo)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(modulo)
                .to_string()
        });
        entorno.lock().unwrap().definir(nombre, valor);
        Ok(())
    }

    fn importar_archivo(&mut self, modulo: &str) -> Result<Value, Error> {
        let mut ruta = if Path::new(modulo).is_absolute() {
            PathBuf::from(modulo)
        } else {
            self.ruta_base.join(modulo)
        };
        if !ruta.exists() && !modulo.ends_with(".vaca") {
            let con_extension = ruta.with_extension("vaca");
            if con_extension.exists() {
                ruta = con_extension;
            }
        }
        let canonico = fs::canonicalize(&ruta).map_err(|e| {
            Error::ejecucion(format!("no se pudo importar '{}': {}", ruta.display(), e))
        })?;

        if let Some(valor) = self.modulos_archivo.get(&canonico) {
            return Ok(valor.clone());
        }
        if !self.importando.insert(canonico.clone()) {
            return Err(Error::ejecucion(format!(
                "importación circular detectada en '{}'",
                ruta.display()
            )));
        }

        let anterior = self.ruta_base.clone();
        self.ruta_base = canonico
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| anterior.clone());

        let resultado = (|| -> Result<Value, Error> {
            let fuente = fs::read_to_string(&canonico).map_err(|e| {
                Error::ejecucion(format!("no se pudo leer '{}': {}", canonico.display(), e))
            })?;
            let programa = Parser::desde_codigo(&fuente)?.parsear_programa()?;
            self.analizar(&programa)?;
            let entorno = Entorno::nuevo(Some(self.global.clone()));
            self.hoistar(&programa, &entorno)?;
            match self.ejecutar_lista(&programa, &entorno)? {
                Flujo::Normal => {}
                _ => {
                    return Err(Error::ejecucion(
                        "flujo de control inválido en el nivel superior del módulo",
                    ))
                }
            }
            let miembros = entorno.lock().unwrap().valores.clone();
            let nombre = canonico
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("modulo")
                .to_string();
            Ok(Value::Modulo(Arc::new(Modulo { nombre, miembros })))
        })();

        self.ruta_base = anterior;
        self.importando.remove(&canonico);
        let valor = resultado?;
        self.modulos_archivo.insert(canonico, valor.clone());
        Ok(valor)
    }
}

// ----------------------------------------------------------------------
// Utilidades
// ----------------------------------------------------------------------

pub fn buscar_metodo(clase: &Arc<Clase>, nombre: &str) -> Option<Arc<Funcion>> {
    if let Some(metodo) = clase.metodos.get(nombre) {
        return Some(metodo.clone());
    }
    clase
        .padre
        .as_ref()
        .and_then(|padre| buscar_metodo(padre, nombre))
}

pub fn verificar_aridad(nombre: &str, min: usize, max: usize, args: &[Value]) -> Result<(), Error> {
    if args.len() < min || args.len() > max {
        let esperado = if min == max {
            format!("{}", min)
        } else if max == usize::MAX {
            format!("al menos {}", min)
        } else {
            format!("entre {} y {}", min, max)
        };
        return Err(Error::ejecucion(format!(
            "'{}' espera {} argumento(s), pero recibió {}",
            nombre,
            esperado,
            args.len()
        )));
    }
    Ok(())
}

fn numero(valor: &Value, op: OpBinario) -> Result<f64, Error> {
    valor.a_decimal().ok_or_else(|| {
        Error::ejecucion(format!(
            "el operador '{}' necesita números y se recibió {}",
            op.simbolo(),
            valor.tipo_nombre()
        ))
    })
}

pub fn indice_de_lista(indice: &Value, len: usize) -> Result<usize, Error> {
    let i = indice
        .a_entero_exacto()
        .ok_or_else(|| Error::ejecucion("el índice debe ser un entero"))?;
    let normalizado = if i < 0 { len as i64 + i } else { i };
    if normalizado < 0 || normalizado >= len as i64 {
        return Err(Error::ejecucion(format!(
            "índice {} fuera de rango (longitud {})",
            i, len
        )));
    }
    Ok(normalizado as usize)
}

fn normalizar_indice(i: i64, len: i64) -> i64 {
    if i < 0 {
        (len + i).max(0)
    } else {
        i.min(len)
    }
}

fn modulo_nombre(modulo: &str) -> String {
    modulo.split('.').next_back().unwrap_or(modulo).to_string()
}

// ----------------------------------------------------------------------
// Funciones nativas globales
// ----------------------------------------------------------------------

fn nativa_imprimir(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let texto = args
        .iter()
        .map(|v| v.a_texto())
        .collect::<Vec<_>>()
        .join(" ");
    interprete.escribir_linea(&texto);
    Ok(Value::Nulo)
}

fn nativa_leer(interprete: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    if let Some(prompt) = args.first() {
        interprete.escribir(&prompt.a_texto());
        let _ = io::stdout().flush();
    }
    let mut linea = String::new();
    let leidos = io::stdin()
        .read_line(&mut linea)
        .map_err(|e| Error::ejecucion(format!("no se pudo leer de la entrada: {}", e)))?;
    if leidos == 0 {
        return Ok(Value::Nulo);
    }
    while linea.ends_with('\n') || linea.ends_with('\r') {
        linea.pop();
    }
    Ok(Value::Texto(linea))
}

fn nativa_entero(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    match &args[0] {
        Value::Entero(n) => Ok(Value::Entero(*n)),
        Value::Decimal(n) => Ok(Value::Entero(*n as i64)),
        Value::Logico(b) => Ok(Value::Entero(if *b { 1 } else { 0 })),
        Value::Texto(t) => t
            .trim()
            .parse::<i64>()
            .map(Value::Entero)
            .map_err(|_| Error::ejecucion(format!("'{}' no se puede convertir a entero", t))),
        otro => Err(Error::ejecucion(format!(
            "no se puede convertir {} a entero",
            otro.tipo_nombre()
        ))),
    }
}

fn nativa_decimal(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    match &args[0] {
        Value::Entero(n) => Ok(Value::Decimal(*n as f64)),
        Value::Decimal(n) => Ok(Value::Decimal(*n)),
        Value::Logico(b) => Ok(Value::Decimal(if *b { 1.0 } else { 0.0 })),
        Value::Texto(t) => t
            .trim()
            .parse::<f64>()
            .map(Value::Decimal)
            .map_err(|_| Error::ejecucion(format!("'{}' no se puede convertir a decimal", t))),
        otro => Err(Error::ejecucion(format!(
            "no se puede convertir {} a decimal",
            otro.tipo_nombre()
        ))),
    }
}

fn nativa_texto(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(args[0].a_texto()))
}

fn nativa_logico(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Logico(valor_verdadero(&args[0])))
}

fn nativa_longitud(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    match &args[0] {
        Value::Texto(t) => Ok(Value::Entero(t.chars().count() as i64)),
        Value::Lista(l) => Ok(Value::Entero(l.lock().unwrap().len() as i64)),
        Value::Diccionario(d) => Ok(Value::Entero(d.lock().unwrap().len() as i64)),
        otro => Err(Error::ejecucion(format!(
            "'longitud' no admite un valor de tipo {}",
            otro.tipo_nombre()
        ))),
    }
}

fn nativa_tipo(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Texto(args[0].tipo_nombre().to_string()))
}

fn nativa_afirmar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    if valor_verdadero(&args[0]) {
        return Ok(Value::Nulo);
    }
    let mensaje = args
        .get(1)
        .map(|v| v.a_texto())
        .unwrap_or_else(|| "afirmación fallida".to_string());
    Err(Error::Lanzado(Value::Texto(mensaje)))
}

fn nativa_rango(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    let (inicio, fin, paso) = match args.len() {
        1 => (
            0,
            args[0]
                .a_entero_exacto()
                .ok_or_else(|| Error::ejecucion("'rango' espera enteros"))?,
            1,
        ),
        2 => (
            args[0]
                .a_entero_exacto()
                .ok_or_else(|| Error::ejecucion("'rango' espera enteros"))?,
            args[1]
                .a_entero_exacto()
                .ok_or_else(|| Error::ejecucion("'rango' espera enteros"))?,
            1,
        ),
        _ => (
            args[0]
                .a_entero_exacto()
                .ok_or_else(|| Error::ejecucion("'rango' espera enteros"))?,
            args[1]
                .a_entero_exacto()
                .ok_or_else(|| Error::ejecucion("'rango' espera enteros"))?,
            args[2]
                .a_entero_exacto()
                .ok_or_else(|| Error::ejecucion("'rango' espera enteros"))?,
        ),
    };
    if paso == 0 {
        return Err(Error::ejecucion("el paso de 'rango' no puede ser 0"));
    }
    let mut valores = Vec::new();
    let mut i = inicio;
    while (paso > 0 && i < fin) || (paso < 0 && i > fin) {
        valores.push(Value::Entero(i));
        match i.checked_add(paso) {
            Some(siguiente) => i = siguiente,
            None => break,
        }
    }
    Ok(Value::lista(valores))
}

fn nativa_error(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    Err(Error::Lanzado(args[0].clone()))
}

fn nativa_copiar(_: &mut Interprete, args: Vec<Value>) -> Result<Value, Error> {
    match &args[0] {
        Value::Lista(l) => Ok(Value::lista(l.lock().unwrap().clone())),
        Value::Diccionario(d) => Ok(Value::Diccionario(Arc::new(Mutex::new(
            d.lock().unwrap().clone(),
        )))),
        otro => Ok(otro.clone()),
    }
}
