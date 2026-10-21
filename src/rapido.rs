//! Utilidades internas de rendimiento.
//!
//! El intérprete usa mapas con claves de texto cortas en rutas muy calientes
//! (variables, diccionarios, campos de objetos). El hasher por defecto de la
//! biblioteca estándar (SipHash) prioriza resistencia a colisiones, no
//! velocidad. FNV-1a es mucho más barato para este caso de uso.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

const BASE: u64 = 0xcbf2_9ce4_8422_2325;
const PRIMO: u64 = 0x0000_0100_0000_01b3;

/// Hasher FNV-1a de 64 bits.
#[derive(Clone, Copy)]
pub struct Fnv1a(u64);

impl Default for Fnv1a {
    #[inline]
    fn default() -> Self {
        Fnv1a(BASE)
    }
}

impl Hasher for Fnv1a {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIMO);
        }
        self.0 = hash;
    }
}

pub type ConstructorRapido = BuildHasherDefault<Fnv1a>;
pub type MapaRapido<C, V> = HashMap<C, V, ConstructorRapido>;
pub type ConjuntoRapido<C> = HashSet<C, ConstructorRapido>;
