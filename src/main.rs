use std::path::PathBuf;

use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    sprite::Anchor,
    text::{JustifyText, Text2dBounds},
    window::WindowResolution,
};

use enum_map::enum_map;

mod collision;
mod configuration;
mod conveyor;
mod game_mode;
mod package;
mod player;
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
            player_scores: enum_map! {
                PlayerIndex::Player1 => PlayerScoreData {
                    score: 0.,
                    multiplier: 1.,
                    multiplier_decrement_freeze_timer: Timer::from_seconds(2., TimerMode::Once),
                },
                PlayerIndex::Player2 => PlayerScoreData {
                    score: 0.,
                    multiplier: 1.,
                    multiplier_decrement_freeze_timer: Timer::from_seconds(2., TimerMode::Once),
                },
            },
            package_wave_timer: Timer::from_seconds(5., TimerMode::Once),
            player_controls: enum_map! {
                PlayerIndex::Player1 => PlayerControls {
                    pad: None,
                    state: enum_map! {
                        KeyAction::MoveUp => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::MoveDown => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::MoveLeft => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::MoveRight => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::Sprint => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::PickupOrThrow => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                    },
                },
                PlayerIndex::Player2 => PlayerControls {
                    pad: None,
                    state: enum_map! {
                        KeyAction::MoveUp => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::MoveDown => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::MoveLeft => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::MoveRight => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::Sprint => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                        KeyAction::PickupOrThrow => ButtonState {
                            pressed: false,
                            state_changed_this_frame: false,
                        },
                    },
                },
            },
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

    spawn_walls(
        &mut commands,
        &app_config,
        &game_config,
        incoming_belt_length,
    );

    let texture_pack = game_config.get_texture_pack();
    let display_sprite = texture_pack.choose_texture_for(TextureTarget::ScoreDisplay, None);
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
    let player_displays_size = enum_map! {
        PlayerIndex::Player1 => Vec2::new(
            game_config.supervisor_config.office_sprite_size.x as f32 * 0.5,
            24.,
        ),
        PlayerIndex::Player2 => Vec2::new(
            game_config.supervisor_config.office_sprite_size.x as f32 * 0.5,
            24.,
        ),
    };
    let player_displays_pos = enum_map! {
        PlayerIndex::Player1 => Vec2::new(
            -(app_config.base_resolution.x as f32 / 2.) + (player_displays_size[PlayerIndex::Player1].x * 0.5),
            12.,
        ),
        PlayerIndex::Player2 => Vec2::new(
            (app_config.base_resolution.x as f32 / 2.) - (player_displays_size[PlayerIndex::Player2].x * 0.5),
            12.,
        ),
    };

    let player_configs = &game_config.player_config.per_player;
    let team_colour = game_config.team_colour;
    let player_displays_border = enum_map! {
        PlayerIndex::Player1 => 6.,
        PlayerIndex::Player2 => 6.,
    };
    let supervisor_office_sprite =
        texture_pack.choose_texture_for(TextureTarget::SupervisorOffice, None);
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
                .spawn(make_display_sprite(
                    team_display_pos,
                    team_display_size,
                    Anchor::BottomCenter,
                    &display_sprite_handle,
                ))
                .with_children(|builder| {
                    builder.spawn((
                        make_score_text(
                            "Team Score: ",
                            team_colour,
                            team_display_size - Vec2::new(team_display_border * 2., 0.),
                            team_display_size.x / 2. - team_display_border,
                        ),
                        PlayerScoreTag::All,
                    ));
                });

            builder
                .spawn(make_display_sprite(
                    player_displays_pos[PlayerIndex::Player1],
                    player_displays_size[PlayerIndex::Player1],
                    Anchor::BottomLeft,
                    &display_sprite_handle,
                ))
                .with_children(|builder| {
                    builder.spawn((
                        make_score_text(
                            "Score: ",
                            player_configs[PlayerIndex::Player1].colour,
                            player_displays_size[PlayerIndex::Player1]
                                - Vec2::new(player_displays_border[PlayerIndex::Player1] * 2., 0.),
                            player_displays_size[PlayerIndex::Player1].x / 2.
                                - player_displays_border[PlayerIndex::Player1],
                        ),
                        PlayerScoreTag::Player(PlayerIndex::Player1),
                    ));
                });

            builder
                .spawn(make_display_sprite(
                    player_displays_pos[PlayerIndex::Player2],
                    player_displays_size[PlayerIndex::Player2],
                    Anchor::BottomRight,
                    &display_sprite_handle,
                ))
                .with_children(|builder| {
                    builder.spawn((
                        make_score_text(
                            "Score: ",
                            player_configs[PlayerIndex::Player2].colour,
                            player_displays_size[PlayerIndex::Player2]
                                - Vec2::new(player_displays_border[PlayerIndex::Player2] * 2., 0.),
                            -player_displays_border[PlayerIndex::Player2],
                        ),
                        PlayerScoreTag::Player(PlayerIndex::Player2),
                    ));
                });
        });

    let background_sprite = texture_pack.choose_texture_for(TextureTarget::Background, None);
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

