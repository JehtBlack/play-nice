use bevy::prelude::*;
use bevy_rapier2d::{
    control::{KinematicCharacterController, KinematicCharacterControllerOutput},
    dynamics::RigidBody,
    geometry::Collider,
    pipeline::QueryFilter,
    plugin::RapierContext,
};

use crate::{
    activate_package_physics, deactivate_package_physics, random::*, AnimationData, Conveyor,
    ConveyorLabelTag, EntityLayer, FacingDirection, GameConfig, GameState, KeyAction, Package,
    PlayerIndex, RenderLayers, TextureTarget,
};

pub enum PlayAreaAligment {
    Left,
    Right,
}

#[derive(Component)]
pub struct Player {
    pub pickup_cooldown_timer: Timer,
    pub throw_timer: Timer,
    pub player_index: PlayerIndex,
}

impl PlayAreaAligment {
    pub fn get_blink_position_modifier(&self, conveyor_tag: &ConveyorLabelTag) -> f32 {
        match conveyor_tag {
            ConveyorLabelTag::Incoming => match self {
                PlayAreaAligment::Left => -1.,
                PlayAreaAligment::Right => 1.,
            },
            ConveyorLabelTag::Outgoing(_) => match self {
                PlayAreaAligment::Left => 1.,
                PlayAreaAligment::Right => -1.,
            },
        }
    }
}

pub fn spawn_player(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    player_pos: Vec3,
    player_index: PlayerIndex,
    rng: &mut ResMut<Rand>,
    game_config: &Res<GameConfig>,
) {
    let (player_sprite, sprite_path) = if let Some(texture) =
        &game_config.player_config.per_player[player_index].sprite_override
    {
        let sprite = texture.choose_texture(Some(rng));
        (sprite, sprite.path.clone())
    } else {
        let texture_pack = game_config.get_texture_pack();
        let sprite = texture_pack.choose_texture_for(TextureTarget::AllPlayers, Some(rng));
        (sprite, format!("{}/{}", texture_pack.root, sprite.path))
    };

    let texture_handle: Handle<Image> = asset_server.load(&sprite_path);
    let sprite_size = player_sprite
        .cell_resolution
        .expect("Player sprite must have a cell resolution")
        .as_vec2();
    let sprite_grid = player_sprite
        .grid_dimensions
        .expect("Player sprite must have grid dimensions");
    let frame_count = sprite_grid.x * sprite_grid.y;
    let atlas_layout = TextureAtlasLayout::from_grid(
        sprite_size,
        sprite_grid.x as usize,
        sprite_grid.y as usize,
        None,
        None,
    );
    let animation_indices = AnimationData {
        start_frame: 0,
        frame_count: frame_count as usize,
        pause: true,
        facing_direction: FacingDirection::Down,
    };
    commands
        .spawn((
            RigidBody::KinematicPositionBased,
            SpriteSheetBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(
                        game_config.player_config.size,
                        game_config.player_config.size,
                    )),
                    ..default()
                },
                atlas: TextureAtlas {
                    layout: texture_atlas_layouts.add(atlas_layout),
                    index: animation_indices.start_frame,
                },
                texture: texture_handle,
                transform: Transform {
                    translation: player_pos,
                    ..default()
                },
                ..default()
            },
            Player {
                pickup_cooldown_timer: Timer::from_seconds(0.3, TimerMode::Once),
                throw_timer: Timer::from_seconds(1., TimerMode::Once),
                player_index: player_index,
            },
            Collider::cuboid(
                game_config.player_config.size / 2.,
                game_config.player_config.size / 2.,
            ),
            KinematicCharacterController::default(),
            RenderLayers::Single(EntityLayer::Player),
            animation_indices,
        ))
        .with_children(|builder| {
            builder.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(
                            game_config.player_config.size * 1.2,
                            game_config.player_config.size * 1.2,
                        )),
                        color: game_config.player_config.per_player[player_index].colour,
                        ..default()
                    },
                    transform: Transform {
                        translation: Vec3::ZERO,
                        ..default()
                    },
                    ..default()
                },
                RenderLayers::Single(EntityLayer::Accent),
            ));
        });
}

