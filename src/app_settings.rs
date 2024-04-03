use bevy::{ecs::system::Resource, math::Vec2};

#[derive(Resource)]
pub struct AppSettings {
    pub base_resolution: Vec2,
    pub rng_seed: Option<u64>,
}
