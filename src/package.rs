use crate::{
    calculate_attach_point_on_conveyor, random::*, Conveyor, ConveyorLabelTag, EntityLayer,
    GameConfig, GameState, RenderLayers, TextureTarget,
};
use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

#[derive(Component)]
pub struct Package;

#[derive(Bundle)]
pub struct PackageBundle {
    pub sprite_bundle: SpriteBundle,
    pub package: Package,
    pub render_layers: RenderLayers,
}

#[derive(Bundle)]
pub struct PackagePhysicsBundle {
    pub rigid_body: RigidBody,
    pub mass_props: ColliderMassProperties,
    pub damping: Damping,
    pub collider: Collider,
    pub locked_axes: LockedAxes,
    pub friction: Friction,
    pub restitution: Restitution,
    pub impulse: ExternalImpulse,
}

impl Default for PackageBundle {
    fn default() -> Self {
        Self {
            sprite_bundle: SpriteBundle::default(),
            package: Package,
            render_layers: RenderLayers::Multi(maplit::btreeset! {EntityLayer::Object}),
        }
    }
}

impl Default for PackagePhysicsBundle {
    fn default() -> Self {
        Self {
            rigid_body: RigidBody::Dynamic,
            mass_props: ColliderMassProperties::Density(500.),
            damping: Damping {
                linear_damping: 1.,
                ..default()
            },
            collider: Collider::default(),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            friction: Friction {
                coefficient: 1.,
                ..default()
            },
            restitution: Restitution {
                coefficient: 0.1,
                ..default()
            },
            impulse: ExternalImpulse::default(),
        }
    }
}

pub fn spawn_package(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    game_config: &Res<GameConfig>,
    package_pos: Vec3,
) {
    let texture_pack = game_config.get_texture_pack();
    let package_sprite = texture_pack.choose_texture_for(TextureTarget::Package, None);
    commands.spawn((
        PackageBundle {
            sprite_bundle: SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(
                        game_config.package_config.size,
                        game_config.package_config.size,
                    )),
                    ..default()
                },
                transform: Transform {
                    translation: package_pos,
                    ..default()
                },
                texture: asset_server
                    .load(&format!("{}/{}", texture_pack.root, package_sprite.path)),
                ..default()
            },
            package: Package,
            render_layers: RenderLayers::Multi(maplit::btreeset! {EntityLayer::Object}),
        },
        PackagePhysicsBundle {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(
                game_config.package_config.size / 2.,
                game_config.package_config.size / 2.,
            ),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            ..default()
        },
    ));
}

pub fn spawn_package_wave(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut conveyor_query: Query<(Entity, &mut Conveyor, &ConveyorLabelTag)>,
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    game_config: Res<GameConfig>,
    mut rng: ResMut<Rand>,
) {
    game_state.package_wave_timer.tick(time.delta());
    if !game_state.package_wave_timer.finished() {
        return;
    }

    game_state.package_wave_timer.reset();
    game_state.package_wave_timer.pause();

    let texture_pack = game_config.get_texture_pack();
    let package_sprite = texture_pack.choose_texture_for(TextureTarget::Package, None);
    let package_sprite_path = format!("{}/{}", texture_pack.root, package_sprite.path);
    for (conveyor_entity, mut conveyor_info, _) in
        conveyor_query.iter_mut().filter(|(_, _, tag)| match **tag {
            ConveyorLabelTag::Incoming => true,
            _ => false,
        })
    {
        let max_packages_per_row =
            (conveyor_info.belt_region.x / game_config.package_config.size).floor();
        let max_packages_rows =
            (conveyor_info.belt_region.y / game_config.package_config.size).floor();
        let max_package_count = (max_packages_per_row * max_packages_rows) as usize;
        let min_package_count = (max_package_count as f32 * 0.5).floor() as usize;
        let package_count = rng.gen_range(min_package_count..=max_package_count);
        let offset = Vec2::new(0., conveyor_info.belt_region.y);
        for _ in 0..package_count {
            let package_local_translation = calculate_attach_point_on_conveyor(
                &conveyor_info,
                offset,
                game_config.package_config.size,
            )
            .extend(0.);
            commands.entity(conveyor_entity).with_children(|builder| {
                builder.spawn(PackageBundle {
                    sprite_bundle: SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(
                                game_config.package_config.size,
                                game_config.package_config.size,
                            )),
                            ..default()
                        },
                        transform: Transform {
                            translation: package_local_translation,
                            ..default()
                        },
                        texture: asset_server.load(&package_sprite_path),
                        ..default()
                    },
                    package: Package,
                    render_layers: RenderLayers::Multi(maplit::btreeset! {EntityLayer::Object}),
                });
            });

            conveyor_info.package_count += 1;
        }

        conveyor_info.idle_timer.pause();
        conveyor_info.active_timer.reset();
        conveyor_info.active_timer.unpause();
    }
}

pub fn activate_package_physics(
    commands: &mut Commands,
    package_entity: Entity,
    game_config: &Res<GameConfig>,
    impulse_to_apply: Vec2,
) {
    commands
        .entity(package_entity)
        .insert(PackagePhysicsBundle {
            rigid_body: RigidBody::Dynamic,
            collider: Collider::cuboid(
                game_config.package_config.size / 2.,
                game_config.package_config.size / 2.,
            ),
            locked_axes: LockedAxes::ROTATION_LOCKED,
            impulse: ExternalImpulse {
                impulse: impulse_to_apply,
                ..default()
            },
            ..default()
        });
}

pub fn deactivate_package_physics(commands: &mut Commands, package_entity: Entity) {
    commands
        .entity(package_entity)
        .remove::<PackagePhysicsBundle>();
}
