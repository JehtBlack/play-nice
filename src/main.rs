use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    sprite::Anchor,
    text::{JustifyText, Text2dBounds},
    window::WindowResolution,
};

mod collision;
mod configuration;
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

use collision::*;
use configuration::*;
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

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let config_path = dotenv::var("CONFIG_PATH").ok().map(|s| PathBuf::from(s));
    let config = read_config(config_path)?;

    let rng = Rand::new(&config.app.rng_seed);

    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(
                        config.app.base_resolution.x as f32,
                        config.app.base_resolution.y as f32,
                    )
                    .with_scale_factor_override(1.),
                    title: "Play Nice!".to_string(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(SpriteLayerPlugin::<RenderLayers>::default())
        .insert_resource(config.app)
        .insert_resource(config.game)
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
    app_config: Res<AppConfig>,
    game_config: Res<GameConfig>,
    mut rng: ResMut<Rand>,
) {
    // default projection has 0.1 near and 1000. far, but Camera2dBundle defaults to -1000. near and 1000. far
    // start with the bundle defaults and mutate the projection scaling mode
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::Fixed {
        width: app_config.base_resolution.x as f32,
        height: app_config.base_resolution.y as f32,
    };
    commands.spawn(camera_bundle);

    spawn_player(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(
            -(app_config.base_resolution.x as f32 / 2.)
                + game_config.conveyor_config.size.x
                + (game_config.player_config.size / 2.),
            0.,
            0.,
        ),
        PlayerIndex::Player1,
        &mut rng,
        &game_config,
    );

    spawn_player(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        Vec3::new(
            (app_config.base_resolution.x as f32 / 2.)
                - game_config.conveyor_config.size.x
                - (game_config.player_config.size / 2.),
            0.,
            0.,
        ),
        PlayerIndex::Player2,
        &mut rng,
        &game_config,
    );

    let conveyor_walkway_size = Vec2::new(
        game_config.conveyor_config.size.x * 2.,
        game_config.supervisor_config.office_sprite_size.y as f32,
    );
    let incoming_belt_length = app_config.base_resolution.y as f32
        - game_config.supervisor_config.office_sprite_size.y as f32
        - conveyor_walkway_size.y;
    let outgoing_belt_length = app_config.base_resolution.y as f32
        - game_config.supervisor_config.office_sprite_size.y as f32;
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        &game_config,
        Vec3::new(-game_config.conveyor_config.size.x / 2., 0., 0.),
        incoming_belt_length,
        PlayAreaAligment::Left,
        ConveyorLabelTag::Incoming,
    );
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        &game_config,
        Vec3::new(
            -(app_config.base_resolution.x as f32 / 2.) + (game_config.conveyor_config.size.x / 2.),
            -(app_config.base_resolution.y as f32 / 2.) + (outgoing_belt_length / 2.),
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
        &game_config,
        Vec3::new(game_config.conveyor_config.size.x / 2., 0., 0.),
        incoming_belt_length,
        PlayAreaAligment::Right,
        ConveyorLabelTag::Incoming,
    );
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        &game_config,
        Vec3::new(
            (app_config.base_resolution.x as f32 / 2.) - (game_config.conveyor_config.size.x / 2.),
            -(app_config.base_resolution.y as f32 / 2.) + (outgoing_belt_length / 2.),
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
        Vec3::new(0., game_config.supervisor_config.monitoring_y_pos, 0.),
        &mut rng,
        &game_config,
    );

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                anchor: Anchor::CenterLeft,
                ..default()
            },
            transform: Transform::from_translation(
                Vec2::new(app_config.base_resolution.x as f32 / 2., 0.).extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(10., app_config.base_resolution.y as f32),
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
                Vec2::new(-(app_config.base_resolution.x as f32) / 2., 0.).extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(10., app_config.base_resolution.y as f32),
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
                    -game_config.conveyor_config.size.x + 10.,
                    (incoming_belt_length / 2.)
                        + (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
                )
                .extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(
                10.,
                game_config.supervisor_config.office_sprite_size.y as f32,
            ),
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
                    game_config.conveyor_config.size.x - 10.,
                    (incoming_belt_length / 2.)
                        + (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
                )
                .extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(
                10.,
                game_config.supervisor_config.office_sprite_size.y as f32,
            ),
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
                    (app_config.base_resolution.y as f32 / 2.)
                        - (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
                )
                .extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(app_config.base_resolution.x as f32, 10.),
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
                Vec2::new(0., -(app_config.base_resolution.y as f32) / 2.).extend(0.),
            ),
            ..default()
        },
        Collider {
            size: Vec2::new(app_config.base_resolution.x as f32, 10.),
        },
        WallTag,
        RenderLayers::Single(EntityLayer::HeldObject),
    ));

    let texture_pack = game_config.get_texture_pack();
    let display_sprite = &texture_pack.score_display;
    let display_sprite_handle =
        asset_server.load(&format!("{}/{}", texture_pack.root, display_sprite.path));
    let team_display_size = Vec2::new(
        game_config.supervisor_config.office_sprite_size.x as f32 * 0.5,
        24.,
    );
    let team_display_pos = Vec2::new(
        0.,
        -(game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
    );
    let team_display_border: f32 = 6.;
    let player_displays_size = [
        Vec2::new(
            game_config.supervisor_config.office_sprite_size.x as f32 * 0.5,
            24.,
        ),
        Vec2::new(
            game_config.supervisor_config.office_sprite_size.x as f32 * 0.5,
            24.,
        ),
    ];
    let player_displays_pos = [
        Vec2::new(
            -(app_config.base_resolution.x as f32 / 2.) + (player_displays_size[0].x * 0.5),
            12.,
        ),
        Vec2::new(
            (app_config.base_resolution.x as f32 / 2.) - (player_displays_size[1].x * 0.5),
            12.,
        ),
    ];

    let player_colours = [PlayerIndex::Player1.into(), PlayerIndex::Player2.into()];
    let team_colour = Color::rgb_linear(0.6, 0.1, 0.6);
    let player_displays_border: [f32; 2] = [6., 6.];
    let supervisor_office_sprite = &texture_pack.supervisor_office;
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(
                        app_config.base_resolution.x as f32,
                        game_config.supervisor_config.office_sprite_size.y as f32,
                    )),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        0.,
                        (app_config.base_resolution.y as f32 / 2.)
                            - (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
                        0.,
                    ),
                    ..default()
                },
                texture: asset_server.load(&format!(
                    "{}/{}",
                    texture_pack.root, supervisor_office_sprite.path
                )),
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

    let background_sprite = &texture_pack.background;
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(
                    app_config.base_resolution.x as f32,
                    app_config.base_resolution.y as f32,
                )),
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(0., 0., 0.),
                ..default()
            },
            texture: asset_server
                .load(&format!("{}/{}", texture_pack.root, background_sprite.path)),
            ..default()
        },
        RenderLayers::Single(EntityLayer::Background),
    ));

    let conveyor_walkway_pos = Vec2::new(
        0.,
        -((app_config.base_resolution.y as f32 / 2.) - (conveyor_walkway_size.y / 2.)),
    );

    commands.spawn((
        Transform::from_translation(conveyor_walkway_pos.extend(0.)),
        Collider {
            size: conveyor_walkway_size,
        },
        RenderLayers::Single(EntityLayer::Debugging),
    ));
}