fn spawn_walls(
    commands: &mut Commands,
    app_config: &Res<AppConfig>,
    game_config: &Res<GameConfig>,
    incoming_belt_length: f32,
) {
    fn make_wall(
        pos: Vec2,
        size: Vec2,
        anchor: Anchor,
    ) -> (SpriteBundle, Collider, WallTag, RenderLayers) {
        (
            SpriteBundle {
                sprite: Sprite {
                    anchor: anchor,
                    ..default()
                },
                transform: Transform::from_translation(pos.extend(0.)),
                ..default()
            },
            Collider { size: size },
            WallTag,
            RenderLayers::Single(EntityLayer::HeldObject),
        )
    }

    commands.spawn(make_wall(
        Vec2::new(app_config.base_resolution.x as f32 / 2., 0.),
        Vec2::new(10., app_config.base_resolution.y as f32),
        Anchor::CenterLeft,
    ));
    commands.spawn(make_wall(
        Vec2::new(-(app_config.base_resolution.x as f32) / 2., 0.),
        Vec2::new(10., app_config.base_resolution.y as f32),
        Anchor::CenterRight,
    ));
    commands.spawn(make_wall(
        Vec2::new(
            -game_config.conveyor_config.size.x + 10.,
            (incoming_belt_length / 2.)
                + (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
        ),
        Vec2::new(
            10.,
            game_config.supervisor_config.office_sprite_size.y as f32,
        ),
        Anchor::CenterLeft,
    ));
    commands.spawn(make_wall(
        Vec2::new(
            game_config.conveyor_config.size.x - 10.,
            (incoming_belt_length / 2.)
                + (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
        ),
        Vec2::new(
            10.,
            game_config.supervisor_config.office_sprite_size.y as f32,
        ),
        Anchor::CenterRight,
    ));
    commands.spawn(make_wall(
        Vec2::new(
            0.,
            (app_config.base_resolution.y as f32 / 2.)
                - (game_config.supervisor_config.office_sprite_size.y as f32 / 2.),
        ),
        Vec2::new(app_config.base_resolution.x as f32, 10.),
        Anchor::BottomCenter,
    ));
    commands.spawn(make_wall(
        Vec2::new(0., -(app_config.base_resolution.y as f32) / 2.),
        Vec2::new(app_config.base_resolution.x as f32, 10.),
        Anchor::TopCenter,
    ));
}

fn make_display_sprite(
    pos: Vec2,
    size: Vec2,
    anchor: Anchor,
    sprite_handle: &Handle<Image>,
) -> (SpriteBundle, RenderLayers) {
    (
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(size),
                anchor: anchor,
                ..default()
            },
            transform: Transform {
                translation: pos.extend(0.),
                ..default()
            },
            texture: sprite_handle.clone(),
            ..default()
        },
        RenderLayers::Single(EntityLayer::OfficeLevelAccent),
    )
}

fn make_score_text(score_text: &str, colour: Color, bounds: Vec2, x_pos: f32) -> Text2dBundle {
    Text2dBundle {
        text: Text::from_sections([
            TextSection::new(
                score_text,
                TextStyle {
                    font_size: 20.0,
                    color: colour.clone(),
                    ..default()
                },
            ),
            TextSection::new(
                "0",
                TextStyle {
                    font_size: 20.0,
                    color: colour,
                    ..default()
                },
            ),
        ])
        .with_justify(JustifyText::Right),
        text_anchor: Anchor::BottomRight,
        text_2d_bounds: Text2dBounds {
            size: bounds,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(x_pos, 0., 100.),
            ..default()
        },
        ..default()
    }
}
