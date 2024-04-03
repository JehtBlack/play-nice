use bevy::prelude::*;

use crate::{
    AnimationData, AnimationTimer, Collider, CollisionEvent, EntityLayer, FacingDirection,
    GameState, Package, PlayAreaAligment, Player, PlayerIndex, RenderLayers, Velocity,
    BASE_PACKAGE_SCORE, BLINKER_SIZE, BLINK_DURATION_SECONDS, CONVEYOR_BORDER_SIZE, CONVEYOR_SIZE,
    CONVEYOR_SPEED, CONVEYOR_SPRITE, CONVEYOR_SPRITE_SIZE, MULTIPLIER_INCREASE_PER_PACKAGE,
    PACKAGE_SIZE,
};

#[derive(Component, PartialEq, Eq)]
pub enum ConveyorLabelTag {
    Incoming,
    Outgoing(PlayerIndex),
}

#[derive(Component)]
pub struct Conveyor {
    pub belt_region: Vec2,
    pub direction: f32,
    pub speed: f32,
    pub active_timer: Timer,
    pub idle_timer: Timer,

    pub package_count: usize,
}

#[derive(Component)]
pub struct Blinker {
    pub blink_timer: Timer,
    pub active_colour: Color,
    pub inactive_colour: Color,
    pub readying_colour: Color,
}

pub fn spawn_conveyor(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    conveyor_pos: Vec3,
    conveyor_belt_length: f32,
    area_alignment: PlayAreaAligment,
    conveyor_tag: ConveyorLabelTag,
) {
    let blinker_pos_modifier = area_alignment.get_blink_position_modifier(&conveyor_tag);
    let blinker = commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::RED,
                    custom_size: Some(Vec2::new(BLINKER_SIZE, BLINKER_SIZE)),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        blinker_pos_modifier * ((CONVEYOR_SIZE.x / 2.) - (BLINKER_SIZE / 2.)),
                        -((conveyor_belt_length / 2.) - (BLINKER_SIZE / 2.)),
                        0.,
                    ),
                    ..default()
                },
                ..default()
            },
            Blinker {
                blink_timer: Timer::from_seconds(BLINK_DURATION_SECONDS, TimerMode::Repeating),
                active_colour: Color::GREEN,
                inactive_colour: Color::RED,
                readying_colour: Color::ORANGE,
            },
            RenderLayers::Single(EntityLayer::Accent),
        ))
        .id();

    let conveyor_border_local_size =
        (CONVEYOR_BORDER_SIZE / CONVEYOR_SPRITE_SIZE.x) * CONVEYOR_SIZE.x;
    let texture_handle: Handle<Image> = asset_server.load(CONVEYOR_SPRITE);
    let atlas_layout = TextureAtlasLayout::from_grid(
        Vec2::new(CONVEYOR_SPRITE_SIZE.x, CONVEYOR_SPRITE_SIZE.y),
        5,
        1,
        None,
        None,
    );
    let animation_indices = AnimationData {
        start_frame: 0,
        frame_count: 5,
        pause: true,
        facing_direction: FacingDirection::Down,
    };
    let mut active_timer =
        Timer::from_seconds(conveyor_belt_length / CONVEYOR_SPEED, TimerMode::Once);
    active_timer.pause();
    let idle_timer = Timer::from_seconds(3., TimerMode::Once);
    commands
        .spawn((
            SpriteSheetBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(CONVEYOR_SIZE.x, conveyor_belt_length)),
                    ..default()
                },
                atlas: TextureAtlas {
                    layout: texture_atlas_layouts.add(atlas_layout),
                    index: animation_indices.start_frame,
                },
                texture: texture_handle,
                transform: Transform {
                    translation: conveyor_pos,
                    ..default()
                },
                ..default()
            },
            Conveyor {
                belt_region: Vec2::new(
                    CONVEYOR_SIZE.x - (conveyor_border_local_size * 2.),
                    conveyor_belt_length,
                ),
                direction: -1.,
                speed: CONVEYOR_SPEED,
                active_timer: active_timer,
                idle_timer: idle_timer,
                package_count: 0,
            },
            Collider {
                size: Vec2::new(CONVEYOR_SIZE.x, conveyor_belt_length),
            },
            RenderLayers::Single(EntityLayer::Furniture),
            animation_indices,
            AnimationTimer(Timer::from_seconds((60. / 5.) / 60., TimerMode::Repeating)),
            conveyor_tag,
        ))
        .add_child(blinker);
}

pub fn calculate_attach_point_on_conveyor(
    conveyor_info: &Conveyor,
    package_relative_offset: Vec2,
) -> Vec2 {
    let max_package_col_count = conveyor_info.belt_region.x / PACKAGE_SIZE;
    let max_package_col_count = max_package_col_count.floor();
    let row = conveyor_info.package_count as f32 / max_package_col_count;
    let col = (row.fract() * max_package_col_count).round();
    let row = row.floor();
    package_relative_offset
        + Vec2::new(
            (col * PACKAGE_SIZE) - ((max_package_col_count * PACKAGE_SIZE) / 2.)
                + (PACKAGE_SIZE / 2.),
            (conveyor_info.belt_region.y / 2.) - (row * PACKAGE_SIZE) - (PACKAGE_SIZE / 2.),
        )
}

