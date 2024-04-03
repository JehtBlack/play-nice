use bevy::{
    ecs::system::Resource,
    prelude::{Deref, DerefMut},
};
pub use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

#[derive(Resource, Deref, DerefMut)]
pub struct Rand(ChaCha8Rng);

impl Rand {
    pub fn new(seed: &Option<u64>) -> Self {
        Self(seed.map_or(ChaCha8Rng::from_entropy(), |seed| {
            ChaCha8Rng::seed_from_u64(seed)
        }))
    }
}
