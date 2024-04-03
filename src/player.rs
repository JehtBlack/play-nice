use std::collections::HashSet;

use bevy::prelude::*;

use crate::{
    random::*, AnimationData, Collider, CollisionEvent, Conveyor, ConveyorLabelTag, EntityLayer,
    FacingDirection, GameSettings, GameState, Package, PlayerIndex, RenderLayers, Velocity,
    PLAYER_SIZE, PLAYER_SPRITES, PLAYER_SPRITE_SIZE, THROW_POWER,
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
) {
    let tone_sprite = rng.gen_range(0..PLAYER_SPRITES.len());
    let texture_handle: Handle<Image> = asset_server.load(PLAYER_SPRITES[tone_sprite]);
    let atlas_layout = TextureAtlasLayout::from_grid(PLAYER_SPRITE_SIZE, 4, 1, None, None);
    let animation_indices = AnimationData {
        start_frame: 0,
        frame_count: 4,
        pause: true,
        facing_direction: FacingDirection::Down,
    };
    commands
        .spawn((
            SpriteSheetBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
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
            Collider {
                size: Vec2::new(PLAYER_SIZE, PLAYER_SIZE),
            },
            RenderLayers::Single(EntityLayer::Player),
            animation_indices,
        ))
        .with_children(|builder| {
            builder.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(PLAYER_SIZE * 1.2, PLAYER_SIZE * 1.2)),
                        color: player_index.into(),
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
    game_settings: Res<GameSettings>,
    mut query: Query<(&mut Transform, &mut AnimationData, &Player), With<Player>>,
    time: Res<Time>,
) {
    for (mut player_transform, mut player_anim_data, player_data) in &mut query {
        let player_control_state =
            &game_state.player_controls[player_data.player_index.index()].state;
        let sprinting = player_control_state.sprint.pressed();
        // bias to facing horizontally TODO: remove this bias
        let mut new_facing_direction: Option<FacingDirection> = None;
        let mut direction: Vec2 = Vec2::ZERO;

        if player_control_state.move_up.pressed() {
            new_facing_direction = Some(FacingDirection::Up);
            direction.y = 1.;
        } else if player_control_state.move_down.pressed() {
            new_facing_direction = Some(FacingDirection::Down);
            direction.y = -1.;
        }

        if player_control_state.move_left.pressed() {
            new_facing_direction = Some(FacingDirection::Left);
            direction.x = -1.;
        } else if player_control_state.move_right.pressed() {
            new_facing_direction = Some(FacingDirection::Right);
            direction.x = 1.;
        }

        new_facing_direction.map(|f| player_anim_data.facing_direction = f);
        player_transform.translation += direction.normalize_or_zero().extend(0.)
            * game_settings.player_move_speed
            * if sprinting {
                game_settings.player_sprint_move_modifier
            } else {
                1.
            }
            * time.delta_seconds();
    }
}

pub fn pickup_package(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut player_query: Query<(Entity, &mut Player, Option<&Children>), With<Player>>,
    mut package_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut RenderLayers,
            Option<&Parent>,
        ),
        (With<Package>, Without<Player>),
    >,
    mut conveyor_query: Query<(Entity, &mut Conveyor, &ConveyorLabelTag)>,
    game_state: Res<GameState>,
) {
    let mut players_that_have_picked_up_a_package_this_frame = HashSet::<Entity>::new();
    for event in collision_events.read() {
        if let Some((player_entity, mut player_info, player_children)) = player_query
            .iter_mut()
            .find(|(p, _, _)| p == &event.entity_a || p == &event.entity_b)
        {
            let player_wants_to_pickup = game_state.player_controls
                [player_info.player_index.index()]
            .state
            .pickup_or_throw
            .just_pressed();
            if !player_wants_to_pickup {
                continue;
            }

            if player_children.map_or(false, |children| {
                children
                    .iter()
                    .find(|child| package_query.get(**child).is_ok())
                    .is_some()
            }) || players_that_have_picked_up_a_package_this_frame
                .get(&player_entity)
                .is_some()
            {
                // player is already holding a package, don't pick up another
                continue;
            }

            // packages can either be picked up from the conveyor or from the floor
            let package_collision = package_query
                .iter_mut()
                .find(|(p, _, _, _, _)| p == &event.entity_a || p == &event.entity_b);
            let conveyor_collision = conveyor_query.iter_mut().find(|(c, _, label)| {
                (c == &event.entity_a || c == &event.entity_b)
                    && label == &&ConveyorLabelTag::Incoming
            });

            if package_collision.is_some() || conveyor_collision.is_some() {
                // rebind package_collision to the first package child of the conveyor if touching a conveyor
                let package_collision = if conveyor_collision.is_some() {
                    let (conveyor_entity, _, _) = conveyor_collision.unwrap();
                    package_query.iter_mut().find(|(_, _, _, _, parent)| {
                        parent.map_or(false, |parent| parent.get() == conveyor_entity)
                    })
                } else {
                    package_collision
                };

                let (
                    package_entity,
                    mut package_transform,
                    mut package_velocity,
                    mut package_layers,
                    package_parent,
                ) = package_collision.unwrap();

                let currently_held = package_parent.map_or(false, |_p| true);
                let conveyor_holding_package = package_parent.map_or(None, |p| {
                    conveyor_query
                        .iter_mut()
                        .find(|(c, _, _)| c == &p.get())
                        .map_or(None, |(_, c, t)| Some((c, t)))
                });
                if !currently_held
                    || conveyor_holding_package.map_or(false, |(mut c, t)| {
                        if t == &ConveyorLabelTag::Incoming {
                            c.package_count -= 1;
                            true
                        } else {
                            false
                        }
                    })
                {
                    // pick up the package
                    package_velocity.0 = Vec2::ZERO;
                    package_transform.translation = Vec3::new(0., PLAYER_SIZE / 2., 0.);
                    match package_layers.as_mut() {
                        RenderLayers::Multi(layers) => {
                            layers.insert(EntityLayer::HeldObject);
                            ()
                        }
                        _ => {}
                    }
                    commands.entity(player_entity).add_child(package_entity);
                    players_that_have_picked_up_a_package_this_frame.insert(player_entity);
                    player_info.throw_timer.reset();
                    player_info.pickup_cooldown_timer.reset();
                }
            }
        }
    }
}

