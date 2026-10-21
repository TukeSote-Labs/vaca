//! Compilador de funciones a bytecode.
//!
//! Convierte el AST de una función en una lista plana de instrucciones con las
//! variables resueltas a posiciones fijas del marco. Si la función usa alguna
//! construcción que la VM todavía no soporta, devuelve `None` y el intérprete
//! de árbol la ejecuta como siempre.

use std::collections::HashMap;

use crate::ast::{Expr, OpBinario, OpUnario, Stmt};
use crate::bytecode::{Chunk, CondLocal, DatosCondicion, DatosSiguiente, Op, Operando};
use crate::value::{Funcion, Value};

pub fn compilar(funcion: &Funcion) -> Option<Chunk> {
    if funcion.es_async {
        return None;
    }

    let mut compilador = Compilador::nuevo();

    for parametro in &funcion.parametros {
        let slot = compilador.declarar(&parametro.nombre)?;
        if let Some(defecto) = &parametro.defecto {
            let salto = compilador.emit(Op::SaltarSiDefinido {
                local: slot,
                destino: 0,
            });
            compilador.expr(defecto)?;
            compilador.emit(Op::GuardarLocal(slot));
            let destino = compilador.codigo.len();
            compilador.parchear_salto_si_definido(salto, destino);
        }
    }

    compilador.sentencias(&funcion.cuerpo)?;
    Some(compilador.fin())
}

struct Bucle {
    continuar: Option<u32>,
    continuar_pendientes: Vec<usize>,
    romper: Vec<usize>,
}

struct Compilador {
    codigo: Vec<Op>,
    constantes: Vec<Value>,
    condiciones: Vec<DatosCondicion>,
    siguientes: Vec<DatosSiguiente>,
    condicionales: Vec<CondLocal>,
    nombres: Vec<String>,
    indices_nombres: HashMap<String, u32>,
    nombres_locales: Vec<String>,
    ambitos: Vec<HashMap<String, u16>>,
    siguiente_local: u16,
    dinamicos: Vec<String>,
    bucles: Vec<Bucle>,
}

impl Compilador {
    fn nuevo() -> Self {
        Compilador {
            codigo: Vec::new(),
            constantes: Vec::new(),
            condiciones: Vec::new(),
            siguientes: Vec::new(),
            condicionales: Vec::new(),
            nombres: Vec::new(),
            indices_nombres: HashMap::new(),
            nombres_locales: Vec::new(),
            ambitos: vec![HashMap::new()],
            siguiente_local: 0,
            dinamicos: Vec::new(),
            bucles: Vec::new(),
        }
    }

    fn fin(self) -> Chunk {
        Chunk {
            codigo: self.codigo,
            constantes: self.constantes,
            condiciones: self.condiciones,
            siguientes: self.siguientes,
            condicionales: self.condicionales,
            nombres: self.nombres,
            nombres_locales: self.nombres_locales,
            max_locales: self.siguiente_local,
            dinamicos: self.dinamicos,
        }
    }

    // ------------------------------------------------------------------
    // Emisión
    // ------------------------------------------------------------------

    fn emit(&mut self, op: Op) -> usize {
        self.codigo.push(op);
        self.codigo.len() - 1
    }

    fn constante(&mut self, valor: Value) -> u32 {
        self.constantes.push(valor);
        (self.constantes.len() - 1) as u32
    }

    fn nombre(&mut self, nombre: &str) -> u32 {
        if let Some(id) = self.indices_nombres.get(nombre) {
            return *id;
        }
        let id = self.nombres.len() as u32;
        self.nombres.push(nombre.to_string());
        self.indices_nombres.insert(nombre.to_string(), id);
        id
    }

    fn parchear_salto(&mut self, indice: usize, destino: usize) {
        self.codigo[indice] = Op::Saltar(destino as u32);
    }

    fn parchear_salto_falso(&mut self, indice: usize, destino: usize) {
        match self.codigo[indice] {
            Op::SaltoCondicional(id) => {
                self.condicionales[id as usize].destino = destino as u32;
            }
            _ => {
                self.codigo[indice] = Op::SaltarSiFalso(destino as u32);
            }
        }
    }