pub fn move_player(
    game_state: Res<GameState>,
    game_config: Res<GameConfig>,
    mut query: Query<
        (
            &mut KinematicCharacterController,
            &mut AnimationData,
            &Player,
        ),
        With<Player>,
    >,
    time: Res<Time>,
) {
    for (mut character_controller, mut player_anim_data, player_data) in &mut query {
        let player_control_state = &game_state.player_controls[player_data.player_index].state;
        let sprinting = player_control_state[KeyAction::Sprint].pressed();
        // bias to facing horizontally TODO: remove this bias
        let mut new_facing_direction: Option<FacingDirection> = None;
        let mut direction: Vec2 = Vec2::ZERO;

        if player_control_state[KeyAction::MoveUp].pressed() {
            new_facing_direction = Some(FacingDirection::Up);
            direction.y = 1.;
        } else if player_control_state[KeyAction::MoveDown].pressed() {
            new_facing_direction = Some(FacingDirection::Down);
            direction.y = -1.;
        }

        if player_control_state[KeyAction::MoveLeft].pressed() {
            new_facing_direction = Some(FacingDirection::Left);
            direction.x = -1.;
        } else if player_control_state[KeyAction::MoveRight].pressed() {
            new_facing_direction = Some(FacingDirection::Right);
            direction.x = 1.;
        }

        new_facing_direction.map(|f| player_anim_data.facing_direction = f);
        character_controller.translation = Some(
            direction.normalize_or_zero()
                * game_config.player_config.move_speed
                * if sprinting {
                    game_config.player_config.sprint_move_modifier
                } else {
                    1.
                }
                * time.delta_seconds(),
        );
    }
}