pub fn check_for_delivered_packages(
    mut commands: Commands,
    mut conveyor_query: Query<(Entity, &mut Conveyor, &ConveyorLabelTag)>,
    package_query: Query<(Entity, &Transform, &Parent), (With<Package>, Without<Player>)>,
    mut game_state: ResMut<GameState>,
) {
    for (conveyor_entity, mut conveyor_info, label) in
        &mut conveyor_query.iter_mut().filter(|(_, _, t)| match *t {
            ConveyorLabelTag::Outgoing(_) => true,
            _ => false,
        })
    {
        for (package_entity, package_transform, _) in package_query
            .iter()
            .filter(|(_, _, p)| p.get() == conveyor_entity)
        {
            if package_transform.translation.y.abs() > (conveyor_info.belt_region.y / 2.) {
                conveyor_info.package_count -= 1;
                commands
                    .entity(conveyor_entity)
                    .remove_children(&[package_entity]);
                commands.entity(package_entity).despawn();
                match label {
                    ConveyorLabelTag::Outgoing(player_index) => {
                        game_state.player_scores[player_index.index()].score += BASE_PACKAGE_SCORE
                            * game_state.player_scores[player_index.index()].multiplier;
                        game_state.player_scores[player_index.index()].multiplier +=
                            MULTIPLIER_INCREASE_PER_PACKAGE;
                        game_state.player_scores[player_index.index()]
                            .multiplier_decrement_freeze_timer
                            .reset();
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn collect_packages_on_outgoing_conveyors(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut package_query: Query<
        (Entity, &mut Transform, &mut Velocity, Option<&Parent>),
        (With<Package>, Without<Player>),
    >,
    mut conveyor_query: Query<(Entity, &mut Conveyor)>,
) {
    for event in collision_events.read() {
        if let Some((package_entity, mut package_transform, mut package_velocity, package_parent)) =
            package_query
                .iter_mut()
                .find(|(p, _, _, _)| p == &event.entity_a || p == &event.entity_b)
        {
            if let Some((conveyor_entity, mut conveyor_info)) = conveyor_query
                .iter_mut()
                .find(|(c, _)| c == &event.entity_a || c == &event.entity_b)
            {
                if package_parent.is_none() {
                    package_velocity.0 = Vec2::ZERO;
                    package_transform.translation =
                        calculate_attach_point_on_conveyor(&conveyor_info, Vec2::ZERO).extend(0.);
                    commands.entity(conveyor_entity).add_child(package_entity);
                    conveyor_info.package_count += 1;
                }
            }
        }
    }
}

pub fn update_conveyors(
    time: Res<Time>,
    mut game_state: ResMut<GameState>,
    mut conveyor_query: Query<(Entity, &mut Conveyor, &mut AnimationData, &ConveyorLabelTag)>,
    mut blinker_query: Query<(Option<&Parent>, &mut Blinker, &mut Sprite)>,
    mut package_query: Query<(Entity, &mut Transform, &Parent), (With<Package>, Without<Player>)>,
) {
    let mut incoming_conveyors_empty = true;
    for (conveyor_entity, mut conveyor_info, mut anim_data, conveyor_type) in &mut conveyor_query {
        let is_incoming = match conveyor_type {
            ConveyorLabelTag::Incoming => true,
            _ => false,
        };

        conveyor_info.active_timer.tick(time.delta());
        conveyor_info.idle_timer.tick(time.delta());

        if conveyor_info.active_timer.just_finished() {
            conveyor_info.active_timer.pause();
            if !is_incoming {
                conveyor_info.idle_timer.reset();
                conveyor_info.idle_timer.unpause();
            }
        }
        if conveyor_info.idle_timer.just_finished() {
            conveyor_info.idle_timer.pause();
            conveyor_info.active_timer.reset();
            conveyor_info.active_timer.unpause();
        }

        let conveyor_active = !conveyor_info.active_timer.finished();
        let conveyor_just_activated = conveyor_info.idle_timer.just_finished();
        for (parent, mut blinker, mut blinker_sprite) in &mut blinker_query {
            if parent.map_or(true, |p| p.get() != conveyor_entity) {
                continue;
            }

            if conveyor_active {
                if conveyor_just_activated {
                    // conveyor just activated, reset blinker
                    blinker_sprite.color = blinker.active_colour;
                    blinker.blink_timer.reset();
                }
                // conveyor is active, blink the blinker
                anim_data.pause = false;
                blinker.blink_timer.tick(time.delta());
                if blinker.blink_timer.just_finished() {
                    blinker_sprite.color = if blinker_sprite.color != blinker.active_colour {
                        blinker.active_colour
                    } else {
                        Color::BLACK
                    };
                }

                if is_incoming {
                    incoming_conveyors_empty = false;
                }
            } else {
                if is_incoming && conveyor_info.package_count > 0 {
                    // player needs to remove packages before the next wave can come
                    conveyor_info.idle_timer.pause();
                    incoming_conveyors_empty = false;
                }
                // conveyor is inactive, make sure blinker is inactive
                anim_data.pause = true;
                if !conveyor_info.idle_timer.paused()
                    && conveyor_info.idle_timer.fraction_remaining() <= 0.25
                {
                    // 25% of the idle time remaining, let player know we're almost active
                    blinker_sprite.color = blinker.readying_colour;
                } else {
                    blinker_sprite.color = blinker.inactive_colour;
                }
            }
        }

        if conveyor_active {
            for (_package_entity, mut package_transform, _) in package_query
                .iter_mut()
                .filter(|(_, _, p)| p.get() == conveyor_entity)
            {
                package_transform.translation.y +=
                    conveyor_info.direction * conveyor_info.speed * time.delta_seconds();
            }
        }
    }

    if incoming_conveyors_empty && game_state.package_wave_timer.paused() {
        game_state.package_wave_timer.unpause();
        for (_, mut conveyor_info, _, _) in
            conveyor_query.iter_mut().filter(|(_, _, _, t)| match **t {
                ConveyorLabelTag::Incoming => true,
                _ => false,
            })
        {
            conveyor_info.idle_timer.reset();
            conveyor_info
                .idle_timer
                .set_duration(game_state.package_wave_timer.duration());
            conveyor_info.idle_timer.unpause();
        }
    }
}