    /// Emite el salto "si es falso" de una condición, fusionando comparaciones
    /// entre locales y literales en una sola instrucción.
    fn emitir_condicion_falso(&mut self, cond: &Expr) -> Option<usize> {
        if let Some(indice) = self.condicion_fusionada(cond) {
            return Some(indice);
        }
        self.expr(cond)?;
        Some(self.emit(Op::SaltarSiFalso(0)))
    }

    fn condicion_fusionada(&mut self, cond: &Expr) -> Option<usize> {
        let Expr::Binaria { op, izq, der } = cond else {
            return None;
        };
        let operador = match op {
            OpBinario::Igual => 0,
            OpBinario::Distinto => 1,
            OpBinario::Menor => 2,
            OpBinario::Mayor => 3,
            OpBinario::MenorIgual => 4,
            OpBinario::MayorIgual => 5,
            _ => return None,
        };
        let izquierda = self.operando(izq)?;
        let derecha = self.operando(der)?;
        let id = self.condicionales.len() as u32;
        self.condicionales.push(CondLocal {
            izquierda,
            derecha,
            operador,
            destino: 0,
        });
        Some(self.emit(Op::SaltoCondicional(id)))
    }

    fn operando(&mut self, expr: &Expr) -> Option<Operando> {
        match expr {
            Expr::Variable(nombre) => self.resolver(nombre).map(Operando::Local),
            Expr::Entero(n) => {
                let k = self.constante(Value::Entero(*n));
                Some(Operando::Constante(k))
            }
            Expr::Decimal(n) => {
                let k = self.constante(Value::Decimal(*n));
                Some(Operando::Constante(k))
            }
            Expr::Texto(texto) => {
                let k = self.constante(Value::Texto(texto.clone()));
                Some(Operando::Constante(k))
            }
            Expr::Logico(b) => {
                let k = self.constante(Value::Logico(*b));
                Some(Operando::Constante(k))
            }
            Expr::Nulo => {
                let k = self.constante(Value::Nulo);
                Some(Operando::Constante(k))
            }
            _ => None,
        }
    }

    fn parchear_salto_verdadero(&mut self, indice: usize, destino: usize) {
        self.codigo[indice] = Op::SaltarSiVerdadero(destino as u32);
    }

    fn parchear_salto_si_definido(&mut self, indice: usize, destino: usize) {
        if let Op::SaltarSiDefinido { local, .. } = self.codigo[indice] {
            self.codigo[indice] = Op::SaltarSiDefinido {
                local,
                destino: destino as u32,
            };
        }
    }

    // ------------------------------------------------------------------
    // Ámbitos y variables
    // ------------------------------------------------------------------

    fn nuevo_ambito(&mut self) {
        self.ambitos.push(HashMap::new());
    }

    fn cerrar_ambito(&mut self) {
        self.ambitos.pop();
    }

    fn declarar(&mut self, nombre: &str) -> Option<u16> {
        let slot = self.siguiente_local;
        self.siguiente_local = self.siguiente_local.checked_add(1)?;
        self.nombres_locales.push(nombre.to_string());
        self.ambitos
            .last_mut()
            .expect("siempre hay un ámbito")
            .insert(nombre.to_string(), slot);
        Some(slot)
    }

    fn declarar_anonimo(&mut self) -> Option<u16> {
        let slot = self.siguiente_local;
        self.siguiente_local = self.siguiente_local.checked_add(1)?;
        self.nombres_locales.push(String::new());
        Some(slot)
    }

    fn resolver(&self, nombre: &str) -> Option<u16> {
        for ambito in self.ambitos.iter().rev() {
            if let Some(slot) = ambito.get(nombre) {
                return Some(*slot);
            }
        }
        None
    }

    fn declarar_dinamica(&mut self, nombre: &str) -> Option<u16> {
        if !self.dinamicos.iter().any(|n| n == nombre) {
            self.dinamicos.push(nombre.to_string());
        }
        self.declarar(nombre)
    }

    // ------------------------------------------------------------------
    // Sentencias
    // ------------------------------------------------------------------

    fn sentencias(&mut self, sentencias: &[Stmt]) -> Option<()> {
        for sentencia in sentencias {
            self.sentencia(sentencia)?;
        }
        Some(())
    }

