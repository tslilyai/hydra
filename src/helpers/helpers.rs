use crate::*;
use serde::Serialize;
use std::collections::HashMap;

pub fn size_of_vec<T>(vec: &Vec<T>) -> usize {
    std::mem::size_of_val(vec) + vec.capacity() * std::mem::size_of::<T>()
}

pub fn serialize_to_bytes<T: Serialize>(item: &T) -> Vec<u8> {
    bincode::serialize(item).unwrap()
}

pub fn merge_vector_hashmaps<T: Clone>(
    h1: &HashMap<String, Vec<T>>,
    h2: &HashMap<String, Vec<T>>,
) -> HashMap<String, Vec<T>> {
    let mut hm = h1.clone();
    for (k, vs1) in hm.iter_mut() {
        if let Some(vs2) = h2.get(k) {
            vs1.extend_from_slice(vs2);
        }
    }
    for (k, vs2) in h2.iter() {
        if let Some(vs1) = hm.get_mut(k) {
            vs1.extend_from_slice(vs2);
        } else {
            hm.insert(k.to_string(), vs2.clone());
        }
    }
    hm
}

pub fn max(a: u64, b: u64) -> u64 {
    if a >= b {
        a
    } else {
        b
    }
}
