use bevy::prelude::*;

use crate::Conveyor;

#[derive(Debug)]
pub enum FacingDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Component)]
pub struct AnimationData {
    pub start_frame: usize,
    pub frame_count: usize,
    pub pause: bool,
    pub facing_direction: FacingDirection,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

impl FacingDirection {
    pub fn as_sprite_index(&self) -> usize {
        match self {
            FacingDirection::Up => 1,
            FacingDirection::Down => 0,
            FacingDirection::Left => 2,
            FacingDirection::Right => 3,
        }
    }

    pub fn as_vector(&self) -> Vec2 {
        match self {
            FacingDirection::Up => Vec2::new(0., 1.),
            FacingDirection::Down => Vec2::new(0., -1.),
            FacingDirection::Left => Vec2::new(-1., 0.),
            FacingDirection::Right => Vec2::new(1., 0.),
        }
    }
}

pub fn animate_sprite_maps(
    time: Res<Time>,
    mut sprite_map_query: Query<(&AnimationData, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (anim_data, mut timer, mut atlas) in sprite_map_query
        .iter_mut()
        .filter(|(anim_data, _, _)| !anim_data.pause)
    {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            atlas.index = (atlas.index + 1) % anim_data.frame_count;
        }
    }
}

pub fn select_sprite_facing_index(
    mut query: Query<(&AnimationData, &mut TextureAtlas), Without<Conveyor>>,
) {
    for (anim_data, mut atlas) in &mut query {
        atlas.index = anim_data.start_frame + anim_data.facing_direction.as_sprite_index();
    }
}
