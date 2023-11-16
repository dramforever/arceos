use core::hash::Hash;
use core::sync::atomic::AtomicU32;

use ahash::RandomState;

static PARK_MILLER_LEHMER_SEED: AtomicU32 = AtomicU32::new(0);
const RAND_MAX: u64 = 2_147_483_647;

fn step_rng() -> u32 {
    use core::sync::atomic::Ordering::SeqCst;
    PARK_MILLER_LEHMER_SEED
        .fetch_update(SeqCst, SeqCst, |x| {
            Some(((u64::from(x) * 48271) % RAND_MAX) as _)
        })
        .unwrap()
}

pub fn random() -> u64 {
    ((step_rng() as u64) << 32) | step_rng() as u64
}

pub struct HashMap<K, V>(hashbrown::HashMap<K, V, RandomState>);

impl<K, V> HashMap<K, V> {
    pub fn new() -> HashMap<K, V> {
        Self(hashbrown::HashMap::with_hasher(RandomState::generate_with(
            random(),
            random(),
            random(),
            random(),
        )))
    }

    pub fn iter(&self) -> impl Iterator<Item=(&K, &V)> {
        self.0.iter()
    }
}

impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn insert(&mut self, key: K, value: V) {
        self.0.insert(key, value);
    }
}