pub fn throw_package(
    mut commands: Commands,
    player_query: Query<(Entity, &mut Player, &AnimationData, &Transform), With<Player>>,
    mut package_query: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            &mut RenderLayers,
            Option<&Parent>,
        ),
        (With<Package>, Without<Player>),
    >,
    game_state: Res<GameState>,
) {
    for (
        package_entity,
        mut package_transform,
        mut package_velocity,
        mut package_layers,
        package_parent,
    ) in &mut package_query
    {
        let package_parent = if package_parent.is_none() {
            continue;
        } else {
            package_parent.unwrap()
        };

        if let Some((_, player_info, player_anim_data, player_transform)) = player_query
            .iter()
            .find(|(p, _, _, _)| p == &package_parent.get())
        {
            let player_control_state =
                &game_state.player_controls[player_info.player_index.index()].state;
            let player_wants_to_throw = player_control_state.pickup_or_throw.just_released();

            if !player_wants_to_throw {
                continue;
            }

            if !player_info.pickup_cooldown_timer.finished() {
                continue;
            }

            // drop the package
            let current_relative_position = package_transform.translation;
            commands.entity(package_entity).remove_parent();
            match package_layers.as_mut() {
                RenderLayers::Multi(layers) => {
                    layers.remove(&EntityLayer::HeldObject);
                    ()
                }
                _ => {}
            }

            // calculate throw distance
            package_transform.translation =
                player_transform.translation + current_relative_position;
            let throw_distance = player_info.throw_timer.fraction() * THROW_POWER;

            let mut direction = player_anim_data.facing_direction.as_vector();
            if player_control_state.move_up.pressed() {
                direction.y = 1.;
            } else if player_control_state.move_down.pressed() {
                direction.y = -1.;
            }

            if player_control_state.move_left.pressed() {
                direction.x = -1.;
            } else if player_control_state.move_right.pressed() {
                direction.x = 1.;
            }

            package_velocity.0 = direction.normalize_or_zero() * (throw_distance / 0.5);
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
            && game_state.player_controls[player_info.player_index.index()]
                .state
                .pickup_or_throw
                .pressed()
            && player_info.pickup_cooldown_timer.finished()
        {
            player_info.throw_timer.tick(time.delta());
        }
    }
}