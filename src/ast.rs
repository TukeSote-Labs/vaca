//! Árbol de sintaxis abstracta de Vaca.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpBinario {
    Suma,
    Resta,
    Multiplicacion,
    Division,
    Modulo,
    Igual,
    Distinto,
    Menor,
    Mayor,
    MenorIgual,
    MayorIgual,
    Y,
    O,
}

impl OpBinario {
    pub fn simbolo(self) -> &'static str {
        match self {
            OpBinario::Suma => "+",
            OpBinario::Resta => "-",
            OpBinario::Multiplicacion => "*",
            OpBinario::Division => "/",
            OpBinario::Modulo => "%",
            OpBinario::Igual => "==",
            OpBinario::Distinto => "!=",
            OpBinario::Menor => "<",
            OpBinario::Mayor => ">",
            OpBinario::MenorIgual => "<=",
            OpBinario::MayorIgual => ">=",
            OpBinario::Y => "y",
            OpBinario::O => "o",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpUnario {
    Negativo,
    Positivo,
    No,
}

#[derive(Debug, Clone)]
pub struct Parametro {
    pub nombre: String,
    pub defecto: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Entero(i64),
    Decimal(f64),
    Texto(String),
    /// Concatenación de texto literal y expresiones dentro de `a"..."`.
    Interpolado(Vec<Expr>),
    Logico(bool),
    Nulo,
    Variable(String),
    Este,
    Lista(Vec<Expr>),
    Diccionario(Vec<(String, Expr)>),
    Binaria {
        op: OpBinario,
        izq: Box<Expr>,
        der: Box<Expr>,
    },
    Unaria {
        op: OpUnario,
        expr: Box<Expr>,
    },
    Llamada {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Miembro {
        objeto: Box<Expr>,
        nombre: String,
    },
    Indice {
        objeto: Box<Expr>,
        indice: Box<Expr>,
    },
    Nuevo {
        clase: String,
        args: Vec<Expr>,
    },
    Funcion {
        nombre: Option<String>,
        parametros: Vec<Parametro>,
        cuerpo: Vec<Stmt>,
        es_async: bool,
    },
    Esperar(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct MetodoClase {
    pub nombre: String,
    pub parametros: Vec<Parametro>,
    pub cuerpo: Vec<Stmt>,
    pub es_async: bool,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expresion(Expr),
    Asignar {
        objetivo: Expr,
        valor: Expr,
    },
    Constante {
        nombre: String,
        valor: Expr,
    },
    Si {
        condicion: Expr,
        entonces: Vec<Stmt>,
        sino_si: Vec<(Expr, Vec<Stmt>)>,
        sino: Option<Vec<Stmt>>,
    },
    Mientras {
        condicion: Expr,
        cuerpo: Vec<Stmt>,
    },
    ParaRango {
        variable: String,
        inicio: Expr,
        fin: Expr,
        paso: Option<Expr>,
        cuerpo: Vec<Stmt>,
    },
    ParaCada {
        variable: String,
        iterable: Expr,
        cuerpo: Vec<Stmt>,
    },
    Funcion {
        nombre: String,
        parametros: Vec<Parametro>,
        cuerpo: Vec<Stmt>,
        es_async: bool,
    },
    Clase {
        nombre: String,
        padre: Option<String>,
        metodos: Vec<MetodoClase>,
    },
    Retornar(Option<Expr>),
    Romper,
    Continuar,
    Lanzar(Expr),
    Intentar {
        cuerpo: Vec<Stmt>,
        variable_error: Option<String>,
        atrapar: Vec<Stmt>,
    },
    Importar {
        modulo: String,
        alias: Option<String>,
    },
    Bloque(Vec<Stmt>),
}
