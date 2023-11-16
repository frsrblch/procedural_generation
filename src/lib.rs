//! Types and traits for generating deterministic pseudorandom numbers.
//!
//! To generate values of type `T`, a key must implement and `PrngKey` and `Generate<T>`

#![feature(associated_type_defaults)]

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

/// Types that can be used as keys when generating deterministic pseudrandom values
pub trait PrngKey {
    fn key(&self) -> u64;
}

/// Types that can be used to generate deterministic pseudorandom values of `T`
pub trait Generate<T> {
    /// A hard-coded random number that is xor'ed with the seed value and key value to produce values that are unique to that seed-key-type
    const XOR: u128;
    /// The sample distribution
    type Distribution = Standard;
    fn distribution() -> Self::Distribution;
}

/// Helper trait for generating deterministic pseudorandom values for `PrngKey` keys that implement `Generate<T>`
pub trait Prng<K: PrngKey> {
    /// Generates a value for the given `Seed` and key `K`
    fn generate<T>(&self, key: &K) -> T
    where
        K: Generate<T>,
        <K as Generate<T>>::Distribution: Distribution<T>;

    fn rng<T>(&self, key: &K) -> rand_pcg::Pcg64Mcg
    where
        K: Generate<T>;
}

/// Seed values for procedurally generating deterministic pseudo-random numbers
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Seed(pub u128);

impl Seed {
    /// Generate a `Seed` by hashing an input `&str`
    pub fn new_from_str(seed: &str) -> Self {
        let hash = &blake3::hash(seed.as_bytes());
        let bytes = std::array::from_fn(|i| hash.as_bytes()[i]);
        let u128 = u128::from_ne_bytes(bytes);
        Seed(u128)
    }
}

impl From<u128> for Seed {
    fn from(value: u128) -> Self {
        Seed(value)
    }
}

impl Distribution<Seed> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Seed {
        Seed(rng.gen())
    }
}

impl<K: PrngKey> Prng<K> for Seed {
    fn generate<T>(&self, key: &K) -> T
    where
        K: Generate<T>,
        <K as Generate<T>>::Distribution: Distribution<T>,
    {
        let mut rng = self.rng(key);
        K::distribution().sample(&mut rng)
    }

    fn rng<T>(&self, key: &K) -> rand_pcg::Pcg64Mcg
    where
        K: Generate<T>,
    {
        // rand_pcg::Pcg64Mcg::new sets the lowest bit to 1, so the key cannot overlap with that bit
        let key = (key.key() as u128) << 64;
        let rng_seed = self.0 ^ K::XOR ^ key;
        rand_pcg::Pcg64Mcg::new(rng_seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct ValueKey(u64);

    impl ValueKey {
        pub fn new(index: u64) -> Self {
            ValueKey(index)
        }
    }

    impl PrngKey for ValueKey {
        fn key(&self) -> u64 {
            self.0
        }
    }

    #[derive(Debug, PartialEq)]
    struct Value1(f64);

    impl rand::distributions::Distribution<Value1> for rand::distributions::Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Value1 {
            Value1(rng.gen())
        }
    }

    impl Generate<Value1> for ValueKey {
        const XOR: u128 = 1;
        fn distribution() -> Self::Distribution {
            rand::distributions::Standard
        }
    }

    #[derive(Debug, PartialEq)]
    struct Value2(f64);

    impl PartialEq<Value2> for Value1 {
        fn eq(&self, other: &Value2) -> bool {
            self.0.eq(&other.0)
        }
    }

    impl rand::distributions::Distribution<Value2> for rand::distributions::Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Value2 {
            Value2(rng.gen())
        }
    }

    impl Generate<Value2> for ValueKey {
        const XOR: u128 = 2;
        fn distribution() -> Self::Distribution {
            rand::distributions::Standard
        }
    }

    #[test]
    fn same_key_and_same_type_returns_same_values() {
        let seed = Seed::new_from_str("value test");
        let key = ValueKey::new(7);
        let value1a = seed.generate::<Value1>(&key);
        let value1b = seed.generate::<Value1>(&key);
        assert_eq!(value1a, value1b);
    }

    #[test]
    fn same_key_and_different_type_returns_different_values() {
        let seed = Seed::new_from_str("value test");
        let key = ValueKey::new(7);
        let value1 = seed.generate::<Value1>(&key);
        let value2 = seed.generate::<Value2>(&key);
        assert_ne!(value1, value2);
    }

    #[test]
    fn unit_key_return_consistent_values() {
        let seed = Seed::new_from_str("global test");

        /// Prng global values
        #[derive(Debug, PartialEq)]
        pub struct Global(f64);

        impl PrngKey for () {
            fn key(&self) -> u64 {
                0
            }
        }

        impl Generate<Global> for () {
            const XOR: u128 = 635184615;
            fn distribution() -> Self::Distribution {
                rand::distributions::Standard
            }
        }

        impl rand::distributions::Distribution<Global> for rand::distributions::Standard {
            fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Global {
                Global(rng.gen())
            }
        }

        let global1 = seed.generate::<Global>(&());
        let global2 = seed.generate::<Global>(&());

        assert_eq!(global1, global2);
    }

    #[test]
    fn zero_and_one_generate_different_values() {
        let seed = Seed::new_from_str("test");
        let k1 = ValueKey(0);
        let k2 = ValueKey(1);
        assert_ne!(seed.generate::<Value1>(&k1), seed.generate::<Value1>(&k2));
    }

    #[test]
    fn prng_rng_and_generate() {
        let seed = Seed::new_from_str("rng and generate");
        let key = ValueKey(23);
        let mut rng = seed.rng::<Value1>(&key);
        let rng_value = rng.gen::<Value1>();
        let generate_value = seed.generate::<Value1>(&key);
        assert_eq!(rng_value, generate_value);
    }
}
