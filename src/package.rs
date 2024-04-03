use bevy::prelude::*;

use crate::{
    calculate_attach_point_on_conveyor, random::*, Collider, Conveyor, ConveyorLabelTag,
    EntityLayer, GameState, RenderLayers, Velocity, PACKAGE_SIZE, PACKAGE_SPRITE,
};

#[derive(Component)]
pub struct Package;

#[derive(Bundle)]
pub struct PackageBundle {
    pub sprite_bundle: SpriteBundle,
    pub package: Package,
    pub velocity: Velocity,
    pub collider: Collider,
    pub render_layers: RenderLayers,
}

impl Default for PackageBundle {
    fn default() -> Self {
        Self {
            sprite_bundle: SpriteBundle::default(),
            package: Package,
            velocity: Velocity(Vec2::ZERO),
            collider: Collider {
                size: Vec2::new(PACKAGE_SIZE, PACKAGE_SIZE),
            },
            render_layers: RenderLayers::Multi(maplit::btreeset! {EntityLayer::Object}),
        }
    }
}

pub fn spawn_package(commands: &mut Commands, asset_server: &Res<AssetServer>, package_pos: Vec3) {
    commands.spawn(PackageBundle {
        sprite_bundle: SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(PACKAGE_SIZE, PACKAGE_SIZE)),
                ..default()
            },
            transform: Transform {
                translation: package_pos,
                ..default()
            },
            texture: asset_server.load(PACKAGE_SPRITE),
            ..default()
        },
        ..default()
    });
}

pub fn spawn_package_wave(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut conveyor_query: Query<(Entity, &mut Conveyor, &ConveyorLabelTag)>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    mut rng: ResMut<Rand>,
) {
    game_state.package_wave_timer.tick(time.delta());
    if !game_state.package_wave_timer.finished() {
        return;
    }

    game_state.package_wave_timer.reset();
    game_state.package_wave_timer.pause();

    for (conveyor_entity, mut conveyor_info, _) in
        conveyor_query.iter_mut().filter(|(_, _, tag)| match **tag {
            ConveyorLabelTag::Incoming => true,
            _ => false,
        })
    {
        let max_packages_per_row = (conveyor_info.belt_region.x / PACKAGE_SIZE).floor();
        let max_packages_rows = (conveyor_info.belt_region.y / PACKAGE_SIZE).floor();
        let max_package_count = (max_packages_per_row * max_packages_rows) as usize;
        let min_package_count = (max_package_count as f32 * 0.5).floor() as usize;
        let package_count = rng.gen_range(min_package_count..=max_package_count);
        let offset = Vec2::new(0., conveyor_info.belt_region.y);
        for _ in 0..package_count {
            let package_local_translation =
                calculate_attach_point_on_conveyor(&conveyor_info, offset).extend(0.);
            commands.entity(conveyor_entity).with_children(|builder| {
                builder.spawn(PackageBundle {
                    sprite_bundle: SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(PACKAGE_SIZE, PACKAGE_SIZE)),
                            ..default()
                        },
                        transform: Transform {
                            translation: package_local_translation,
                            ..default()
                        },
                        texture: asset_server.load(PACKAGE_SPRITE),
                        ..default()
                    },
                    ..default()
                });
            });

            conveyor_info.package_count += 1;
        }

        conveyor_info.idle_timer.pause();
        conveyor_info.active_timer.reset();
        conveyor_info.active_timer.unpause();
    }
}