    fn sentencia(&mut self, stmt: &Stmt) -> Option<()> {
        match stmt {
            Stmt::Expresion(expr) => {
                self.expr(expr)?;
                self.emit(Op::Pop);
            }
            Stmt::Asignar { objetivo, valor } => match objetivo {
                Expr::Variable(nombre) => {
                    let slot = match self.resolver(nombre) {
                        Some(slot) => slot,
                        None => self.declarar_dinamica(nombre)?,
                    };
                    if let Expr::Binaria { op, izq, der } = valor {
                        if let Expr::Variable(origen) = izq.as_ref() {
                            if origen == nombre {
                                if let Expr::Entero(k) = der.as_ref() {
                                    let k = self.constante(Value::Entero(*k));
                                    match op {
                                        OpBinario::Suma => {
                                            self.emit(Op::SumarLocalConst {
                                                local: slot,
                                                constante: k,
                                            });
                                            return Some(());
                                        }
                                        OpBinario::Resta => {
                                            self.emit(Op::RestarLocalConst {
                                                local: slot,
                                                constante: k,
                                            });
                                            return Some(());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                    self.expr(valor)?;
                    self.emit(Op::GuardarLocal(slot));
                }
                Expr::Miembro { objeto, nombre } => {
                    self.expr(valor)?;
                    self.expr(objeto)?;
                    let id = self.nombre(nombre);
                    self.emit(Op::GuardarMiembro(id));
                }
                Expr::Indice { objeto, indice } => {
                    self.expr(valor)?;
                    self.expr(objeto)?;
                    self.expr(indice)?;
                    self.emit(Op::GuardarIndice);
                }
                _ => return None,
            },
            Stmt::Si {
                condicion,
                entonces,
                sino_si,
                sino,
            } => self.si(condicion, entonces, sino_si, sino)?,
            Stmt::Mientras { condicion, cuerpo } => self.mientras(condicion, cuerpo)?,
            Stmt::ParaRango {
                variable,
                inicio,
                fin,
                paso,
                cuerpo,
            } => self.para_rango(variable, inicio, fin, paso.as_ref(), cuerpo)?,
            Stmt::ParaCada {
                variable,
                iterable,
                cuerpo,
            } => self.para_cada(variable, iterable, cuerpo)?,
            Stmt::Retornar(expr) => {
                match expr {
                    Some(expr) => self.expr(expr)?,
                    None => {
                        self.emit(Op::Nulo);
                    }
                }
                self.emit(Op::Retornar);
            }
            Stmt::Romper => {
                let salto = self.emit(Op::Saltar(0));
                self.bucles.last_mut()?.romper.push(salto);
            }
            Stmt::Continuar => {
                let continuar = self.bucles.last()?.continuar;
                match continuar {
                    Some(destino) => {
                        self.emit(Op::Saltar(destino));
                    }
                    None => {
                        let salto = self.emit(Op::Saltar(0));
                        self.bucles
                            .last_mut()
                            .expect("hay un bucle")
                            .continuar_pendientes
                            .push(salto);
                    }
                }
            }
            Stmt::Lanzar(expr) => {
                self.expr(expr)?;
                self.emit(Op::Lanzar);
            }
            Stmt::Bloque(sentencias) => {
                self.nuevo_ambito();
                self.sentencias(sentencias)?;
                self.cerrar_ambito();
            }
            // Constantes, funciones y clases anidadas, y try/catch requieren un
            // entorno real; se ejecutan con el intérprete de árbol.
            Stmt::Constante { .. }
            | Stmt::Funcion { .. }
            | Stmt::Clase { .. }
            | Stmt::Intentar { .. }
            | Stmt::Importar { .. } => return None,
        }
        Some(())
    }

    fn si(
        &mut self,
        condicion: &Expr,
        entonces: &[Stmt],
        sino_si: &[(Expr, Vec<Stmt>)],
        sino: &Option<Vec<Stmt>>,
    ) -> Option<()> {
        let mut saltos_fin = Vec::new();
        let mut saltos_siguiente = Vec::new();

        saltos_siguiente.push(self.emitir_condicion_falso(condicion)?);

        self.nuevo_ambito();
        self.sentencias(entonces)?;
        self.cerrar_ambito();

        for (condicion, cuerpo) in sino_si {
            saltos_fin.push(self.emit(Op::Saltar(0)));
            let destino = self.codigo.len();
            if let Some(salto) = saltos_siguiente.pop() {
                self.parchear_salto_falso(salto, destino);
            }
            saltos_siguiente.push(self.emitir_condicion_falso(condicion)?);
            self.nuevo_ambito();
            self.sentencias(cuerpo)?;
            self.cerrar_ambito();
        }

        if let Some(cuerpo) = sino {
            saltos_fin.push(self.emit(Op::Saltar(0)));
            let destino = self.codigo.len();
            if let Some(salto) = saltos_siguiente.pop() {
                self.parchear_salto_falso(salto, destino);
            }
            self.nuevo_ambito();
            self.sentencias(cuerpo)?;
            self.cerrar_ambito();
        }

        let fin = self.codigo.len();
        if let Some(salto) = saltos_siguiente.pop() {
            self.parchear_salto_falso(salto, fin);
        }
        for salto in saltos_fin {
            self.parchear_salto(salto, fin);
        }
        Some(())
    }

    fn mientras(&mut self, condicion: &Expr, cuerpo: &[Stmt]) -> Option<()> {
        let inicio = self.codigo.len();
        let salida = self.emitir_condicion_falso(condicion)?;

        self.bucles.push(Bucle {
            continuar: Some(inicio as u32),
            continuar_pendientes: Vec::new(),
            romper: Vec::new(),
        });
        self.nuevo_ambito();
        self.sentencias(cuerpo)?;
        self.cerrar_ambito();
        let bucle = self.bucles.pop()?;

        self.emit(Op::Saltar(inicio as u32));
        let fin = self.codigo.len();
        self.parchear_salto_falso(salida, fin);
        for salto in bucle.romper {
            self.parchear_salto(salto, fin);
        }
        Some(())
    }

    fn para_rango(
        &mut self,
        variable: &str,
        inicio: &Expr,
        fin: &Expr,
        paso: Option<&Expr>,
        cuerpo: &[Stmt],
    ) -> Option<()> {
        // Los límites se evalúan en el ámbito externo, como en el intérprete.
        self.expr(inicio)?;
        self.emit(Op::AEntero);
        let inicio_tmp = self.declarar_anonimo()?;
        self.emit(Op::GuardarLocal(inicio_tmp));

        self.expr(fin)?;
        self.emit(Op::AEntero);
        let fin_slot = self.declarar_anonimo()?;
        self.emit(Op::GuardarLocal(fin_slot));

        match paso {
            Some(expr) => self.expr(expr)?,
            None => {
                let uno = self.constante(Value::Entero(1));
                self.emit(Op::Constante(uno));
            }
        }
        self.emit(Op::AEntero);
        let paso_slot = self.declarar_anonimo()?;
        self.emit(Op::GuardarLocal(paso_slot));
        self.emit(Op::VerificarPaso(paso_slot));

        self.nuevo_ambito();
        let variable_slot = self.declarar(variable)?;
        self.emit(Op::CargarLocal(inicio_tmp));
        self.emit(Op::GuardarLocal(variable_slot));

        let inicio_bucle = self.codigo.len();
        let condicion_id = self.condiciones.len() as u32;
        self.condiciones.push(DatosCondicion {
            variable: variable_slot,
            fin: fin_slot,
            paso: paso_slot,
            destino: 0,
        });
        self.emit(Op::ParaCondicion(condicion_id));

        self.bucles.push(Bucle {
            continuar: None,
            continuar_pendientes: Vec::new(),
            romper: Vec::new(),
        });
        self.nuevo_ambito();
        self.sentencias(cuerpo)?;
        self.cerrar_ambito();
        let bucle = self.bucles.pop()?;

        let continuar_ip = self.codigo.len();
        let siguiente_id = self.siguientes.len() as u32;
        self.siguientes.push(DatosSiguiente {
            variable: variable_slot,
            paso: paso_slot,
            inicio: inicio_bucle as u32,
            fin: 0,
        });
        self.emit(Op::ParaSiguiente(siguiente_id));

        let fin_bucle = self.codigo.len();
        self.condiciones[condicion_id as usize].destino = fin_bucle as u32;
        self.siguientes[siguiente_id as usize].fin = fin_bucle as u32;
        for salto in bucle.continuar_pendientes {
            self.parchear_salto(salto, continuar_ip);
        }
        for salto in bucle.romper {
            self.parchear_salto(salto, fin_bucle);
        }

        self.cerrar_ambito();
        Some(())
    }

    fn para_cada(&mut self, variable: &str, iterable: &Expr, cuerpo: &[Stmt]) -> Option<()> {
        self.expr(iterable)?;
        self.emit(Op::Iterable);
        let lista_tmp = self.declarar_anonimo()?;
        self.emit(Op::GuardarLocal(lista_tmp));

        self.nuevo_ambito();
        let variable_slot = self.declarar(variable)?;
        let indice = self.declarar_anonimo()?;
        let cero = self.constante(Value::Entero(0));
        self.emit(Op::Constante(cero));
        self.emit(Op::GuardarLocal(indice));

        let inicio_bucle = self.codigo.len();
        self.emit(Op::CargarLocal(indice));
        self.emit(Op::CargarLocal(lista_tmp));
        self.emit(Op::Longitud);
        self.emit(Op::Menor);
        let salida = self.emit(Op::SaltarSiFalso(0));

        self.emit(Op::CargarLocal(lista_tmp));
        self.emit(Op::CargarLocal(indice));
        self.emit(Op::CargarIndice);
        self.emit(Op::GuardarLocal(variable_slot));

        self.bucles.push(Bucle {
            continuar: None,
            continuar_pendientes: Vec::new(),
            romper: Vec::new(),
        });
        self.nuevo_ambito();
        self.sentencias(cuerpo)?;
        self.cerrar_ambito();
        let bucle = self.bucles.pop()?;

        let continuar_ip = self.codigo.len();
        self.emit(Op::CargarLocal(indice));
        let uno = self.constante(Value::Entero(1));
        self.emit(Op::Constante(uno));
        self.emit(Op::Sumar);
        self.emit(Op::GuardarLocal(indice));
        self.emit(Op::Saltar(inicio_bucle as u32));

        let fin_bucle = self.codigo.len();
        self.parchear_salto_falso(salida, fin_bucle);
        for salto in bucle.continuar_pendientes {
            self.parchear_salto(salto, continuar_ip);
        }
        for salto in bucle.romper {
            self.parchear_salto(salto, fin_bucle);
        }

        self.cerrar_ambito();
        Some(())
    }

    // ------------------------------------------------------------------
    // Expresiones
    // ------------------------------------------------------------------

    fn expr(&mut self, expr: &Expr) -> Option<()> {
        match expr {
            Expr::Entero(n) => {
                let k = self.constante(Value::Entero(*n));
                self.emit(Op::Constante(k));
            }
            Expr::Decimal(n) => {
                let k = self.constante(Value::Decimal(*n));
                self.emit(Op::Constante(k));
            }
            Expr::Texto(t) => {
                let k = self.constante(Value::Texto(t.clone()));
                self.emit(Op::Constante(k));
            }
            Expr::Logico(b) => {
                self.emit(if *b { Op::Verdadero } else { Op::Falso });
            }
            Expr::Nulo => {
                self.emit(Op::Nulo);
            }
            Expr::Interpolado(partes) => {
                for parte in partes {
                    self.expr(parte)?;
                }
                let n = partes.len();
                self.emit(Op::Interpolar(u16::try_from(n).ok()?));
            }
            Expr::Variable(nombre) => match self.resolver(nombre) {
                Some(slot) => {
                    self.emit(Op::CargarLocal(slot));
                }
                None => {
                    let id = self.nombre(nombre);
                    self.emit(Op::CargarExterna(id));
                }
            },
            Expr::Este => match self.resolver("este") {
                Some(slot) => {
                    self.emit(Op::CargarLocal(slot));
                }
                None => {
                    let id = self.nombre("este");
                    self.emit(Op::CargarExterna(id));
                }
            },
            Expr::Lista(elementos) => {
                for elemento in elementos {
                    self.expr(elemento)?;
                }
                let n = elementos.len();
                self.emit(Op::Lista(u16::try_from(n).ok()?));
            }
            Expr::Diccionario(pares) => {
                for (clave, valor) in pares {
                    let k = self.constante(Value::Texto(clave.clone()));
                    self.emit(Op::Constante(k));
                    self.expr(valor)?;
                }
                let n = pares.len();
                self.emit(Op::Diccionario(u16::try_from(n).ok()?));
            }
            Expr::Binaria { op, izq, der } => self.binaria(*op, izq, der)?,
            Expr::Unaria { op, expr } => {
                self.expr(expr)?;
                self.emit(match op {
                    OpUnario::No => Op::No,
                    OpUnario::Negativo => Op::Negativo,
                    OpUnario::Positivo => Op::Positivo,
                });
            }
            Expr::Llamada { callee, args } => {
                if let Expr::Miembro { objeto, nombre } = callee.as_ref() {
                    self.expr(objeto)?;
                    for arg in args {
                        self.expr(arg)?;
                    }
                    let id = self.nombre(nombre);
                    self.emit(Op::LlamarMetodo {
                        nombre: id,
                        args: u16::try_from(args.len()).ok()?,
                    });
                } else {
                    self.expr(callee)?;
                    for arg in args {
                        self.expr(arg)?;
                    }
                    self.emit(Op::Llamar(u16::try_from(args.len()).ok()?));
                }
            }
            Expr::Miembro { objeto, nombre } => {
                self.expr(objeto)?;
                let id = self.nombre(nombre);
                self.emit(Op::CargarMiembro(id));
            }
            Expr::Indice { objeto, indice } => {
                self.expr(objeto)?;
                self.expr(indice)?;
                self.emit(Op::CargarIndice);
            }
            Expr::Nuevo { clase, args } => {
                for arg in args {
                    self.expr(arg)?;
                }
                let id = self.nombre(clase);
                self.emit(Op::Nuevo {
                    clase: id,
                    args: u16::try_from(args.len()).ok()?,
                });
            }
            // Los cierres necesitan materializar el marco como entorno.
            Expr::Funcion { .. } | Expr::Esperar(_) => return None,
        }
        Some(())
    }

    fn binaria(&mut self, op: OpBinario, izq: &Expr, der: &Expr) -> Option<()> {
        match op {
            OpBinario::Y => {
                self.expr(izq)?;
                let falso = self.emit(Op::SaltarSiFalso(0));
                self.expr(der)?;
                let falso_der = self.emit(Op::SaltarSiFalso(0));
                self.emit(Op::Verdadero);
                let fin = self.emit(Op::Saltar(0));
                let destino_falso = self.codigo.len();
                self.parchear_salto_falso(falso, destino_falso);
                self.parchear_salto_falso(falso_der, destino_falso);
                self.emit(Op::Falso);
                let fin_real = self.codigo.len();
                self.parchear_salto(fin, fin_real);
            }
            OpBinario::O => {
                self.expr(izq)?;
                let verdadero = self.emit(Op::SaltarSiVerdadero(0));
                self.expr(der)?;
                let verdadero_der = self.emit(Op::SaltarSiVerdadero(0));
                self.emit(Op::Falso);
                let fin = self.emit(Op::Saltar(0));
                let destino_verdadero = self.codigo.len();
                self.parchear_salto_verdadero(verdadero, destino_verdadero);
                self.parchear_salto_verdadero(verdadero_der, destino_verdadero);
                self.emit(Op::Verdadero);
                let fin_real = self.codigo.len();
                self.parchear_salto(fin, fin_real);
            }
            _ => {
                self.expr(izq)?;
                self.expr(der)?;
                self.emit(match op {
                    OpBinario::Suma => Op::Sumar,
                    OpBinario::Resta => Op::Restar,
                    OpBinario::Multiplicacion => Op::Multiplicar,
                    OpBinario::Division => Op::Dividir,
                    OpBinario::Modulo => Op::Modulo,
                    OpBinario::Igual => Op::Igual,
                    OpBinario::Distinto => Op::Distinto,
                    OpBinario::Menor => Op::Menor,
                    OpBinario::Mayor => Op::Mayor,
                    OpBinario::MenorIgual => Op::MenorIgual,
                    OpBinario::MayorIgual => Op::MayorIgual,
                    OpBinario::Y | OpBinario::O => unreachable!(),
                });
            }
        }
        Some(())
    }
}
