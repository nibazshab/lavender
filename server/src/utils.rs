use ahash::RandomState;
use rand::distr::Alphanumeric;
use rand::{Rng, rng};

static HASHER: RandomState = RandomState::with_seeds(
    0xb147_5fe5_e4d6_2b24,
    0xdfe2_45e0_b058_29fd,
    0x1b02_5f88_b560_c646,
    0xca42_0223_28d8_0700,
);

pub fn hash(t: &str) -> i64 {
    HASHER.hash_one(t) as i64
}

pub fn rand_string(n: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}
