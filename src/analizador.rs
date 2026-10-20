//! Análisis semántico previo a la ejecución.
//!
//! Detecta variables locales declaradas que nunca se leen, al estilo de Go.
//! Las variables globales, los parámetros de funciones y los nombres que
//! empiezan por `_` están exentos.

use std::collections::HashMap;

use crate::ast::{Expr, Parametro, Stmt};
use crate::error::Error;

/// Analiza un programa. `globales_conocidos` son los nombres ya definidos en
/// el entorno global (builtins, REPL, ejecuciones anteriores).
pub fn analizar(programa: &[Stmt], globales_conocidos: &[String]) -> Result<(), Error> {
    let mut analizador = Analizador::nuevo(globales_conocidos);
    analizador.programa(programa);
    analizador.resultado()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TipoAmbito {
    Global,
    Funcion,
    Bloque,
}

struct Declaracion {
    usada: bool,
    reportar: bool,
    contexto: String,
}

struct Ambito {
    tipo: TipoAmbito,
    variables: HashMap<String, Declaracion>,
    contexto: String,
}

struct Analizador {
    ambitos: Vec<Ambito>,
    errores: Vec<String>,
    errores_constantes: Vec<String>,
}

impl Analizador {
    fn nuevo(globales_conocidos: &[String]) -> Self {
        let mut global = Ambito {
            tipo: TipoAmbito::Global,
            variables: HashMap::new(),
            contexto: "global".to_string(),
        };
        for nombre in globales_conocidos {
            global.variables.insert(
                nombre.clone(),
                Declaracion {
                    usada: true,
                    reportar: false,
                    contexto: "global".to_string(),
                },
            );
        }
        Analizador {
            ambitos: vec![global],
            errores: Vec::new(),
            errores_constantes: Vec::new(),
        }
    }

    fn resultado(mut self) -> Result<(), Error> {
        let mut mensajes = Vec::new();

        if !self.errores_constantes.is_empty() {
            self.errores_constantes.sort();
            self.errores_constantes.dedup();
            mensajes.push(format!(
                "las constantes son inmutables y deben escribirse en MAYÚSCULAS: {}",
                self.errores_constantes.join(", ")
            ));
        }

        if !self.errores.is_empty() {
            self.errores.sort();
            self.errores.dedup();
            mensajes.push(format!(
                "variables declaradas pero nunca usadas: {}",
                self.errores.join(", ")
            ));
        }

        if mensajes.is_empty() {
            Ok(())
        } else {
            Err(Error::Semantico(mensajes.join("; ")))
        }
    }

    fn programa(&mut self, programa: &[Stmt]) {
        self.pre_escanear(programa);
        self.sentencias(programa);
    }

    fn sentencias(&mut self, sentencias: &[Stmt]) {
        for stmt in sentencias {
            self.sentencia(stmt);
        }
    }

    fn sentencia(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expresion(expr) => self.expr(expr),
            Stmt::Asignar { objetivo, valor } => self.asignacion(objetivo, valor),
            Stmt::Constante { nombre, valor } => {
                if nombre.chars().any(|c| c.is_lowercase()) {
                    self.errores_constantes.push(format!("'{}'", nombre));
                }
                self.expr(valor);
            }
            Stmt::Si {
                condicion,
                entonces,
                sino_si,
                sino,
            } => {
                self.expr(condicion);
                self.bloque(entonces);
                for (cond, cuerpo) in sino_si {
                    self.expr(cond);
                    self.bloque(cuerpo);
                }
                if let Some(cuerpo) = sino {
                    self.bloque(cuerpo);
                }
            }
            Stmt::Mientras { condicion, cuerpo } => {
                self.expr(condicion);
                self.bloque(cuerpo);
            }
            Stmt::ParaRango {
                variable,
                inicio,
                fin,
                paso,
                cuerpo,
            } => {
                self.expr(inicio);
                self.expr(fin);
                if let Some(paso) = paso {
                    self.expr(paso);
                }
                self.ambitos.push(Ambito {
                    tipo: TipoAmbito::Bloque,
                    variables: HashMap::new(),
                    contexto: format!("bucle 'para {}'", variable),
                });
                self.declarar_en_actual(variable, true);
                self.bloque(cuerpo);
                self.cerrar_ambito();
            }
            Stmt::ParaCada {
                variable,
                iterable,
                cuerpo,
            } => {
                self.expr(iterable);
                self.ambitos.push(Ambito {
                    tipo: TipoAmbito::Bloque,
                    variables: HashMap::new(),
                    contexto: format!("bucle 'para {} en ...'", variable),
                });
                self.declarar_en_actual(variable, true);
                self.bloque(cuerpo);
                self.cerrar_ambito();
            }
            Stmt::Funcion {
                nombre,
                parametros,
                cuerpo,
                ..
            } => self.funcion(nombre, parametros, cuerpo),
            Stmt::Clase {
                nombre, metodos, ..
            } => {
                for metodo in metodos {
                    self.funcion(
                        &format!("{}.{}", nombre, metodo.nombre),
                        &metodo.parametros,
                        &metodo.cuerpo,
                    );
                }
            }
            Stmt::Retornar(Some(expr)) => self.expr(expr),
            Stmt::Retornar(None) => {}
            Stmt::Romper | Stmt::Continuar => {}
            Stmt::Lanzar(expr) => self.expr(expr),
            Stmt::Intentar {
                cuerpo,
                variable_error,
                atrapar,
            } => {
                self.bloque(cuerpo);
                self.ambitos.push(Ambito {
                    tipo: TipoAmbito::Bloque,
                    variables: HashMap::new(),
                    contexto: "atrapar".to_string(),
                });
                if let Some(nombre) = variable_error {
                    self.declarar_en_actual(nombre, true);
                }
                self.pre_escanear(atrapar);
                self.sentencias(atrapar);
                self.cerrar_ambito();
            }
            Stmt::Importar { .. } => {}
            Stmt::Bloque(sentencias) => self.bloque(sentencias),
        }
    }

    fn asignacion(&mut self, objetivo: &Expr, valor: &Expr) {
        match objetivo {
            Expr::Variable(nombre) => {
                // Una asignación solo declara si el nombre no existe en
                // ningún ámbito visible, igual que en tiempo de ejecución.
                let reportar = self.debe_reportar();
                self.declarar_si_nuevo(nombre, reportar);
                self.expr(valor);
            }
            Expr::Miembro { objeto, .. } => {
                self.expr(objeto);
                self.expr(valor);
            }
            Expr::Indice { objeto, indice } => {
                self.expr(objeto);
                self.expr(indice);
                self.expr(valor);
            }
            _ => self.expr(valor),
        }
    }

    fn bloque(&mut self, sentencias: &[Stmt]) {
        self.ambitos.push(Ambito {
            tipo: TipoAmbito::Bloque,
            variables: HashMap::new(),
            contexto: self.contexto_bloque(),
        });
        self.pre_escanear(sentencias);
        self.sentencias(sentencias);
        self.cerrar_ambito();
    }

    fn funcion(&mut self, nombre: &str, parametros: &[Parametro], cuerpo: &[Stmt]) {
        self.ambitos.push(Ambito {
            tipo: TipoAmbito::Funcion,
            variables: HashMap::new(),
            contexto: format!("función '{}'", nombre),
        });

        for parametro in parametros {
            self.declarar_en_actual(&parametro.nombre, false);
        }

        // Las declaraciones locales se registran antes de analizar las
        // funciones anidadas, para no confundir escrituras a variables
        // exteriores con declaraciones locales.
        self.pre_escanear(cuerpo);

        for parametro in parametros {
            if let Some(defecto) = &parametro.defecto {
                self.expr(defecto);
            }
        }

        self.sentencias(cuerpo);
        self.cerrar_ambito();
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable(nombre) => self.usar(nombre),
            Expr::Entero(_)
            | Expr::Decimal(_)
            | Expr::Texto(_)
            | Expr::Logico(_)
            | Expr::Nulo
            | Expr::Este => {}
            Expr::Interpolado(partes) => {
                for parte in partes {
                    self.expr(parte);
                }
            }
            Expr::Lista(elementos) => {
                for elemento in elementos {
                    self.expr(elemento);
                }
            }
            Expr::Diccionario(pares) => {
                for (_, valor) in pares {
                    self.expr(valor);
                }
            }
            Expr::Binaria { izq, der, .. } => {
                self.expr(izq);
                self.expr(der);
            }
            Expr::Unaria { expr, .. } => self.expr(expr),
            Expr::Llamada { callee, args } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            Expr::Miembro { objeto, .. } => self.expr(objeto),
            Expr::Indice { objeto, indice } => {
                self.expr(objeto);
                self.expr(indice);
            }
            Expr::Nuevo { clase, args } => {
                self.usar(clase);
                for arg in args {
                    self.expr(arg);
                }
            }
            Expr::Funcion {
                nombre,
                parametros,
                cuerpo,
                ..
            } => {
                let nombre = nombre.as_deref().unwrap_or("anónima");
                self.funcion(nombre, parametros, cuerpo);
            }
            Expr::Esperar(expr) => self.expr(expr),
        }
    }

    /// Registra las declaraciones directas de un bloque antes de recorrerlo.
    fn pre_escanear(&mut self, sentencias: &[Stmt]) {
        let reportar = self.debe_reportar();
        for stmt in sentencias {
            match stmt {
                Stmt::Asignar {
                    objetivo: Expr::Variable(nombre),
                    ..
                } => self.declarar_si_nuevo(nombre, reportar),
                Stmt::Constante { nombre, .. } => self.declarar_en_actual(nombre, reportar),
                Stmt::Importar { modulo, alias } => {
                    let nombre = alias.clone().unwrap_or_else(|| nombre_importado(modulo));
                    // Los imports no se reportan como no usados: importar un
                    // archivo puede ejecutar efectos secundarios.
                    self.declarar_en_actual(&nombre, false);
                }
                Stmt::Funcion { nombre, .. } | Stmt::Clase { nombre, .. } => {
                    self.declarar_en_actual(nombre, false)
                }
                _ => {}
            }
        }
    }

    fn debe_reportar(&self) -> bool {
        !matches!(
            self.ambitos.last().map(|a| a.tipo),
            Some(TipoAmbito::Global)
        )
    }

    fn contexto_bloque(&self) -> String {
        for ambito in self.ambitos.iter().rev() {
            match ambito.tipo {
                TipoAmbito::Funcion => return format!("bloque dentro de {}", ambito.contexto),
                TipoAmbito::Global => break,
                TipoAmbito::Bloque => {}
            }
        }
        "bloque".to_string()
    }

    fn contexto_actual(&self) -> String {
        self.ambitos
            .last()
            .map(|a| a.contexto.clone())
            .unwrap_or_else(|| "global".to_string())
    }

    fn buscar(&self, nombre: &str) -> Option<usize> {
        for (indice, ambito) in self.ambitos.iter().enumerate().rev() {
            if ambito.variables.contains_key(nombre) {
                return Some(indice);
            }
        }
        None
    }

    fn usar(&mut self, nombre: &str) {
        if let Some(indice) = self.buscar(nombre) {
            if let Some(declaracion) = self.ambitos[indice].variables.get_mut(nombre) {
                declaracion.usada = true;
            }
        }
    }

    fn declarar_si_nuevo(&mut self, nombre: &str, reportar: bool) {
        if self.buscar(nombre).is_some() {
            return;
        }
        self.declarar_en_actual(nombre, reportar);
    }

    fn declarar_en_actual(&mut self, nombre: &str, reportar: bool) {
        let ignorar = nombre == "_" || nombre.starts_with('_');
        let contexto = self.contexto_actual();
        if let Some(ambito) = self.ambitos.last_mut() {
            ambito.variables.insert(
                nombre.to_string(),
                Declaracion {
                    usada: false,
                    reportar: reportar && !ignorar,
                    contexto,
                },
            );
        }
    }

    fn cerrar_ambito(&mut self) {
        let ambito = match self.ambitos.pop() {
            Some(ambito) => ambito,
            None => return,
        };
        if ambito.tipo == TipoAmbito::Global {
            return;
        }
        for (nombre, declaracion) in ambito.variables {
            if declaracion.reportar && !declaracion.usada {
                self.errores
                    .push(format!("variable '{}' ({})", nombre, declaracion.contexto));
            }
        }
    }
}

fn nombre_importado(modulo: &str) -> String {
    if modulo.ends_with(".vaca") || modulo.contains('/') || modulo.contains('\\') {
        std::path::Path::new(modulo)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(modulo)
            .to_string()
    } else {
        modulo.split('.').next_back().unwrap_or(modulo).to_string()
    }
}
