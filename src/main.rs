use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    sprite::Anchor,
    text::{JustifyText, Text2dBounds},
    window::WindowResolution,
};

mod app_settings;
mod collision;
mod conveyor;
mod game_mode;
mod package;
mod player;
mod player_identification;
mod random;
mod render_layers;
mod sprite_animation;
mod sprite_render_layers;
mod supervisor;
mod user_input;

use app_settings::*;
use collision::*;
use conveyor::*;
use game_mode::*;
use package::*;
use player::*;
use player_identification::*;
use random::*;
use render_layers::*;
use sprite_animation::*;
use sprite_render_layers::*;
use supervisor::*;
use user_input::*;

const PACKAGE_SIZE: f32 = 30.;

const PLAYER_SIZE: f32 = 30.;
const PLAYER_SPRITE_SIZE: Vec2 = Vec2::new(128., 128.);

const CONVEYOR_SIZE: Vec2 = Vec2::new(128., 500.);
const CONVEYOR_SPEED: f32 = 100.;
const CONVEYOR_SPRITE_SIZE: Vec2 = Vec2::new(128., 128.);
const CONVEYOR_BORDER_SIZE: f32 = 14.;

const BLINKER_SIZE: f32 = 20.;
const BLINK_DURATION_SECONDS: f32 = 0.1;

const PLAYER_SPRITES: [&'static str; 4] = [
    "sprites/player_skin_tone_a.png",
    "sprites/player_skin_tone_b.png",
    "sprites/player_skin_tone_c.png",
    "sprites/player_skin_tone_d.png",
];
const PACKAGE_SPRITE: &'static str = "sprites/box.png";
const CONVEYOR_SPRITE: &'static str = "sprites/conveyor.png";
const BACKGROUND_SPRITE: &'static str = "sprites/background.png";
const SUPERVISOR_SPRITES: [&'static str; 4] = [
    "sprites/supervisor_skin_tone_a.png",
    "sprites/supervisor_skin_tone_b.png",
    "sprites/supervisor_skin_tone_c.png",
    "sprites/supervisor_skin_tone_d.png",
];
const SUPERVISOR_OFFICE_SPRITE: &'static str = "sprites/supervisor_office.png";
const DISPLAY_SPRITE: &'static str = "sprites/display.png";

const SUPERVISOR_OFFICE_SIZE: Vec2 = Vec2::new(400., 150.);

const BASE_PACKAGE_SCORE: f32 = 5.;
const MULTIPLIER_INCREASE_PER_PACKAGE: f32 = 0.1;
const MULTIPLIER_DECREASE_PER_SECOND: f32 = 0.1;

