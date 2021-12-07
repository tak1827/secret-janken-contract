use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use sha2::{Digest, Sha256};
use std::convert::TryInto;
use subtle::ConstantTimeEq;

use crate::contract::INVERSE_BASIS_POINT;
use crate::viewing_key::VIEWING_KEY_SIZE;

pub const SHA256_HASH_SIZE: usize = 32;

pub fn ct_slice_compare(s1: &[u8], s2: &[u8]) -> bool {
    bool::from(s1.ct_eq(s2))
}

pub fn create_hashed_password(s1: &str) -> [u8; VIEWING_KEY_SIZE] {
    Sha256::digest(s1.as_bytes())
        .as_slice()
        .try_into()
        .expect("Wrong password length")
}

// pub fn to_array<T, const N: usize>(v: Vec<T>) -> [T; N] {
//     v.try_into()
//         .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
// }

pub fn sha_256(data: &[u8]) -> [u8; SHA256_HASH_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_slice());
    result
}

pub fn calculate_fee(amount: u64, fee_rate: u64) -> u64 {
    amount * fee_rate / INVERSE_BASIS_POINT
}

pub struct Prng {
    rng: ChaChaRng,
}

impl Prng {
    pub fn new(seed: &[u8], entropy: &[u8]) -> Self {
        let mut hasher = Sha256::new();

        // write input message
        hasher.update(&seed);
        hasher.update(&entropy);
        let hash = hasher.finalize();

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(hash.as_slice());

        let rng = ChaChaRng::from_seed(hash_bytes);

        Self { rng }
    }

    pub fn rand_bytes(&mut self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        self.rng.fill_bytes(&mut bytes);

        bytes
    }

    pub fn new_rand_bytes(seed: &[u8], entropy: &[u8]) -> Vec<u8> {
        let mut rng = Self::new(seed, (entropy).as_ref());
        let rand_slice = rng.rand_bytes();
        sha_256(&rand_slice).to_vec()
    }
}
