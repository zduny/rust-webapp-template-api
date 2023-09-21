use num_bigint::BigUint;

use zzrpc::{api, Produce};

#[api]
pub trait Api {
    async fn fibonacci(&self, n: u64) -> BigUint;

    async fn factorial(&self, n: u64) -> BigUint;
}

#[derive(Debug, Produce)]
pub struct Producer {}

impl Producer {
    async fn fibonacci(&self, n: u64) -> BigUint {
        crate::fibonacci(n)
    }

    async fn factorial(&self, n: u64) -> BigUint {
        crate::factorial(n)
    }
}
