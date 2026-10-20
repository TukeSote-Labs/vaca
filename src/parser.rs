//! Analizador sintáctico: tokens -> AST.

use crate::ast::{Expr, MetodoClase, OpBinario, OpUnario, Parametro, Stmt};
use crate::error::Error;
use crate::lexer::{desescapar, Lexer};
use crate::token::{Token, TokenTipo};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn nuevo(mut tokens: Vec<Token>) -> Self {
        if !matches!(tokens.last().map(|t| &t.tipo), Some(TokenTipo::Fin)) {
            tokens.push(Token::nuevo(TokenTipo::Fin, 1, 1));
        }
        Parser { tokens, pos: 0 }
    }

    pub fn desde_codigo(fuente: &str) -> Result<Self, Error> {
        let tokens = Lexer::nuevo(fuente).tokenizar()?;
        Ok(Parser::nuevo(tokens))
    }

    pub fn parsear_programa(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut sentencias = Vec::new();
        while !self.es_fin() {
            if self.coincidir(&TokenTipo::PuntoYComa) {
                continue;
            }
            sentencias.push(self.declaracion()?);
            while self.coincidir(&TokenTipo::PuntoYComa) {}
        }
        Ok(sentencias)
    }

    pub fn parsear_expresion_completa(&mut self) -> Result<Expr, Error> {
        let expr = self.expresion()?;
        if !self.es_fin() {
            return Err(self.error_actual(format!(
                "se esperaba el final de la expresión, pero se encontró {}",
                self.actual().tipo.descripcion()
            )));
        }
        Ok(expr)
    }

    // ------------------------------------------------------------------
    // Utilidades
    // ------------------------------------------------------------------

    fn actual(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn siguiente_tipo(&self) -> Option<&TokenTipo> {
        self.tokens.get(self.pos + 1).map(|t| &t.tipo)
    }

    fn es(&self, tipo: &TokenTipo) -> bool {
        &self.actual().tipo == tipo
    }

    fn es_fin(&self) -> bool {
        matches!(self.actual().tipo, TokenTipo::Fin)
    }

    fn avanzar(&mut self) -> Token {
        let token = self.actual().clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        token
    }

    fn coincidir(&mut self, tipo: &TokenTipo) -> bool {
        if self.es(tipo) {
            self.avanzar();
            true
        } else {
            false
        }
    }

    fn esperar(&mut self, tipo: &TokenTipo, mensaje: &str) -> Result<Token, Error> {
        if self.es(tipo) {
            Ok(self.avanzar())
        } else {
            Err(self.error_actual(format!(
                "{}, se encontró {}",
                mensaje,
                self.actual().tipo.descripcion()
            )))
        }
    }

    fn esperar_identificador(&mut self, mensaje: &str) -> Result<String, Error> {
        match self.actual().tipo.clone() {
            TokenTipo::Identificador(nombre) => {
                self.avanzar();
                Ok(nombre)
            }
            _ => Err(self.error_actual(format!(
                "{}, se encontró {}",
                mensaje,
                self.actual().tipo.descripcion()
            ))),
        }
    }

    /// Después de '.' se permite cualquier palabra, incluso palabras clave,
    /// para poder acceder a campos como `peticion.metodo`.
    fn esperar_nombre_miembro(&mut self, mensaje: &str) -> Result<String, Error> {
        let tipo = self.actual().tipo.clone();
        match tipo {
            TokenTipo::Identificador(nombre) => {
                self.avanzar();
                Ok(nombre)
            }
            palabra if palabra.es_palabra() => {
                self.avanzar();
                Ok(palabra.simbolo().to_string())
            }
            _ => Err(self.error_actual(format!(
                "{}, se encontró {}",
                mensaje,
                self.actual().tipo.descripcion()
            ))),
        }
    }

    fn error_actual(&self, mensaje: impl Into<String>) -> Error {
        Error::sintactico(mensaje, self.actual().linea, self.actual().columna)
    }

    fn error_en(&self, token: &Token, mensaje: impl Into<String>) -> Error {
        Error::sintactico(mensaje, token.linea, token.columna)
    }

    // ------------------------------------------------------------------
    // Sentencias
    // ------------------------------------------------------------------

    fn declaracion(&mut self) -> Result<Stmt, Error> {
        match self.actual().tipo.clone() {
            TokenTipo::Si => self.declaracion_si(),
            TokenTipo::Mientras => self.declaracion_mientras(),
            TokenTipo::Para => self.declaracion_para(),
            TokenTipo::Fn | TokenTipo::Funcion => self.declaracion_funcion(false),
            TokenTipo::Metodo => self.declaracion_funcion(false),
            TokenTipo::Async => {
                self.avanzar();
                if !matches!(
                    self.actual().tipo,
                    TokenTipo::Fn | TokenTipo::Funcion | TokenTipo::Metodo
                ) {
                    return Err(
                        self.error_actual("después de 'async' se esperaba una función ('fn')")
                    );
                }
                self.declaracion_funcion(true)
            }
            TokenTipo::Clase => self.declaracion_clase(),
            TokenTipo::Retornar | TokenTipo::Devolver => {
                self.avanzar();
                let valor = if self.es_fin()
                    || self.es(&TokenTipo::LlaveDer)
                    || self.es(&TokenTipo::PuntoYComa)
                {
                    None
                } else {
                    Some(self.expresion()?)
                };
                Ok(Stmt::Retornar(valor))
            }
            TokenTipo::Romper => {
                self.avanzar();
                Ok(Stmt::Romper)
            }
            TokenTipo::Continuar => {
                self.avanzar();
                Ok(Stmt::Continuar)
            }
            TokenTipo::Lanzar => {
                self.avanzar();
                let valor = self.expresion()?;
                Ok(Stmt::Lanzar(valor))
            }
            TokenTipo::Intentar => {
                self.avanzar();
                let cuerpo = self.bloque()?;
                self.esperar(
                    &TokenTipo::Atrapar,
                    "se esperaba 'atrapar' después del bloque de 'intentar'",
                )?;
                let variable_error = match self.actual().tipo.clone() {
                    TokenTipo::Identificador(nombre) => {
                        self.avanzar();
                        Some(nombre)
                    }
                    _ => None,
                };
                let atrapar = self.bloque()?;
                Ok(Stmt::Intentar {
                    cuerpo,
                    variable_error,
                    atrapar,
                })
            }
            TokenTipo::Importar => {
                self.avanzar();
                let modulo = match self.actual().tipo.clone() {
                    TokenTipo::Texto(texto) => {
                        self.avanzar();
                        texto
                    }
                    TokenTipo::Identificador(primero) => {
                        self.avanzar();
                        let mut nombre = primero;
                        while self.coincidir(&TokenTipo::Punto) {
                            nombre.push('.');
                            nombre.push_str(&self.esperar_identificador(
                                "se esperaba un nombre después de '.'",
                            )?);
                        }
                        nombre
                    }
                    _ => {
                        return Err(self.error_actual(
                            "después de 'importar' se esperaba el nombre de un módulo o una ruta entre comillas",
                        ))
                    }
                };
                let alias = if self.coincidir(&TokenTipo::Como) {
                    Some(self.esperar_identificador("se esperaba un alias después de 'como'")?)
                } else {
                    None
                };
                Ok(Stmt::Importar { modulo, alias })
            }
            TokenTipo::Constante => {
                self.avanzar();
                let nombre = self.esperar_identificador("se esperaba el nombre de la constante")?;
                self.esperar(
                    &TokenTipo::Igual,
                    "se esperaba '=' después del nombre de la constante",
                )?;
                let valor = self.expresion()?;
                Ok(Stmt::Constante { nombre, valor })
            }
            TokenTipo::LlaveIzq => {
                let cuerpo = self.bloque()?;
                Ok(Stmt::Bloque(cuerpo))
            }
            _ => self.declaracion_simple(),
        }
    }

    fn declaracion_simple(&mut self) -> Result<Stmt, Error> {
        // Declaración con anotación de tipo: nombre: Tipo = valor
        if matches!(self.actual().tipo, TokenTipo::Identificador(_))
            && matches!(self.siguiente_tipo(), Some(TokenTipo::DosPuntos))
        {
            let nombre = self.esperar_identificador("se esperaba un nombre de variable")?;
            self.avanzar(); // ':'
            let _tipo = self.esperar_identificador("se esperaba un nombre de tipo")?;
            let valor = if self.coincidir(&TokenTipo::Igual) {
                self.expresion()?
            } else {
                Expr::Nulo
            };
            return Ok(Stmt::Asignar {
                objetivo: Expr::Variable(nombre),
                valor,
            });
        }

        let objetivo = self.expresion()?;

        let compuesto = if self.coincidir(&TokenTipo::MasIgual) {
            Some(OpBinario::Suma)
        } else if self.coincidir(&TokenTipo::MenosIgual) {
            Some(OpBinario::Resta)
        } else if self.coincidir(&TokenTipo::AsteriscoIgual) {
            Some(OpBinario::Multiplicacion)
        } else if self.coincidir(&TokenTipo::DiagonalIgual) {
            Some(OpBinario::Division)
        } else {
            None
        };

        if let Some(op) = compuesto {
            let valor = self.expresion()?;
            if !es_objetivo_valido(&objetivo) {
                return Err(self.error_actual(
                    "el lado izquierdo de una asignación debe ser una variable, un campo o un índice",
                ));
            }
            return Ok(Stmt::Asignar {
                objetivo: objetivo.clone(),
                valor: Expr::Binaria {
                    op,
                    izq: Box::new(objetivo),
                    der: Box::new(valor),
                },
            });
        }

        if self.coincidir(&TokenTipo::Igual) {
            let valor = self.expresion()?;
            if !es_objetivo_valido(&objetivo) {
                return Err(self.error_actual(
                    "el lado izquierdo de una asignación debe ser una variable, un campo o un índice",
                ));
            }
            return Ok(Stmt::Asignar { objetivo, valor });
        }

        Ok(Stmt::Expresion(objetivo))
    }

    fn declaracion_si(&mut self) -> Result<Stmt, Error> {
        self.esperar(&TokenTipo::Si, "se esperaba 'si'")?;
        let condicion = self.expresion()?;
        let entonces = self.bloque()?;
        let mut sino_si = Vec::new();
        let mut sino = None;
        while self.coincidir(&TokenTipo::Sino) {
            if self.coincidir(&TokenTipo::Si) {
                let cond = self.expresion()?;
                let cuerpo = self.bloque()?;
                sino_si.push((cond, cuerpo));
            } else {
                sino = Some(self.bloque()?);
                break;
            }
        }
        Ok(Stmt::Si {
            condicion,
            entonces,
            sino_si,
            sino,
        })
    }

    fn declaracion_mientras(&mut self) -> Result<Stmt, Error> {
        self.esperar(&TokenTipo::Mientras, "se esperaba 'mientras'")?;
        let condicion = self.expresion()?;
        let cuerpo = self.bloque()?;
        Ok(Stmt::Mientras { condicion, cuerpo })
    }

    fn declaracion_para(&mut self) -> Result<Stmt, Error> {
        self.esperar(&TokenTipo::Para, "se esperaba 'para'")?;
        let variable =
            self.esperar_identificador("se esperaba el nombre de la variable del bucle")?;
        if self.coincidir(&TokenTipo::En) {
            let iterable = self.expresion()?;
            let cuerpo = self.bloque()?;
            return Ok(Stmt::ParaCada {
                variable,
                iterable,
                cuerpo,
            });
        }
        self.esperar(
            &TokenTipo::Igual,
            "se esperaba '=' o 'en' después de la variable del bucle",
        )?;
        let inicio = self.expresion()?;
        self.esperar(&TokenTipo::Hasta, "se esperaba 'hasta' en el bucle 'para'")?;
        let fin = self.expresion()?;
        let paso = if self.coincidir(&TokenTipo::Paso) {
            Some(self.expresion()?)
        } else {
            None
        };
        let cuerpo = self.bloque()?;
        Ok(Stmt::ParaRango {
            variable,
            inicio,
            fin,
            paso,
            cuerpo,
        })
    }

    fn declaracion_funcion(&mut self, es_async: bool) -> Result<Stmt, Error> {
        if matches!(
            self.actual().tipo,
            TokenTipo::Fn | TokenTipo::Funcion | TokenTipo::Metodo
        ) {
            self.avanzar();
        } else {
            return Err(self.error_actual("se esperaba 'fn' para declarar una función"));
        }
        let nombre = self.esperar_identificador("se esperaba el nombre de la función")?;
        let parametros = self.parametros()?;
        let cuerpo = self.bloque()?;
        Ok(Stmt::Funcion {
            nombre,
            parametros,
            cuerpo,
            es_async,
        })
    }

    fn declaracion_clase(&mut self) -> Result<Stmt, Error> {
        self.esperar(&TokenTipo::Clase, "se esperaba 'clase'")?;
        let nombre = self.esperar_identificador("se esperaba el nombre de la clase")?;
        let padre = if self.coincidir(&TokenTipo::ParentesisIzq) {
            let p = self.esperar_identificador("se esperaba el nombre de la clase padre")?;
            self.esperar(
                &TokenTipo::ParentesisDer,
                "se esperaba ')' después del nombre de la clase padre",
            )?;
            Some(p)
        } else {
            None
        };
        self.esperar(
            &TokenTipo::LlaveIzq,
            "se esperaba '{' para iniciar el cuerpo de la clase",
        )?;
        let mut metodos = Vec::new();
        loop {
            while self.coincidir(&TokenTipo::PuntoYComa) {}
            if self.coincidir(&TokenTipo::LlaveDer) {
                break;
            }
            if self.es_fin() {
                return Err(self.error_actual("clase sin cerrar (falta '}')"));
            }
            let es_async = if self.coincidir(&TokenTipo::Async) {
                if !matches!(
                    self.actual().tipo,
                    TokenTipo::Fn | TokenTipo::Funcion | TokenTipo::Metodo
                ) {
                    return Err(
                        self.error_actual("después de 'async' se esperaba una función ('fn')")
                    );
                }
                true
            } else {
                false
            };
            if !matches!(
                self.actual().tipo,
                TokenTipo::Fn | TokenTipo::Funcion | TokenTipo::Metodo
            ) {
                return Err(self.error_actual(
                    "dentro de una clase solo se permiten métodos ('fn' o 'metodo')",
                ));
            }
            self.avanzar();
            let nombre_metodo = self.esperar_identificador("se esperaba el nombre del método")?;
            let parametros = self.parametros()?;
            let cuerpo = self.bloque()?;
            metodos.push(MetodoClase {
                nombre: nombre_metodo,
                parametros,
                cuerpo,
                es_async,
            });
        }
        Ok(Stmt::Clase {
            nombre,
            padre,
            metodos,
        })
    }

    fn parametros(&mut self) -> Result<Vec<Parametro>, Error> {
        self.esperar(
            &TokenTipo::ParentesisIzq,
            "se esperaba '(' para iniciar los parámetros",
        )?;
        let mut parametros = Vec::new();
        if !self.es(&TokenTipo::ParentesisDer) {
            loop {
                let nombre = self.esperar_identificador("se esperaba el nombre de un parámetro")?;
                let defecto = if self.coincidir(&TokenTipo::Igual) {
                    Some(self.expresion()?)
                } else {
                    None
                };
                parametros.push(Parametro { nombre, defecto });
                if !self.coincidir(&TokenTipo::Coma) {
                    break;
                }
            }
        }
        self.esperar(
            &TokenTipo::ParentesisDer,
            "se esperaba ')' al final de los parámetros",
        )?;
        Ok(parametros)
    }

    fn bloque(&mut self) -> Result<Vec<Stmt>, Error> {
        self.esperar(
            &TokenTipo::LlaveIzq,
            "se esperaba '{' para iniciar un bloque",
        )?;
        let mut sentencias = Vec::new();
        loop {
            while self.coincidir(&TokenTipo::PuntoYComa) {}
            if self.coincidir(&TokenTipo::LlaveDer) {
                break;
            }
            if self.es_fin() {
                return Err(self.error_actual("bloque sin cerrar (falta '}')"));
            }
            sentencias.push(self.declaracion()?);
        }
        Ok(sentencias)
    }

    // ------------------------------------------------------------------
    // Expresiones
    // ------------------------------------------------------------------

    pub fn expresion(&mut self) -> Result<Expr, Error> {
        self.o()
    }

    fn o(&mut self) -> Result<Expr, Error> {
        let mut expr = self.y()?;
        while self.coincidir(&TokenTipo::O) {
            let der = self.y()?;
            expr = Expr::Binaria {
                op: OpBinario::O,
                izq: Box::new(expr),
                der: Box::new(der),
            };
        }
        Ok(expr)
    }

    fn y(&mut self) -> Result<Expr, Error> {
        let mut expr = self.negacion()?;
        while self.coincidir(&TokenTipo::Y) {
            let der = self.negacion()?;
            expr = Expr::Binaria {
                op: OpBinario::Y,
                izq: Box::new(expr),
                der: Box::new(der),
            };
        }
        Ok(expr)
    }

    /// `no` se aplica a una comparación completa: `no a == b` equivale a
    /// `no (a == b)`, pero se combina con `y`/`o` como un operando más.
    fn negacion(&mut self) -> Result<Expr, Error> {
        if self.coincidir(&TokenTipo::No) {
            let expr = self.negacion()?;
            return Ok(Expr::Unaria {
                op: OpUnario::No,
                expr: Box::new(expr),
            });
        }
        self.igualdad()
    }

    fn igualdad(&mut self) -> Result<Expr, Error> {
        let mut expr = self.comparacion()?;
        loop {
            let op = if self.coincidir(&TokenTipo::IgualIgual) {
                OpBinario::Igual
            } else if self.coincidir(&TokenTipo::Distinto) {
                OpBinario::Distinto
            } else {
                break;
            };
            let der = self.comparacion()?;
            expr = Expr::Binaria {
                op,
                izq: Box::new(expr),
                der: Box::new(der),
            };
        }
        Ok(expr)
    }

    fn comparacion(&mut self) -> Result<Expr, Error> {
        let mut expr = self.suma()?;
        loop {
            let op = if self.coincidir(&TokenTipo::Menor) {
                OpBinario::Menor
            } else if self.coincidir(&TokenTipo::Mayor) {
                OpBinario::Mayor
            } else if self.coincidir(&TokenTipo::MenorIgual) {
                OpBinario::MenorIgual
            } else if self.coincidir(&TokenTipo::MayorIgual) {
                OpBinario::MayorIgual
            } else {
                break;
            };
            let der = self.suma()?;
            expr = Expr::Binaria {
                op,
                izq: Box::new(expr),
                der: Box::new(der),
            };
        }
        Ok(expr)
    }

    fn suma(&mut self) -> Result<Expr, Error> {
        let mut expr = self.multiplicacion()?;
        loop {
            let op = if self.coincidir(&TokenTipo::Mas) {
                OpBinario::Suma
            } else if self.coincidir(&TokenTipo::Menos) {
                OpBinario::Resta
            } else {
                break;
            };
            let der = self.multiplicacion()?;
            expr = Expr::Binaria {
                op,
                izq: Box::new(expr),
                der: Box::new(der),
            };
        }
        Ok(expr)
    }

    fn multiplicacion(&mut self) -> Result<Expr, Error> {
        let mut expr = self.unario()?;
        loop {
            let op = if self.coincidir(&TokenTipo::Asterisco) {
                OpBinario::Multiplicacion
            } else if self.coincidir(&TokenTipo::Diagonal) {
                OpBinario::Division
            } else if self.coincidir(&TokenTipo::Porcentaje) {
                OpBinario::Modulo
            } else {
                break;
            };
            let der = self.unario()?;
            expr = Expr::Binaria {
                op,
                izq: Box::new(expr),
                der: Box::new(der),
            };
        }
        Ok(expr)
    }

    fn unario(&mut self) -> Result<Expr, Error> {
        if self.coincidir(&TokenTipo::Menos) {
            let expr = self.unario()?;
            return Ok(Expr::Unaria {
                op: OpUnario::Negativo,
                expr: Box::new(expr),
            });
        }
        if self.coincidir(&TokenTipo::Mas) {
            let expr = self.unario()?;
            return Ok(Expr::Unaria {
                op: OpUnario::Positivo,
                expr: Box::new(expr),
            });
        }
        if self.coincidir(&TokenTipo::Esperar) {
            let expr = self.unario()?;
            return Ok(Expr::Esperar(Box::new(expr)));
        }
        self.postfijo()
    }

    fn postfijo(&mut self) -> Result<Expr, Error> {
        let mut expr = self.primario()?;
        loop {
            if self.coincidir(&TokenTipo::ParentesisIzq) {
                let args = self.argumentos()?;
                expr = Expr::Llamada {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.coincidir(&TokenTipo::Punto) {
                let nombre = self.esperar_nombre_miembro(
                    "se esperaba el nombre de un campo o método después de '.'",
                )?;
                expr = Expr::Miembro {
                    objeto: Box::new(expr),
                    nombre,
                };
            } else if self.coincidir(&TokenTipo::CorcheteIzq) {
                let indice = self.expresion()?;
                self.esperar(
                    &TokenTipo::CorcheteDer,
                    "se esperaba ']' después del índice",
                )?;
                expr = Expr::Indice {
                    objeto: Box::new(expr),
                    indice: Box::new(indice),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn argumentos(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        if !self.es(&TokenTipo::ParentesisDer) {
            loop {
                args.push(self.expresion()?);
                if !self.coincidir(&TokenTipo::Coma) {
                    break;
                }
            }
        }
        self.esperar(
            &TokenTipo::ParentesisDer,
            "se esperaba ')' al final de los argumentos",
        )?;
        Ok(args)
    }

    fn primario(&mut self) -> Result<Expr, Error> {
        let token = self.actual().clone();
        match token.tipo.clone() {
            TokenTipo::Entero(n) => {
                self.avanzar();
                Ok(Expr::Entero(n))
            }
            TokenTipo::Decimal(n) => {
                self.avanzar();
                Ok(Expr::Decimal(n))
            }
            TokenTipo::Texto(texto) => {
                self.avanzar();
                Ok(Expr::Texto(texto))
            }
            TokenTipo::TextoInterpolado(crudo) => {
                self.avanzar();
                self.parsear_interpolacion(&crudo, &token)
            }
            TokenTipo::Verdadero => {
                self.avanzar();
                Ok(Expr::Logico(true))
            }
            TokenTipo::Falso => {
                self.avanzar();
                Ok(Expr::Logico(false))
            }
            TokenTipo::Nulo => {
                self.avanzar();
                Ok(Expr::Nulo)
            }
            TokenTipo::Identificador(nombre) => {
                self.avanzar();
                Ok(Expr::Variable(nombre))
            }
            TokenTipo::Este => {
                self.avanzar();
                Ok(Expr::Este)
            }
            TokenTipo::Nuevo => {
                self.avanzar();
                let clase = self.esperar_identificador(
                    "se esperaba el nombre de la clase después de 'nuevo'",
                )?;
                self.esperar(
                    &TokenTipo::ParentesisIzq,
                    "se esperaba '(' después del nombre de la clase",
                )?;
                let args = self.argumentos()?;
                Ok(Expr::Nuevo { clase, args })
            }
            TokenTipo::ParentesisIzq => {
                self.avanzar();
                let expr = self.expresion()?;
                self.esperar(
                    &TokenTipo::ParentesisDer,
                    "se esperaba ')' para cerrar la expresión",
                )?;
                Ok(expr)
            }
            TokenTipo::CorcheteIzq => {
                self.avanzar();
                let mut elementos = Vec::new();
                if !self.es(&TokenTipo::CorcheteDer) {
                    loop {
                        elementos.push(self.expresion()?);
                        if !self.coincidir(&TokenTipo::Coma) {
                            break;
                        }
                        if self.es(&TokenTipo::CorcheteDer) {
                            break;
                        }
                    }
                }
                self.esperar(
                    &TokenTipo::CorcheteDer,
                    "se esperaba ']' al final de la lista",
                )?;
                Ok(Expr::Lista(elementos))
            }
            TokenTipo::LlaveIzq => {
                self.avanzar();
                let mut pares = Vec::new();
                if !self.es(&TokenTipo::LlaveDer) {
                    loop {
                        let clave = match self.actual().tipo.clone() {
                            TokenTipo::Texto(t) => {
                                self.avanzar();
                                t
                            }
                            TokenTipo::Identificador(t) => {
                                self.avanzar();
                                t
                            }
                            palabra if palabra.es_palabra() => {
                                self.avanzar();
                                palabra.simbolo().to_string()
                            }
                            _ => return Err(self.error_actual(
                                "la clave de un diccionario debe ser un texto o un identificador",
                            )),
                        };
                        self.esperar(
                            &TokenTipo::DosPuntos,
                            "se esperaba ':' después de la clave del diccionario",
                        )?;
                        let valor = self.expresion()?;
                        pares.push((clave, valor));
                        if !self.coincidir(&TokenTipo::Coma) {
                            break;
                        }
                        if self.es(&TokenTipo::LlaveDer) {
                            break;
                        }
                    }
                }
                self.esperar(
                    &TokenTipo::LlaveDer,
                    "se esperaba '}' al final del diccionario",
                )?;
                Ok(Expr::Diccionario(pares))
            }
            TokenTipo::Fn | TokenTipo::Funcion => {
                self.avanzar();
                let parametros = self.parametros()?;
                let cuerpo = self.bloque()?;
                Ok(Expr::Funcion {
                    nombre: None,
                    parametros,
                    cuerpo,
                    es_async: false,
                })
            }
            TokenTipo::Async => {
                self.avanzar();
                if !matches!(self.actual().tipo, TokenTipo::Fn | TokenTipo::Funcion) {
                    return Err(
                        self.error_actual("después de 'async' se esperaba una función ('fn')")
                    );
                }
                self.avanzar();
                let parametros = self.parametros()?;
                let cuerpo = self.bloque()?;
                Ok(Expr::Funcion {
                    nombre: None,
                    parametros,
                    cuerpo,
                    es_async: true,
                })
            }
            _ => Err(self.error_actual(format!(
                "se esperaba una expresión, pero se encontró {}",
                token.tipo.descripcion()
            ))),
        }
    }

    fn parsear_interpolacion(&self, crudo: &str, token: &Token) -> Result<Expr, Error> {
        let caracteres: Vec<char> = crudo.chars().collect();
        let mut partes = Vec::new();
        let mut literal = String::new();
        let mut i = 0usize;

        while i < caracteres.len() {
            let c = caracteres[i];
            if c == '\\' && i + 1 < caracteres.len() {
                let siguiente = caracteres[i + 1];
                if siguiente == '{' || siguiente == '}' {
                    literal.push(siguiente);
                } else {
                    literal.push('\\');
                    literal.push(siguiente);
                }
                i += 2;
                continue;
            }
            if c != '{' {
                literal.push(c);
                i += 1;
                continue;
            }

            if !literal.is_empty() {
                partes.push(Expr::Texto(desescapar(&literal)));
                literal.clear();
            }

            let mut profundidad = 1usize;
            let mut j = i + 1;
            let mut fuente_expr = String::new();
            let mut en_texto: Option<char> = None;
            while j < caracteres.len() {
                let cj = caracteres[j];
                if let Some(comilla) = en_texto {
                    if cj == '\\' && j + 1 < caracteres.len() {
                        let siguiente = caracteres[j + 1];
                        if siguiente == comilla {
                            // Las comillas internas deben escaparse por el texto
                            // exterior: \"...\" equivale a "..." en la expresión.
                            fuente_expr.push(siguiente);
                            en_texto = None;
                        } else {
                            fuente_expr.push(cj);
                            fuente_expr.push(siguiente);
                        }
                        j += 2;
                        continue;
                    }
                    if cj == comilla {
                        en_texto = None;
                    }
                    fuente_expr.push(cj);
                    j += 1;
                    continue;
                }
                if cj == '\\' && j + 1 < caracteres.len() {
                    let siguiente = caracteres[j + 1];
                    if siguiente == '"' || siguiente == '\'' {
                        fuente_expr.push(siguiente);
                        en_texto = Some(siguiente);
                        j += 2;
                        continue;
                    }
                    fuente_expr.push(cj);
                    j += 1;
                    continue;
                }
                match cj {
                    '"' | '\'' => {
                        en_texto = Some(cj);
                        fuente_expr.push(cj);
                    }
                    '{' => {
                        profundidad += 1;
                        fuente_expr.push(cj);
                    }
                    '}' => {
                        profundidad -= 1;
                        if profundidad == 0 {
                            break;
                        }
                        fuente_expr.push(cj);
                    }
                    _ => fuente_expr.push(cj),
                }
                j += 1;
            }

            if profundidad != 0 {
                return Err(self.error_en(token, "interpolación sin cerrar: falta '}' en el texto"));
            }

            let fuente_expr = fuente_expr.trim();
            if fuente_expr.is_empty() {
                return Err(
                    self.error_en(token, "las llaves de interpolación no pueden estar vacías")
                );
            }
            let mut sub = Parser::desde_codigo(fuente_expr)?;
            let expr = sub.expresion()?;
            if !sub.es_fin() {
                return Err(sub.error_actual("expresión inválida dentro de la interpolación"));
            }
            partes.push(expr);
            i = j + 1;
        }

        if !literal.is_empty() {
            partes.push(Expr::Texto(desescapar(&literal)));
        }
        if partes.is_empty() {
            partes.push(Expr::Texto(String::new()));
        }
        Ok(Expr::Interpolado(partes))
    }
}

fn es_objetivo_valido(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Variable(_) | Expr::Miembro { .. } | Expr::Indice { .. }
    )
}