pub fn pickup_package(
    mut commands: Commands,
    rapier_context: Res<RapierContext>,
    mut player_query: Query<
        (
            Entity,
            &mut Player,
            &Transform,
            &KinematicCharacterControllerOutput,
            Option<&Children>,
        ),
        With<Player>,
    >,
    mut package_query: Query<
        (
            Entity,
            &mut Transform,
            &mut RenderLayers,
            Option<&Parent>,
            Has<RigidBody>,
        ),
        (With<Package>, Without<Player>),
    >,
    mut conveyor_query: Query<(Entity, &mut Conveyor, &ConveyorLabelTag)>,
    game_state: Res<GameState>,
    game_config: Res<GameConfig>,
) {
    for (player_entity, mut player_info, player_transform, player_output, player_children) in
        player_query.iter_mut()
    {
        let player_wants_to_pickup = game_state.player_controls[player_info.player_index].state
            [KeyAction::PickupOrThrow]
            .just_pressed();
        if !player_wants_to_pickup {
            continue;
        }

        if player_children.map_or(false, |children| {
            children
                .iter()
                .find(|child| package_query.get(**child).is_ok())
                .is_some()
        }) {
            // player is already holding a package, don't pick up another
            continue;
        }

        let colliding_conveyors = conveyor_query.iter_mut().filter_map(
            |(conveyor_entity, conveyor_info, conveyor_label)| {
                if conveyor_label != &ConveyorLabelTag::Incoming {
                    return None;
                }

                if let Some(collision) = player_output.collisions.iter().find_map(|collision| {
                    if collision.entity == conveyor_entity {
                        Some(collision)
                    } else {
                        None
                    }
                }) {
                    Some((
                        conveyor_entity,
                        conveyor_info,
                        conveyor_label,
                        collision.toi,
                    ))
                } else {
                    None
                }
            },
        );

        let most_recent_conveyor_collision = colliding_conveyors.max_by(|a, b| {
            a.3.toi
                .partial_cmp(&b.3.toi)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let package = if let Some((conveyor_entity, conveyor_info, conveyor_label, _)) =
            most_recent_conveyor_collision
        {
            if let Some((
                package_entity,
                package_transform,
                package_layers,
                package_parent,
                package_rigid_body,
            )) = package_query.iter_mut().find(|(_, _, _, parent, _)| {
                parent.map_or(false, |parent| parent.get() == conveyor_entity)
            }) {
                Some((
                    package_entity,
                    package_transform,
                    package_layers,
                    package_parent,
                    package_rigid_body,
                    Some((conveyor_entity, conveyor_info, conveyor_label)),
                ))
            } else {
                None
            }
        } else {
            None
        };

        let package = if let Some(package) = package {
            Some(package)
        } else {
            let sensor_area = Collider::ball(game_config.player_config.size * 2.5);
            let filter = QueryFilter {
                exclude_collider: Some(player_entity),
                ..default()
            };
            let mut nearby_packages = Vec::new();
            let mut nearby_conveyors = Vec::new();
            rapier_context.intersections_with_shape(
                player_transform.translation.truncate(),
                0.,
                &sensor_area,
                filter,
                |colliding_entity| {
                    if package_query.get(colliding_entity).is_ok() {
                        nearby_packages.push(colliding_entity);
                    } else if conveyor_query.get(colliding_entity).is_ok() {
                        nearby_conveyors.push(colliding_entity);
                    }
                    true
                },
            );

            if !nearby_packages.is_empty() {
                // find nearest pacakge
                let candidate = package_query
                    .iter_mut()
                    .filter(|(package_entity, _, _, _, _)| nearby_packages.contains(package_entity))
                    .min_by(|a, b| {
                        let sq_dist_a =
                            a.1.translation
                                .truncate()
                                .distance_squared(player_transform.translation.truncate());
                        let sq_dist_b =
                            b.1.translation
                                .truncate()
                                .distance_squared(player_transform.translation.truncate());
                        sq_dist_a
                            .partial_cmp(&sq_dist_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                if let Some((
                    package_entity,
                    package_transform,
                    package_layers,
                    package_parent,
                    package_rigid_body,
                )) = candidate
                {
                    Some((
                        package_entity,
                        package_transform,
                        package_layers,
                        package_parent,
                        package_rigid_body,
                        None,
                    ))
                } else {
                    None
                }
            } else {
                // get first incoming conveyor
                let candidate = conveyor_query
                    .iter_mut()
                    .filter(|(conveyor_entity, _, conveyor_label)| {
                        nearby_conveyors.contains(conveyor_entity)
                            && **conveyor_label == ConveyorLabelTag::Incoming
                    })
                    .next();
                if let Some((conveyor_entity, conveyor_info, conveyor_label)) = candidate {
                    if let Some((
                        package_entity,
                        package_transform,
                        package_layers,
                        package_parent,
                        package_rigid_body,
                    )) = package_query.iter_mut().find(|(_, _, _, parent, _)| {
                        parent.map_or(false, |parent| parent.get() == conveyor_entity)
                    }) {
                        Some((
                            package_entity,
                            package_transform,
                            package_layers,
                            package_parent,
                            package_rigid_body,
                            Some((conveyor_entity, conveyor_info, conveyor_label)),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        let (
            package_entity,
            mut package_transform,
            mut package_layers,
            package_parent,
            package_rigid_body,
            conveyor_holding_package,
        ) = if let Some(package) = package {
            package
        } else {
            continue;
        };

        if !package_parent.map_or(false, |_p| true)
            || conveyor_holding_package.map_or(false, |(_, mut conveyor_info, conveyor_label)| {
                if conveyor_label == &ConveyorLabelTag::Incoming {
                    conveyor_info.package_count -= 1;
                    true
                } else {
                    false
                }
            })
        {
            // pick up the package
            package_transform.translation = Vec3::new(0., game_config.player_config.size / 2., 0.);
            match package_layers.as_mut() {
                RenderLayers::Multi(layers) => {
                    layers.insert(EntityLayer::HeldObject);
                }
                _ => {}
            }
            commands.entity(player_entity).add_child(package_entity);
            player_info.throw_timer.reset();
            player_info.pickup_cooldown_timer.reset();
            if package_rigid_body {
                deactivate_package_physics(&mut commands, package_entity);
            }
        }
    }
}

pub fn throw_package(
    mut commands: Commands,
    player_query: Query<(Entity, &mut Player, &AnimationData, &Transform), With<Player>>,
    mut package_query: Query<
        (Entity, &mut Transform, &mut RenderLayers, Option<&Parent>),
        (With<Package>, Without<Player>),
    >,
    game_state: Res<GameState>,
    game_config: Res<GameConfig>,
) {
    for (package_entity, mut package_transform, mut package_layers, package_parent) in package_query
        .iter_mut()
        .filter(|(_, _, _, package_parent)| package_parent.is_some())
    {
        let package_parent = package_parent.unwrap();

        if let Some((_, player_info, player_anim_data, player_transform)) = player_query
            .iter()
            .find(|(p, _, _, _)| p == &package_parent.get())
        {
            let player_control_state = &game_state.player_controls[player_info.player_index].state;
            let player_wants_to_throw =
                player_control_state[KeyAction::PickupOrThrow].just_released();

            if !player_wants_to_throw || !player_info.pickup_cooldown_timer.finished() {
                continue;
            }

            // drop the package
            commands.entity(package_entity).remove_parent();
            match package_layers.as_mut() {
                RenderLayers::Multi(layers) => {
                    layers.remove(&EntityLayer::HeldObject);
                    ()
                }
                _ => {}
            }

            // calculate throw distance
            let throw_distance = player_info.throw_timer.fraction()
                * (1000. * game_config.player_config.throw_power);

            let mut direction = player_anim_data.facing_direction.as_vector();
            if player_control_state[KeyAction::MoveUp].pressed() {
                direction.y = 1.;
            } else if player_control_state[KeyAction::MoveDown].pressed() {
                direction.y = -1.;
            }

            if player_control_state[KeyAction::MoveLeft].pressed() {
                direction.x = -1.;
            } else if player_control_state[KeyAction::MoveRight].pressed() {
                direction.x = 1.;
            }

            package_transform.translation = player_transform.translation
                + (direction * (game_config.player_config.size / 2.)).extend(0.);
            activate_package_physics(
                &mut commands,
                package_entity,
                &game_config,
                direction * throw_distance,
            );
        }
    }
}

pub fn player_charge_throw(
    mut player_query: Query<(&mut Player, &Children), With<Player>>,
    game_state: Res<GameState>,
    time: Res<Time>,
) {
    for (mut player_info, player_children) in &mut player_query {
        player_info.pickup_cooldown_timer.tick(time.delta());
        if player_children.len() > 0
            && game_state.player_controls[player_info.player_index].state[KeyAction::PickupOrThrow]
                .pressed()
            && player_info.pickup_cooldown_timer.finished()
        {
            player_info.throw_timer.tick(time.delta());
        }
    }
}