/// number of world units a full power throw will cause a package to travel in one second
const THROW_POWER: f32 = 100.;
/// world units / second / second
const FRICTION: f32 = 100.;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let app_settings = AppSettings {
        base_resolution: Vec2::new(
            dotenv::var("RESOLUTION_X").map_or(Ok(1280.), |f| f.parse())?,
            dotenv::var("RESOLUTION_Y").map_or(Ok(720.), |f| f.parse())?,
        ),
        rng_seed: dotenv::var("RNG_SEED").map_or(None, |s| {
            let v: anyhow::Result<u64> = s.parse::<u64>().or(Ok({
                let mut hasher = DefaultHasher::new();
                s.hash(&mut hasher);
                hasher.finish()
            }));
            v.ok()
        }),
    };

    let game_settings = GameSettings {
        player_move_speed: dotenv::var("PLAYER_MOVE_SPEED").map_or(Ok(150.), |f| f.parse())?,
        player_sprint_move_modifier: dotenv::var("PLAYER_SPRINT_MOVE_MODIFIER")
            .map_or(Ok(2.), |f| f.parse())?,
        supervisor_monitoring_y_pos: dotenv::var("SUPERVISOR_MONITOR_Y_POS").map_or(
            Ok((app_settings.base_resolution.y / 2.) - (SUPERVISOR_OFFICE_SIZE.y / 2.)),
            |f| f.parse(),
        )?,
    };

    let rng = Rand::new(&app_settings.rng_seed);

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        app_settings.base_resolution.x,
                        app_settings.base_resolution.y,
                    )
                    .with_scale_factor_override(1.),
                    title: "Play Nice!".to_string(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(SpriteLayerPlugin::<RenderLayers>::default())
        .insert_resource(game_settings)
        .insert_resource(app_settings)
        .insert_resource(rng)
        .insert_resource(GameState {
            player_scores: [
                PlayerScoreData {
                    score: 0.,
                    multiplier: 1.,
                    multiplier_decrement_freeze_timer: Timer::from_seconds(2., TimerMode::Once),
                },
                PlayerScoreData {
                    score: 0.,
                    multiplier: 1.,
                    multiplier_decrement_freeze_timer: Timer::from_seconds(2., TimerMode::Once),
                },
            ],
            package_wave_timer: Timer::from_seconds(5., TimerMode::Once),
            player_controls: [
                PlayerControls {
                    pad: None,
                    mapping: ControlMapping {
                        move_up: ButtonMapping {
                            keyboard_key: KeyCode::KeyW,
                            gamepad_button: Some(GamepadButtonType::DPadUp),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_down: ButtonMapping {
                            keyboard_key: KeyCode::KeyS,
                            gamepad_button: Some(GamepadButtonType::DPadDown),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_left: ButtonMapping {
                            keyboard_key: KeyCode::KeyA,
                            gamepad_button: Some(GamepadButtonType::DPadLeft),
                            gamepad_axis: Some(GamepadAxisType::LeftStickX),
                        },
                        move_right: ButtonMapping {
                            keyboard_key: KeyCode::KeyD,
                            gamepad_button: Some(GamepadButtonType::DPadRight),
                            gamepad_axis: Some(GamepadAxisType::LeftStickX),
                        },
                        sprint: ButtonMapping {
                            keyboard_key: KeyCode::ShiftLeft,
                            gamepad_button: Some(GamepadButtonType::LeftTrigger),
                            gamepad_axis: Some(GamepadAxisType::LeftZ),
                        },
                        pickup_or_throw: ButtonMapping {
                            keyboard_key: KeyCode::Space,
                            gamepad_button: Some(GamepadButtonType::RightTrigger),
                            gamepad_axis: Some(GamepadAxisType::RightZ),
                        },
                    },
                    state: ControlState {
                        move_up: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        move_down: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        move_left: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        move_right: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        sprint: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        pickup_or_throw: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                    },
                },
                PlayerControls {
                    pad: None,
                    mapping: ControlMapping {
                        move_up: ButtonMapping {
                            keyboard_key: KeyCode::ArrowUp,
                            gamepad_button: Some(GamepadButtonType::DPadUp),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_down: ButtonMapping {
                            keyboard_key: KeyCode::ArrowDown,
                            gamepad_button: Some(GamepadButtonType::DPadDown),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_left: ButtonMapping {
                            keyboard_key: KeyCode::ArrowLeft,
                            gamepad_button: Some(GamepadButtonType::DPadLeft),
                            gamepad_axis: Some(GamepadAxisType::LeftStickX),
                        },
                        move_right: ButtonMapping {
                            keyboard_key: KeyCode::ArrowRight,
                            gamepad_button: Some(GamepadButtonType::DPadRight),
                            gamepad_axis: Some(GamepadAxisType::LeftStickX),
                        },
                        sprint: ButtonMapping {
                            keyboard_key: KeyCode::ShiftRight,
                            gamepad_button: Some(GamepadButtonType::LeftTrigger),
                            gamepad_axis: Some(GamepadAxisType::LeftZ),
                        },
                        pickup_or_throw: ButtonMapping {
                            keyboard_key: KeyCode::ControlRight,
                            gamepad_button: Some(GamepadButtonType::RightTrigger),
                            gamepad_axis: Some(GamepadAxisType::RightZ),
                        },
                    },
                    state: ControlState {
                        move_up: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        move_down: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        move_left: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        move_right: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        sprint: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        pickup_or_throw: ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                    },
                },
            ],
        })
        .add_event::<CollisionEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                gamepad_connected,
                update_controller_mappings,
                spawn_package_wave,
                move_player,
                update_conveyors,
                update_velocities,
                player_charge_throw,
                throw_package,
                check_for_collisions,
                pickup_package,
                collect_packages_on_outgoing_conveyors,
                check_for_delivered_packages,
                update_supervisor,
                check_supervisor_can_see_players,
                react_to_basic_collisions,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                animate_sprite_maps,
                select_sprite_facing_index,
                update_score_multipiers,
                update_scores,
                bevy::window::close_on_esc,
            ),
        )
        .add_systems(Last, clear_frame_collisions)
        .run();

    Ok(())
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    app_settings: Res<AppSettings>,
    game_settings: Res<GameSettings>,
    mut rng: ResMut<Rand>,
) {
    // default projection has 0.1 near and 1000. far, but Camera2dBundle defaults to -1000. near and 1000. far
    // start with the bundle defaults and mutate the projection scaling mode
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::Fixed {
        width: app_settings.base_resolution.x,
        height: app_settings.base_resolution.y,
    };
    commands.spawn(camera_bundle);

    spawn_player(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(
            -(app_settings.base_resolution.x / 2.) + CONVEYOR_SIZE.x + (PLAYER_SIZE / 2.),
            0.,
            0.,
        ),
        PlayerIndex::Player1,
        &mut rng,
    );

    spawn_player(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(
            (app_settings.base_resolution.x / 2.) - CONVEYOR_SIZE.x - (PLAYER_SIZE / 2.),
            0.,
            0.,
        ),
        PlayerIndex::Player2,
        &mut rng,
    );

    let conveyor_walkway_size = Vec2::new(CONVEYOR_SIZE.x * 2., SUPERVISOR_OFFICE_SIZE.y);
    let incoming_belt_length =
        app_settings.base_resolution.y - SUPERVISOR_OFFICE_SIZE.y - conveyor_walkway_size.y;
    let outgoing_belt_length = app_settings.base_resolution.y - SUPERVISOR_OFFICE_SIZE.y;
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(-CONVEYOR_SIZE.x / 2., 0., 0.),
        incoming_belt_length,
        PlayAreaAligment::Left,
        ConveyorLabelTag::Incoming,
    );
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(
            -(app_settings.base_resolution.x / 2.) + (CONVEYOR_SIZE.x / 2.),
            -(app_settings.base_resolution.y / 2.) + (outgoing_belt_length / 2.),
            0.,
        ),
        outgoing_belt_length,
        PlayAreaAligment::Left,
        ConveyorLabelTag::Outgoing(PlayerIndex::Player1),
    );

    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(CONVEYOR_SIZE.x / 2., 0., 0.),
        incoming_belt_length,
        PlayAreaAligment::Right,
        ConveyorLabelTag::Incoming,
    );
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(
            (app_settings.base_resolution.x / 2.) - (CONVEYOR_SIZE.x / 2.),
            -(app_settings.base_resolution.y / 2.) + (outgoing_belt_length / 2.),
            0.,
        ),
        outgoing_belt_length,
        PlayAreaAligment::Right,
        ConveyorLabelTag::Outgoing(PlayerIndex::Player2),
    );

    spawn_supervisor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(0., game_settings.supervisor_monitoring_y_pos, 0.),
        &mut rng,
    );

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::CenterLeft,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(app_settings.base_resolution.x / 2., 0.).extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(10., app_settings.base_resolution.y),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::CenterRight,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(-app_settings.base_resolution.x / 2., 0.).extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(10., app_settings.base_resolution.y),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::CenterLeft,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(
                    -CONVEYOR_SIZE.x + 10.,
                    (incoming_belt_length / 2.) + (SUPERVISOR_OFFICE_SIZE.y / 2.),
                )
                .extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(10., SUPERVISOR_OFFICE_SIZE.y),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::CenterRight,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(
                    CONVEYOR_SIZE.x - 10.,
                    (incoming_belt_length / 2.) + (SUPERVISOR_OFFICE_SIZE.y / 2.),
                )
                .extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(10., SUPERVISOR_OFFICE_SIZE.y),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::BottomCenter,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(
                    0.,
                    (app_settings.base_resolution.y / 2.) - (SUPERVISOR_OFFICE_SIZE.y / 2.),
                )
                .extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(app_settings.base_resolution.x, 10.),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::TopCenter,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(0., -app_settings.base_resolution.y / 2.).extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(app_settings.base_resolution.x, 10.),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    let display_sprite_handle = asset_server.load(DISPLAY_SPRITE);
    let team_display_size = Vec2::new(SUPERVISOR_OFFICE_SIZE.x * 0.5, 24.);
    let team_display_pos = Vec2::new(0., -(SUPERVISOR_OFFICE_SIZE.y / 2.));
    let team_display_border: f32 = 6.;
    let player_displays_size = [
        Vec2::new(SUPERVISOR_OFFICE_SIZE.x * 0.5, 24.),
        Vec2::new(SUPERVISOR_OFFICE_SIZE.x * 0.5, 24.),
    ];
    let player_displays_pos = [
        Vec2::new(
            -(app_settings.base_resolution.x / 2.) + (player_displays_size[0].x * 0.5),
            12.,
        ),
        Vec2::new(
            (app_settings.base_resolution.x / 2.) - (player_displays_size[1].x * 0.5),
            12.,
        ),
    ];

    let player_colours = [PlayerIndex::Player1.into(), PlayerIndex::Player2.into()];
    let team_colour = Color::rgb_linear(0.6, 0.1, 0.6);
    let player_displays_border: [f32; 2] = [6., 6.];
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(
                        app_settings.base_resolution.x,
                        SUPERVISOR_OFFICE_SIZE.y,
                    )),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        0.,
                        (app_settings.base_resolution.y / 2.) - (SUPERVISOR_OFFICE_SIZE.y / 2.),
                        0.,
                    ),
                    ..default()
                },
                texture: asset_server.load(SUPERVISOR_OFFICE_SPRITE),
                ..default()
            },
            RenderLayers::Single(EntityLayer::OfficeLevelFurniture),
        ))
        .with_children(|builder| {
            builder
                .spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(team_display_size),
                            anchor: Anchor::BottomCenter,
                            ..default()
                        },
                        transform: Transform {
                            translation: team_display_pos.extend(0.),
                            ..default()
                        },
                        texture: display_sprite_handle.clone(),
                        ..default()
                    },
                    RenderLayers::Single(EntityLayer::OfficeLevelAccent),
                ))
                .with_children(|builder| {
                    builder.spawn((
                        Text2dBundle {
                            text: Text::from_sections([
                                TextSection::new(
                                    "Team Score: ",
                                    TextStyle {
                                        font_size: 20.0,
                                        color: team_colour,
                                        ..default()
                                    },
                                ),
                                TextSection::new(
                                    "0",
                                    TextStyle {
                                        font_size: 20.0,
                                        color: team_colour,
                                        ..default()
                                    },
                                ),
                            ])
                            .with_justify(JustifyText::Right),
                            text_anchor: Anchor::BottomRight,
                            text_2d_bounds: Text2dBounds {
                                size: Vec2::new(
                                    team_display_size.x - (team_display_border * 2.),
                                    team_display_size.y,
                                ),
                                ..default()
                            },
                            transform: Transform {
                                translation: Vec3::new(
                                    team_display_size.x / 2. - team_display_border,
                                    0.,
                                    100.,
                                ),
                                ..default()
                            },
                            ..default()
                        },
                        TeamScoreTag,
                    ));
                });

            builder
                .spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(player_displays_size[0]),
                            anchor: Anchor::BottomLeft,
                            ..default()
                        },
                        transform: Transform {
                            translation: player_displays_pos[0].extend(0.),
                            ..default()
                        },
                        texture: display_sprite_handle.clone(),
                        ..default()
                    },
                    RenderLayers::Single(EntityLayer::OfficeLevelAccent),
                ))
                .with_children(|builder| {
                    builder.spawn((
                        Text2dBundle {
                            text: Text::from_sections([
                                TextSection::new(
                                    "Score: ",
                                    TextStyle {
                                        font_size: 20.0,
                                        color: player_colours[0],
                                        ..default()
                                    },
                                ),
                                TextSection::new(
                                    "0",
                                    TextStyle {
                                        font_size: 20.0,
                                        color: player_colours[0],
                                        ..default()
                                    },
                                ),
                            ])
                            .with_justify(JustifyText::Right),
                            text_anchor: Anchor::BottomRight,
                            text_2d_bounds: Text2dBounds {
                                size: Vec2::new(
                                    player_displays_size[0].x - (player_displays_border[0] * 2.),
                                    player_displays_size[0].y,
                                ),
                                ..default()
                            },
                            transform: Transform {
                                translation: Vec3::new(
                                    player_displays_size[0].x - player_displays_border[0],
                                    0.,
                                    100.,
                                ),
                                ..default()
                            },
                            ..default()
                        },
                        PlayerScoreTag,
                    ));
                });

            builder
                .spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(player_displays_size[1]),
                            anchor: Anchor::BottomRight,
                            ..default()
                        },
                        transform: Transform {
                            translation: player_displays_pos[1].extend(0.),
                            ..default()
                        },
                        texture: display_sprite_handle.clone(),
                        ..default()
                    },
                    RenderLayers::Single(EntityLayer::OfficeLevelAccent),
                ))
                .with_children(|builder| {
                    builder.spawn((
                        Text2dBundle {
                            text: Text::from_sections([
                                TextSection::new(
                                    "Score: ",
                                    TextStyle {
                                        font_size: 20.0,
                                        color: player_colours[1],
                                        ..default()
                                    },
                                ),
                                TextSection::new(
                                    "0",
                                    TextStyle {
                                        font_size: 20.0,
                                        color: player_colours[1],
                                        ..default()
                                    },
                                ),
                            ])
                            .with_justify(JustifyText::Right),
                            text_anchor: Anchor::BottomRight,
                            text_2d_bounds: Text2dBounds {
                                size: Vec2::new(
                                    player_displays_size[1].x - (player_displays_border[1] * 2.),
                                    player_displays_size[1].y,
                                ),
                                ..default()
                            },
                            transform: Transform {
                                translation: Vec3::new(-player_displays_border[1], 0., 100.),
                                ..default()
                            },
                            ..default()
                        },
                        PlayerScoreTag,
                    ));
                });
        });

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(
                    app_settings.base_resolution.x,
                    app_settings.base_resolution.y,
                )),
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0., 0., 0.),
                ..default()
            },
            texture: asset_server.load(BACKGROUND_SPRITE),
            ..default()
        },
        RenderLayers::Single(EntityLayer::Background),
    ));

    let conveyor_walkway_pos = Vec2::new(
        0.,
        -((app_settings.base_resolution.y / 2.) - (conveyor_walkway_size.y / 2.)),
    );

    commands.spawn((
        Transform::from_translation(conveyor_walkway_pos.extend(0.)),
        Collider {
            size: conveyor_walkway_size,
        },
        RenderLayers::Single(EntityLayer::Debugging),
    ));
}
