//! Bytecode de Vaca: instrucciones y unidad de código compilada.

use crate::value::Value;

/// Instrucciones de la máquina virtual.
///
/// Es una máquina de pila: las operaciones consumen y producen valores en
/// `Interprete::pila_vm`. Los locales viven en un `Vec<Option<Value>>` por
/// llamada, así que acceder a una variable es un índice de array.
#[derive(Debug, Clone, Copy)]
pub enum Op {
    /// Empuja una constante del pool.
    Constante(u32),
    Nulo,
    Verdadero,
    Falso,

    CargarLocal(u16),
    GuardarLocal(u16),
    /// `local = local + constante`.
    SumarLocalConst {
        local: u16,
        constante: u32,
    },
    /// `local = local - constante`.
    RestarLocalConst {
        local: u16,
        constante: u32,
    },
    /// Compara dos operandos y salta si es falso (`Chunk::condicionales`).
    SaltoCondicional(u32),
    /// Salta si el local ya tiene valor (usado para parámetros por defecto).
    SaltarSiDefinido {
        local: u16,
        destino: u32,
    },
    CargarExterna(u32),

    Pop,

    Sumar,
    Restar,
    Multiplicar,
    Dividir,
    Modulo,
    Igual,
    Distinto,
    Menor,
    Mayor,
    MenorIgual,
    MayorIgual,
    No,
    Negativo,
    Positivo,

    Saltar(u32),
    SaltarSiFalso(u32),
    SaltarSiVerdadero(u32),

    Llamar(u16),
    LlamarMetodo {
        nombre: u32,
        args: u16,
    },

    CargarMiembro(u32),
    GuardarMiembro(u32),
    CargarIndice,
    GuardarIndice,

    Longitud,
    Lista(u16),
    Diccionario(u16),
    Iterable,
    Interpolar(u16),
    Nuevo {
        clase: u32,
        args: u16,
    },

    /// Convierte el valor de la pila a entero o falla.
    AEntero,
    /// Falla si el local (el paso) vale 0.
    VerificarPaso(u16),
    /// Termina el bucle si la variable salió del rango (`Chunk::condiciones`).
    ParaCondicion(u32),
    /// Avanza la variable del bucle (`Chunk::siguientes`).
    ParaSiguiente(u32),

    Retornar,
    Lanzar,
}

/// Datos de una condición de bucle `para`.
#[derive(Debug, Clone, Copy)]
pub struct DatosCondicion {
    pub variable: u16,
    pub fin: u16,
    pub paso: u16,
    pub destino: u32,
}

/// Datos del avance de un bucle `para`.
#[derive(Debug, Clone, Copy)]
pub struct DatosSiguiente {
    pub variable: u16,
    pub paso: u16,
    pub inicio: u32,
    pub fin: u32,
}

/// Operando de una comparación fusionada.
#[derive(Debug, Clone, Copy)]
pub enum Operando {
    Local(u16),
    Constante(u32),
}

/// Comparación fusionada con salto si el resultado es falso.
#[derive(Debug, Clone, Copy)]
pub struct CondLocal {
    pub izquierda: Operando,
    pub derecha: Operando,
    /// 0 = igual, 1 = distinto, 2 = menor, 3 = mayor, 4 = menor igual, 5 = mayor igual.
    pub operador: u8,
    pub destino: u32,
}

/// Código compilado de una función.
pub struct Chunk {
    pub codigo: Vec<Op>,
    pub constantes: Vec<Value>,
    /// Nombres usados por `CargarExterna`, `CargarMiembro`, etc.
    pub nombres: Vec<String>,
    /// Nombres de los locales, para mensajes de error.
    pub nombres_locales: Vec<String>,
    pub max_locales: u16,
    /// Datos auxiliares de los bucles `para`.
    pub condiciones: Vec<DatosCondicion>,
    pub siguientes: Vec<DatosSiguiente>,
    pub condicionales: Vec<CondLocal>,
    /// Nombres que el cuerpo declara por asignación. Si alguno existe en el
    /// entorno de definición en el momento de la llamada, el código no puede
    /// usarse (la asignación debería actualizar esa variable externa).
    pub dinamicos: Vec<String>,
}
