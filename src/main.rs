use std::collections::{hash_map::DefaultHasher, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use bevy::input::gamepad::{GamepadConnection, GamepadEvent};
use bevy::sprite::collide_aabb::Collision;
use bevy::{
    prelude::*,
    render::camera::ScalingMode,
    sprite::collide_aabb::collide,
    sprite::Anchor,
    text::{Text2dBounds, TextAlignment},
    window::WindowResolution,
};
use interpolation::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

mod sprite_render_layers;
use sprite_render_layers::*;

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

#[derive(PartialEq, Eq, Clone, Copy)]
enum PlayerIndex {
    Player1,
    Player2,
}

impl PlayerIndex {
    fn index(&self) -> usize {
        match self {
            PlayerIndex::Player1 => 0,
            PlayerIndex::Player2 => 1,
        }
    }
}

impl Into<usize> for PlayerIndex {
    fn into(self) -> usize {
        self.index()
    }
}

impl Into<Color> for PlayerIndex {
    fn into(self) -> Color {
        match self {
            PlayerIndex::Player1 => Color::rgb_linear(1.0, 0.3, 0.3),
            PlayerIndex::Player2 => Color::rgb_linear(0.3, 0.3, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
enum EntityLayer {
    Background,
    Debugging,
    Furniture,
    Object,
    Accent,
    Player,
    HeldObject,
    OfficeLevelFurniture,
    OfficeLevelAccent,
    SuperVisor,
}

#[derive(Debug, Clone, Component)]
enum RenderLayers {
    Single(EntityLayer),
    Multi(BTreeSet<EntityLayer>),
}

impl LayerIndex for RenderLayers {
    fn as_z_coordinate(&self) -> f32 {
        fn as_z_coordinate_internal(layer: &EntityLayer) -> f32 {
            match layer {
                EntityLayer::Background => -1.,
                EntityLayer::Debugging => 0.,
                EntityLayer::Furniture => 1.,
                EntityLayer::Object => 2.,
                EntityLayer::Accent => 3.,
                EntityLayer::Player => 20.,
                EntityLayer::HeldObject => 21.,
                EntityLayer::OfficeLevelFurniture => 22.,
                EntityLayer::OfficeLevelAccent => 23.,
                EntityLayer::SuperVisor => 24.,
            }
        }

        match self {
            RenderLayers::Single(layer) => as_z_coordinate_internal(layer),
            RenderLayers::Multi(layers) => layers
                .iter()
                .max_by(|a, b| as_z_coordinate_internal(a).total_cmp(&&as_z_coordinate_internal(b)))
                .map_or(0., |l| as_z_coordinate_internal(l)),
        }
    }
}

#[derive(Debug)]
enum FacingDirection {
    Up,
    Down,
    Left,
    Right,
}

impl FacingDirection {
    fn as_sprite_index(&self) -> usize {
        match self {
            FacingDirection::Up => 1,
            FacingDirection::Down => 0,
            FacingDirection::Left => 2,
            FacingDirection::Right => 3,
        }
    }

    fn as_vector(&self) -> Vec2 {
        match self {
            FacingDirection::Up => Vec2::new(0., 1.),
            FacingDirection::Down => Vec2::new(0., -1.),
            FacingDirection::Left => Vec2::new(-1., 0.),
            FacingDirection::Right => Vec2::new(1., 0.),
        }
    }
}

#[derive(Debug, Component)]
struct AnimationData {
    start_frame: usize,
    frame_count: usize,
    pause: bool,
    facing_direction: FacingDirection,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Component)]
struct Player {
    pub pickup_cooldown_timer: Timer,
    pub throw_timer: Timer,
    pub player_index: PlayerIndex,
}

#[derive(Component)]
struct Collider {
    pub size: Vec2,
}

#[derive(Event)]
struct CollisionEvent {
    pub collision: Collision,
    pub entity_a: Entity,
    pub entity_b: Entity,
}

#[derive(Resource)]
struct GameSettings {
    pub player_move_speed: f32,
    pub player_sprint_move_modifier: f32,
    pub supervisor_monitoring_y_pos: f32,
}
#[derive(Resource)]
struct AppSettings {
    pub base_resolution: Vec2,
    pub rng_seed: Option<u64>,
}

#[derive(Component)]
struct Package;

#[derive(Bundle)]
struct PackageBundle {
    sprite_bundle: SpriteBundle,
    package: Package,
    velocity: Velocity,
    collider: Collider,
    render_layers: RenderLayers,
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

#[derive(Component, PartialEq, Eq)]
enum ConveyorLabelTag {
    Incoming,
    Outgoing(PlayerIndex),
}

#[derive(Component)]
struct Conveyor {
    pub belt_region: Vec2,
    pub direction: f32,
    pub speed: f32,
    pub active_timer: Timer,
    pub idle_timer: Timer,

    pub package_count: usize,
}

#[derive(Component)]
struct Blinker {
    pub blink_timer: Timer,
    pub active_colour: Color,
    pub inactive_colour: Color,
    pub readying_colour: Color,
}

enum PlayAreaAligment {
    Left,
    Right,
}

#[derive(Component)]
struct Supervisor {
    pub field_of_view: f32,
    pub monitoring_timer: Timer,
    pub distracted_timer: Timer,
}

impl PlayAreaAligment {
    fn get_blink_position_modifier(&self, conveyor_tag: &ConveyorLabelTag) -> f32 {
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

#[derive(Resource, Deref, DerefMut)]
struct Rand(ChaCha8Rng);

impl Rand {
    fn new(seed: &Option<u64>) -> Self {
        Self(seed.map_or(ChaCha8Rng::from_entropy(), |seed| {
            ChaCha8Rng::seed_from_u64(seed)
        }))
    }
}

struct PlayerScoreData {
    pub score: f32,
    pub multiplier: f32,
    pub multiplier_decrement_freeze_timer: Timer,
}

struct ButtonMapping {
    pub keyboard_key: KeyCode,
    pub gamepad_button: Option<GamepadButtonType>,
    pub gamepad_axis: Option<GamepadAxisType>,
}

struct ControlMapping {
    pub move_up: ButtonMapping,
    pub move_down: ButtonMapping,
    pub move_left: ButtonMapping,
    pub move_right: ButtonMapping,
    pub sprint: ButtonMapping,
    pub pickup_or_throw: ButtonMapping,
}

#[derive(Clone, Copy)]
struct ButtonState {
    pub pressed: bool,
    pub state_changed_this_frame: bool,
}

impl ButtonState {
    fn pressed(&self) -> bool {
        self.pressed
    }
    fn released(&self) -> bool {
        !self.pressed
    }
    fn just_pressed(&self) -> bool {
        self.pressed && self.state_changed_this_frame
    }
    fn just_released(&self) -> bool {
        !self.pressed && self.state_changed_this_frame
    }
}

#[derive(Clone, Copy)]
struct ControlState {
    pub move_up: ButtonState,
    pub move_down: ButtonState,
    pub move_left: ButtonState,
    pub move_right: ButtonState,
    pub sprint: ButtonState,
    pub pickup_or_throw: ButtonState,
}

struct PlayerControls {
    pad: Option<Gamepad>,
    mapping: ControlMapping,
    state: ControlState,
}

#[derive(Resource)]
struct GameState {
    pub player_scores: [PlayerScoreData; 2],
    pub package_wave_timer: Timer,
    pub player_controls: [PlayerControls; 2],
}

#[derive(Component)]
struct TeamScoreTag;

#[derive(Component)]
struct PlayerScoreTag;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct WallTag;

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
                            keyboard_key: KeyCode::W,
                            gamepad_button: Some(GamepadButtonType::DPadUp),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_down: ButtonMapping {
                            keyboard_key: KeyCode::S,
                            gamepad_button: Some(GamepadButtonType::DPadDown),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_left: ButtonMapping {
                            keyboard_key: KeyCode::A,
                            gamepad_button: Some(GamepadButtonType::DPadLeft),
                            gamepad_axis: Some(GamepadAxisType::LeftStickX),
                        },
                        move_right: ButtonMapping {
                            keyboard_key: KeyCode::D,
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
                            keyboard_key: KeyCode::Up,
                            gamepad_button: Some(GamepadButtonType::DPadUp),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_down: ButtonMapping {
                            keyboard_key: KeyCode::Down,
                            gamepad_button: Some(GamepadButtonType::DPadDown),
                            gamepad_axis: Some(GamepadAxisType::LeftStickY),
                        },
                        move_left: ButtonMapping {
                            keyboard_key: KeyCode::Left,
                            gamepad_button: Some(GamepadButtonType::DPadLeft),
                            gamepad_axis: Some(GamepadAxisType::LeftStickX),
                        },
                        move_right: ButtonMapping {
                            keyboard_key: KeyCode::Right,
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
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
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
        &mut texture_atlases,
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
        &mut texture_atlases,
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
        &mut texture_atlases,
        Vec3::new(-CONVEYOR_SIZE.x / 2., 0., 0.),
        incoming_belt_length,
        PlayAreaAligment::Left,
        ConveyorLabelTag::Incoming,
    );
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlases,
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
        &mut texture_atlases,
        Vec3::new(CONVEYOR_SIZE.x / 2., 0., 0.),
        incoming_belt_length,
        PlayAreaAligment::Right,
        ConveyorLabelTag::Incoming,
    );
    spawn_conveyor(
        &mut commands,
        &asset_server,
        &mut texture_atlases,
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
        &mut texture_atlases,
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
                            .with_alignment(TextAlignment::Right),
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
                            .with_alignment(TextAlignment::Right),
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
                            .with_alignment(TextAlignment::Right),
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

fn gamepad_connected(
    mut game_state: ResMut<GameState>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.read() {
        match event {
            GamepadEvent::Connection(connection_event) => match connection_event.connection {
                GamepadConnection::Connected(_) => {
                    if let Some(player_control) = game_state
                        .player_controls
                        .iter_mut()
                        .find(|player_control| player_control.pad.is_none())
                    {
                        player_control.pad = Some(connection_event.gamepad);
                    }
                }
                GamepadConnection::Disconnected => {
                    if let Some(player_control) =
                        game_state
                            .player_controls
                            .iter_mut()
                            .find(|player_control| {
                                player_control
                                    .pad
                                    .map_or(false, |p| p.id == connection_event.gamepad.id)
                            })
                    {
                        player_control.pad = None;
                    }
                }
            },
            _ => {}
        }
    }
}

fn update_controller_mappings(
    mut game_state: ResMut<GameState>,
    keyboard_input: Res<Input<KeyCode>>,
    gamepad_buttons: Res<Input<GamepadButton>>,
    gamepad_axes: Res<Axis<GamepadAxis>>,
) {
    const GAMEPAD_AXIS_THRESHOLD: f32 = 0.5;

    for player_control in game_state.player_controls.iter_mut() {
        let prev_control_state = player_control.state.clone();
        let mut new_control_state = ControlState {
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
        };

        if let Some(pad) = player_control.pad {
            if let Some(axis) = player_control.mapping.move_up.gamepad_axis {
                new_control_state.move_up.pressed |= gamepad_axes
                    .get(GamepadAxis {
                        gamepad: pad,
                        axis_type: axis,
                    })
                    .map_or(false, |v| v > GAMEPAD_AXIS_THRESHOLD);
            }
            if let Some(axis) = player_control.mapping.move_down.gamepad_axis {
                new_control_state.move_down.pressed |= gamepad_axes
                    .get(GamepadAxis {
                        gamepad: pad,
                        axis_type: axis,
                    })
                    .map_or(false, |v| v < -GAMEPAD_AXIS_THRESHOLD);
            }
            if let Some(axis) = player_control.mapping.move_left.gamepad_axis {
                new_control_state.move_left.pressed |= gamepad_axes
                    .get(GamepadAxis {
                        gamepad: pad,
                        axis_type: axis,
                    })
                    .map_or(false, |v| v < -GAMEPAD_AXIS_THRESHOLD);
            }
            if let Some(axis) = player_control.mapping.move_right.gamepad_axis {
                new_control_state.move_right.pressed |= gamepad_axes
                    .get(GamepadAxis {
                        gamepad: pad,
                        axis_type: axis,
                    })
                    .map_or(false, |v| v > GAMEPAD_AXIS_THRESHOLD);
            }
            if let Some(axis) = player_control.mapping.sprint.gamepad_axis {
                new_control_state.sprint.pressed |= gamepad_axes
                    .get(GamepadAxis {
                        gamepad: pad,
                        axis_type: axis,
                    })
                    .map_or(false, |v| v > GAMEPAD_AXIS_THRESHOLD);
            }
            if let Some(axis) = player_control.mapping.pickup_or_throw.gamepad_axis {
                new_control_state.pickup_or_throw.pressed |= gamepad_axes
                    .get(GamepadAxis {
                        gamepad: pad,
                        axis_type: axis,
                    })
                    .map_or(false, |v| v > GAMEPAD_AXIS_THRESHOLD);
            }

            if let Some(button) = player_control.mapping.move_up.gamepad_button {
                new_control_state.move_up.pressed |= gamepad_buttons.pressed(GamepadButton {
                    gamepad: pad,
                    button_type: button,
                });
            }
            if let Some(button) = player_control.mapping.move_down.gamepad_button {
                new_control_state.move_down.pressed |= gamepad_buttons.pressed(GamepadButton {
                    gamepad: pad,
                    button_type: button,
                });
            }
            if let Some(button) = player_control.mapping.move_left.gamepad_button {
                new_control_state.move_left.pressed |= gamepad_buttons.pressed(GamepadButton {
                    gamepad: pad,
                    button_type: button,
                });
            }
            if let Some(button) = player_control.mapping.move_right.gamepad_button {
                new_control_state.move_right.pressed |= gamepad_buttons.pressed(GamepadButton {
                    gamepad: pad,
                    button_type: button,
                });
            }
            if let Some(button) = player_control.mapping.sprint.gamepad_button {
                new_control_state.sprint.pressed |= gamepad_buttons.pressed(GamepadButton {
                    gamepad: pad,
                    button_type: button,
                });
            }
            if let Some(button) = player_control.mapping.pickup_or_throw.gamepad_button {
                new_control_state.pickup_or_throw.pressed |=
                    gamepad_buttons.pressed(GamepadButton {
                        gamepad: pad,
                        button_type: button,
                    });
            }
        }

        if keyboard_input.pressed(player_control.mapping.move_up.keyboard_key) {
            new_control_state.move_up.pressed |= true;
        }
        if keyboard_input.pressed(player_control.mapping.move_down.keyboard_key) {
            new_control_state.move_down.pressed |= true;
        }
        if keyboard_input.pressed(player_control.mapping.move_left.keyboard_key) {
            new_control_state.move_left.pressed |= true;
        }
        if keyboard_input.pressed(player_control.mapping.move_right.keyboard_key) {
            new_control_state.move_right.pressed |= true;
        }
        if keyboard_input.pressed(player_control.mapping.sprint.keyboard_key) {
            new_control_state.sprint.pressed |= true;
        }
        if keyboard_input.pressed(player_control.mapping.pickup_or_throw.keyboard_key) {
            new_control_state.pickup_or_throw.pressed |= true;
        }

        new_control_state.move_up.state_changed_this_frame =
            new_control_state.move_up.pressed != prev_control_state.move_up.pressed;
        new_control_state.move_down.state_changed_this_frame =
            new_control_state.move_down.pressed != prev_control_state.move_down.pressed;
        new_control_state.move_left.state_changed_this_frame =
            new_control_state.move_left.pressed != prev_control_state.move_left.pressed;
        new_control_state.move_right.state_changed_this_frame =
            new_control_state.move_right.pressed != prev_control_state.move_right.pressed;
        new_control_state.sprint.state_changed_this_frame =
            new_control_state.sprint.pressed != prev_control_state.sprint.pressed;
        new_control_state.pickup_or_throw.state_changed_this_frame =
            new_control_state.pickup_or_throw.pressed != prev_control_state.pickup_or_throw.pressed;

        player_control.state = new_control_state;
    }
}

fn spawn_player(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    player_pos: Vec3,
    player_index: PlayerIndex,
    rng: &mut ResMut<Rand>,
) {
    let tone_sprite = rng.gen_range(0..PLAYER_SPRITES.len());
    let texture_handle: Handle<Image> = asset_server.load(PLAYER_SPRITES[tone_sprite]);
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, PLAYER_SPRITE_SIZE, 4, 1, None, None);
    let animation_indices = AnimationData {
        start_frame: 0,
        frame_count: 4,
        pause: true,
        facing_direction: FacingDirection::Down,
    };
    commands
        .spawn((
            SpriteSheetBundle {
                texture_atlas: texture_atlases.add(texture_atlas),
                sprite: TextureAtlasSprite {
                    custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
                    index: animation_indices.start_frame,
                    ..default()
                },
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

fn spawn_supervisor(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    supervisor_start_pos: Vec3,
    rng: &mut ResMut<Rand>,
) {
    let tone_sprite = rng.gen_range(0..SUPERVISOR_SPRITES.len());
    let texture_handle: Handle<Image> = asset_server.load(SUPERVISOR_SPRITES[tone_sprite]);
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, PLAYER_SPRITE_SIZE, 2, 1, None, None);
    let animation_indices = AnimationData {
        start_frame: 0,
        frame_count: 2,
        pause: true,
        facing_direction: FacingDirection::Down,
    };
    let monitoring_timer = Timer::from_seconds(5., TimerMode::Once);
    let mut distracted_timer = Timer::from_seconds(5., TimerMode::Once);
    distracted_timer.pause();
    commands.spawn((
        SpriteSheetBundle {
            texture_atlas: texture_atlases.add(texture_atlas),
            sprite: TextureAtlasSprite {
                custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
                index: animation_indices.start_frame,
                ..default()
            },
            transform: Transform {
                translation: supervisor_start_pos,
                ..default()
            },
            ..default()
        },
        Supervisor {
            field_of_view: 90.,
            monitoring_timer: monitoring_timer,
            distracted_timer: distracted_timer,
        },
        RenderLayers::Single(EntityLayer::SuperVisor),
        animation_indices,
    ));
}

fn spawn_package(commands: &mut Commands, asset_server: &Res<AssetServer>, package_pos: Vec3) {
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

fn spawn_package_wave(
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

fn spawn_conveyor(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
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
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle,
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
                texture_atlas: texture_atlases.add(texture_atlas),
                sprite: TextureAtlasSprite {
                    custom_size: Some(Vec2::new(CONVEYOR_SIZE.x, conveyor_belt_length)),
                    index: animation_indices.start_frame,
                    ..default()
                },
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

fn move_player(
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

fn animate_sprite_maps(
    time: Res<Time>,
    mut sprite_map_query: Query<(&AnimationData, &mut AnimationTimer, &mut TextureAtlasSprite)>,
) {
    for (anim_data, mut timer, mut sprite_map) in sprite_map_query
        .iter_mut()
        .filter(|(anim_data, _, _)| !anim_data.pause)
    {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            sprite_map.index = (sprite_map.index + 1) % anim_data.frame_count;
        }
    }
}

fn select_sprite_facing_index(
    mut query: Query<(&AnimationData, &mut TextureAtlasSprite), Without<Conveyor>>,
) {
    for (anim_data, mut sprite_map) in &mut query {
        sprite_map.index = anim_data.start_frame + anim_data.facing_direction.as_sprite_index();
    }
}

fn update_conveyors(
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
                    && conveyor_info.idle_timer.percent_left() <= 0.25
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

fn check_for_delivered_packages(
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

fn check_for_collisions(
    collider_query: Query<(Entity, &Collider, &GlobalTransform, Option<&Parent>)>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for (entity_a, collider_a, transform_a, _parent_a) in collider_query.iter() {
        for (entity_b, collider_b, transform_b, _parent_b) in collider_query.iter() {
            if entity_a == entity_b {
                continue;
            }

            let collision = collide(
                transform_a.translation(),
                collider_a.size,
                transform_b.translation(),
                collider_b.size,
            );

            if let Some(collision) = collision {
                collision_events.send(CollisionEvent {
                    collision: collision,
                    entity_a: entity_a,
                    entity_b: entity_b,
                });
            }
        }
    }
}

fn check_supervisor_can_see_players(
    supervisor_query: Query<(&Transform, &Supervisor)>,
    player_query: Query<&Transform, With<Player>>,
) {
    for (supervisor_transform, supervisor) in supervisor_query
        .iter()
        .filter(|(_, s)| !s.monitoring_timer.finished())
    {
        for player_transform in &mut player_query.iter() {
            let player_pos = player_transform.translation;
            let supervisor_pos = supervisor_transform.translation;
            let supervisor_facing = supervisor_transform.up();
            let supervisor_to_player = player_pos - supervisor_pos;
            let supervisor_to_player_angle = supervisor_facing.angle_between(supervisor_to_player);
            if supervisor_to_player_angle < (supervisor.field_of_view / 2.) {
                // player is in the supervisor's field of view
            }
        }
    }
}

fn update_supervisor(
    mut supervisor_query: Query<(&mut Transform, &mut AnimationData, &mut Supervisor)>,
    time: Res<Time>,
    app_settings: Res<AppSettings>,
    game_settings: Res<GameSettings>,
) {
    let supervisor_offscreen_distraction_pos =
        (app_settings.base_resolution.y / 2.) + (PLAYER_SIZE / 2.);

    for (mut supervisor_transform, mut supervisor_anim_data, mut supervisor) in
        &mut supervisor_query
    {
        supervisor.monitoring_timer.tick(time.delta());
        supervisor.distracted_timer.tick(time.delta());
        if supervisor.monitoring_timer.just_finished() {
            // supervisor is now distracted
            supervisor.monitoring_timer.pause();
            supervisor.distracted_timer.reset();
            supervisor.distracted_timer.unpause();
        }
        if supervisor.distracted_timer.just_finished() {
            // supervisor is now monitoring
            supervisor.distracted_timer.pause();
            supervisor.monitoring_timer.reset();
            supervisor.monitoring_timer.unpause();
        }

        let monitoring = !supervisor.monitoring_timer.finished();
        if monitoring {
            // supervisor "distraction" complete, return to monitoring
            let t = supervisor.monitoring_timer.percent() / 0.4;
            supervisor_transform.translation.y = supervisor_transform
                .translation
                .y
                .lerp(&game_settings.supervisor_monitoring_y_pos, &t.clamp(0., 1.));
            supervisor_anim_data.facing_direction = FacingDirection::Down;
        } else {
            // supervisor monitoring complete, "distract" them
            let t = supervisor.distracted_timer.percent() / 0.4;
            supervisor_transform.translation.y = supervisor_transform
                .translation
                .y
                .lerp(&supervisor_offscreen_distraction_pos, &t.clamp(0., 1.));
            supervisor_anim_data.facing_direction = FacingDirection::Up;
        }
    }
}

fn pickup_package(
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

fn react_to_basic_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut query: Query<(Entity, &mut Transform, &Collider, Option<&Parent>)>,
    package_query: Query<
        (Entity, Option<&Parent>),
        (With<Package>, Without<Conveyor>, Without<WallTag>),
    >,
    conveyor_query: Query<Entity, (With<Conveyor>, Without<Package>, Without<WallTag>)>,
    wall_query: Query<Entity, (With<WallTag>, Without<Conveyor>, Without<Package>)>,
) {
    // stop the players or packages going where they shouldn't
    // only exception will be a package that is on an outgoing conveyor
    // which can leave the bottom of the screen

    for event in collision_events.read() {
        if let Some(
            [(entity_a, mut transform_a, collider_a, parent_a), (_, transform_b, collider_b, parent_b)],
        ) = query.get_many_mut([event.entity_a, event.entity_b]).ok()
        {
            if parent_a.is_some() || parent_b.is_some() {
                // don't calculate collisions with any child entities
                continue;
            }

            if conveyor_query.get(entity_a).is_ok() || wall_query.get(entity_a).is_ok() {
                // conveyors and walls are immovable objects
                continue;
            }

            if let Some((_, package_parent)) = package_query.get(event.entity_a).ok() {
                if package_parent.is_some() {
                    // don't move packages that are being held
                    continue;
                }

                if let Some(_) = conveyor_query.get(event.entity_b).ok() {
                    // no need to resolve a conveyor/package collision here
                    continue;
                }
            }

            match event.collision {
                Collision::Left => {
                    transform_a.translation.x = transform_b.translation.x
                        - (collider_b.size.x / 2.)
                        - (collider_a.size.x / 2.);
                }
                Collision::Right => {
                    transform_a.translation.x = transform_b.translation.x
                        + (collider_b.size.x / 2.)
                        + (collider_a.size.x / 2.);
                }
                Collision::Top => {
                    transform_a.translation.y = transform_b.translation.y
                        + (collider_b.size.y / 2.)
                        + (collider_a.size.y / 2.);
                }
                Collision::Bottom => {
                    transform_a.translation.y = transform_b.translation.y
                        - (collider_b.size.y / 2.)
                        - (collider_a.size.y / 2.);
                }
                Collision::Inside => {}
            }
        }
    }
}

fn throw_package(
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
            let throw_distance = player_info.throw_timer.percent() * THROW_POWER;

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

fn player_charge_throw(
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

fn update_velocities(mut velocity_query: Query<(&mut Transform, &mut Velocity)>, time: Res<Time>) {
    for (mut transform, mut velocity) in &mut velocity_query {
        transform.translation += velocity.0.extend(0.) * time.delta_seconds();

        let norm = velocity.0.normalize_or_zero();
        let deceleration_due_to_friction = norm.abs() * FRICTION * time.delta_seconds();
        velocity.0 = velocity.0.signum()
            * (velocity.0.abs() - deceleration_due_to_friction).clamp(Vec2::ZERO, Vec2::INFINITY);
    }
}

fn calculate_attach_point_on_conveyor(
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

fn collect_packages_on_outgoing_conveyors(
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

fn clear_frame_collisions(mut collision_events: EventReader<CollisionEvent>) {
    collision_events.clear();
}

fn update_score_multipiers(mut game_state: ResMut<GameState>, time: Res<Time>) {
    for player_data in &mut game_state.player_scores {
        player_data
            .multiplier_decrement_freeze_timer
            .tick(time.delta());
        if player_data.multiplier_decrement_freeze_timer.finished() {
            player_data.multiplier = (player_data.multiplier
                - MULTIPLIER_DECREASE_PER_SECOND * time.delta_seconds())
            .clamp(1., f32::INFINITY);
        }
    }
}

fn update_scores(
    game_state: ResMut<GameState>,
    mut player_query: Query<&mut Text, (With<PlayerScoreTag>, Without<TeamScoreTag>)>,
    mut team_query: Query<&mut Text, (With<TeamScoreTag>, Without<PlayerScoreTag>)>,
) {
    for (i, mut player_score) in player_query.iter_mut().enumerate() {
        player_score.sections[1].value = if game_state.player_scores[i].multiplier > 1. {
            format!(
                "{} [x{:.1}]",
                game_state.player_scores[i].score as u64, game_state.player_scores[i].multiplier
            )
        } else {
            (game_state.player_scores[i].score as u64).to_string()
        };
    }

    for mut team_score in &mut team_query {
        team_score.sections[1].value = (game_state
            .player_scores
            .iter()
            .fold(0., |acc, p| acc + p.score)
            .floor() as u64)
            .to_string();
    }
}
