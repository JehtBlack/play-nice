use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use bevy::{
    ecs::system::Resource,
    input::{
        gamepad::{GamepadAxisType, GamepadButtonType},
        keyboard::KeyCode,
    },
    math::{UVec2, Vec2},
    render::color::Color,
};
use enum_map::{enum_map, Enum, EnumMap};
use serde::{Deserialize, Serialize};

use crate::random::*;

#[derive(Serialize, Deserialize)]
pub enum AxisDirection {
    Positive,
    Negative,
}

#[derive(Serialize, Deserialize)]
pub enum KeyBind {
    Key(KeyCode),
    ControllerButton(GamepadButtonType),
    ControllerAxis((GamepadAxisType, AxisDirection)),
}

#[derive(Serialize, Deserialize)]
pub struct KeyBindConfig {
    pub priamry: KeyBind,
    pub secondary: KeyBind,
}

#[derive(Enum, Serialize, Deserialize, Clone)]
pub enum KeyAction {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Sprint,
    PickupOrThrow,
}

#[derive(Debug, Enum, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum PlayerIndex {
    Player1,
    Player2,
}

#[derive(Enum, Deserialize, Serialize)]
pub enum TextureTarget {
    AllPlayers,
    Supervisor,
    Package,
    Conveyor,
    Background,
    SupervisorOffice,
    ScoreDisplay,
}

#[derive(Deserialize, Serialize)]
pub enum TextureValue {
    Only(SpriteSheetConfig),
    Choose(Vec<SpriteSheetConfig>),
}

#[derive(Deserialize, Serialize)]
pub struct SpriteSheetConfig {
    pub path: String,
    pub grid_dimensions: Option<UVec2>,
    pub cell_resolution: Option<UVec2>,
}

#[derive(Deserialize, Serialize)]
pub struct TexturePack {
    pub root: String,
    pub texture_map: EnumMap<TextureTarget, TextureValue>,
}

#[derive(Deserialize, Serialize)]
pub struct PerPlayerConfig {
    pub colour: Color,
    pub sprite_override: Option<TextureValue>,
    pub key_map: EnumMap<KeyAction, KeyBindConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct PlayerConfig {
    pub size: f32,
    pub move_speed: f32,
    pub sprint_move_modifier: f32,
    pub throw_power: f32,
    pub per_player: EnumMap<PlayerIndex, PerPlayerConfig>,
}

#[derive(Deserialize, Serialize)]
pub struct SupervisorConfig {
    pub size: f32,
    pub monitoring_y_pos: f32,
    pub office_sprite_size: UVec2,
}

#[derive(Deserialize, Serialize)]
pub struct ConveyorConfig {
    pub size: Vec2,
    pub speed: f32,
    pub border_size: f32,
    pub blinker_size: f32,
    pub blink_duration_seconds: f32,
}

#[derive(Deserialize, Serialize)]
pub struct PackageConfig {
    pub size: f32,
    pub base_score_value: f32,
}

#[derive(Deserialize, Serialize)]
pub struct ScoreConfig {
    pub multiplier_increase_per_package: f32,
    pub multiplier_decrease_per_second: f32,
}

#[derive(Resource, Deserialize, Serialize)]
pub struct AppConfig {
    pub base_resolution: UVec2,
    pub rng_seed: Option<u64>,
}

#[derive(Resource, Deserialize, Serialize)]
pub struct GameConfig {
    #[serde(default = "default_texture_pack_key")]
    pub selected_texture_pack: String,
    #[serde(default = "default_texture_pack")]
    pub texture_packs: HashMap<String, TexturePack>,
    #[serde(default = "default_team_colour")]
    pub team_colour: Color,
    #[serde(default)]
    pub player_config: PlayerConfig,
    #[serde(default)]
    pub supervisor_config: SupervisorConfig,
    #[serde(default)]
    pub conveyor_config: ConveyorConfig,
    #[serde(default)]
    pub package_config: PackageConfig,
    #[serde(default)]
    pub score_config: ScoreConfig,
    pub friction: f32,
}

#[derive(Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub game: GameConfig,
}

impl Default for TexturePack {
    fn default() -> Self {
        Self {
            root: "sprites".to_string(),
            texture_map: enum_map! {
                TextureTarget::AllPlayers => TextureValue::Choose(vec![
                    SpriteSheetConfig {
                        path: "player_skin_tone_a.png".to_string(),
                        grid_dimensions: Some(UVec2::new(4, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                    SpriteSheetConfig {
                        path: "player_skin_tone_b.png".to_string(),
                        grid_dimensions: Some(UVec2::new(4, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                    SpriteSheetConfig {
                        path: "player_skin_tone_c.png".to_string(),
                        grid_dimensions: Some(UVec2::new(4, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                    SpriteSheetConfig {
                        path: "player_skin_tone_d.png".to_string(),
                        grid_dimensions: Some(UVec2::new(4, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                ]),
                TextureTarget::Supervisor => TextureValue::Choose(vec![
                    SpriteSheetConfig {
                        path: "supervisor_skin_tone_a.png".to_string(),
                        grid_dimensions: Some(UVec2::new(2, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                    SpriteSheetConfig {
                        path: "supervisor_skin_tone_b.png".to_string(),
                        grid_dimensions: Some(UVec2::new(2, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                    SpriteSheetConfig {
                        path: "supervisor_skin_tone_c.png".to_string(),
                        grid_dimensions: Some(UVec2::new(2, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                    SpriteSheetConfig {
                        path: "supervisor_skin_tone_d.png".to_string(),
                        grid_dimensions: Some(UVec2::new(2, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    },
                ]),
                TextureTarget::Package => TextureValue::Only(SpriteSheetConfig {
                    path: "box.png".to_string(),
                    grid_dimensions: None,
                    cell_resolution: None,
                }),
                TextureTarget::Conveyor => TextureValue::Only(SpriteSheetConfig {
                    path: "conveyor.png".to_string(),
                    grid_dimensions: Some(UVec2::new(5, 1)),
                    cell_resolution: Some(UVec2::new(128, 128)),
                }),
                TextureTarget::Background => TextureValue::Only(SpriteSheetConfig {
                    path: "background.png".to_string(),
                    grid_dimensions: None,
                    cell_resolution: None,
                }),
                TextureTarget::SupervisorOffice => TextureValue::Only(SpriteSheetConfig {
                    path: "supervisor_office.png".to_string(),
                    grid_dimensions: None,
                    cell_resolution: None,
                }),
                TextureTarget::ScoreDisplay => TextureValue::Only(SpriteSheetConfig {
                    path: "display.png".to_string(),
                    grid_dimensions: None,
                    cell_resolution: None,
                })
            },
        }
    }
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            size: 30.,
            move_speed: 150.,
            sprint_move_modifier: 2.,
            throw_power: 15.,
            per_player: enum_map! {
                PlayerIndex::Player1 => PerPlayerConfig {
                    colour: Color::rgb_linear(1.0, 0.3, 0.3),
                    sprite_override: Some(TextureValue::Only(SpriteSheetConfig {
                        path: "sprites/custom_player.png".to_string(),
                        grid_dimensions: Some(UVec2::new(4, 1)),
                        cell_resolution: Some(UVec2::new(128, 128)),
                    })),
                    key_map: default_key_map_player_one(),
                },
                PlayerIndex::Player2 => PerPlayerConfig {
                    colour: Color::rgb_linear(0.3, 0.3, 1.6),
                    sprite_override: None,
                    key_map: default_key_map_player_two(),
                },
            },
        }
    }
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            size: 30.,
            monitoring_y_pos: 285.,
            office_sprite_size: UVec2::new(400, 150),
        }
    }
}

impl Default for ConveyorConfig {
    fn default() -> Self {
        Self {
            size: Vec2::new(128., 500.),
            speed: 100.,
            border_size: 14.,
            blinker_size: 20.,
            blink_duration_seconds: 0.1,
        }
    }
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            size: 30.,
            base_score_value: 5.,
        }
    }
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            multiplier_increase_per_package: 0.1,
            multiplier_decrease_per_second: 0.1,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            base_resolution: UVec2::new(1280, 720),
            rng_seed: Some(1000),
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            selected_texture_pack: "default".to_string(),
            texture_packs: maplit::hashmap! {
                "default".to_string() => TexturePack::default(),
            },
            team_colour: default_team_colour(),
            player_config: PlayerConfig::default(),
            supervisor_config: SupervisorConfig::default(),
            conveyor_config: ConveyorConfig::default(),
            package_config: PackageConfig::default(),
            score_config: ScoreConfig::default(),
            friction: 100.,
        }
    }
}

impl TextureValue {
    pub fn choose_texture(&self, rng: Option<&mut Rand>) -> &SpriteSheetConfig {
        match self {
            TextureValue::Only(config) => config,
            TextureValue::Choose(configs) => {
                let index = rng.map_or(0, |rng| rng.gen_range(0..configs.len()));
                &configs[index]
            }
        }
    }
}

impl TexturePack {
    pub fn choose_texture_for(
        &self,
        target: TextureTarget,
        rng: Option<&mut Rand>,
    ) -> &SpriteSheetConfig {
        self.texture_map[target].choose_texture(rng)
    }
}

impl GameConfig {
    pub fn get_texture_pack(&self) -> &TexturePack {
        self.texture_packs
            .get(&self.selected_texture_pack)
            .expect("Selected texture pack not found")
    }

    pub fn get_key_map(&self, player_index: PlayerIndex) -> &EnumMap<KeyAction, KeyBindConfig> {
        &self.player_config.per_player[player_index].key_map
    }
}

fn default_key_map_player_one() -> EnumMap<KeyAction, KeyBindConfig> {
    default_key_map(PlayerIndex::Player1)
}

fn default_key_map_player_two() -> EnumMap<KeyAction, KeyBindConfig> {
    default_key_map(PlayerIndex::Player2)
}

fn default_key_map(player_index: PlayerIndex) -> EnumMap<KeyAction, KeyBindConfig> {
    enum_map! {
        KeyAction::MoveUp => KeyBindConfig {
            priamry: KeyBind::Key(match player_index {
                PlayerIndex::Player1 => KeyCode::KeyW,
                PlayerIndex::Player2 => KeyCode::ArrowUp,
            }),
            secondary: KeyBind::ControllerAxis((GamepadAxisType::LeftStickY, AxisDirection::Positive)),
        },
        KeyAction::MoveDown => KeyBindConfig {
            priamry: KeyBind::Key(match player_index {
                PlayerIndex::Player1 => KeyCode::KeyS,
                PlayerIndex::Player2 => KeyCode::ArrowDown,
            }),
            secondary: KeyBind::ControllerAxis((GamepadAxisType::LeftStickY, AxisDirection::Negative)),
        },
        KeyAction::MoveLeft => KeyBindConfig {
            priamry: KeyBind::Key(match player_index {
                PlayerIndex::Player1 => KeyCode::KeyA,
                PlayerIndex::Player2 => KeyCode::ArrowLeft,
            }),
            secondary: KeyBind::ControllerAxis((GamepadAxisType::LeftStickX, AxisDirection::Negative)),
        },
        KeyAction::MoveRight => KeyBindConfig {
            priamry: KeyBind::Key(match player_index {
                PlayerIndex::Player1 => KeyCode::KeyD,
                PlayerIndex::Player2 => KeyCode::ArrowRight,
            }),
            secondary: KeyBind::ControllerAxis((GamepadAxisType::LeftStickX, AxisDirection::Positive)),
        },
        KeyAction::Sprint => KeyBindConfig {
            priamry: KeyBind::Key(match player_index {
                PlayerIndex::Player1 => KeyCode::ShiftLeft,
                PlayerIndex::Player2 => KeyCode::ShiftRight,
            }),
            secondary: KeyBind::ControllerAxis((GamepadAxisType::LeftZ, AxisDirection::Positive)),
        },
        KeyAction::PickupOrThrow => KeyBindConfig {
            priamry: KeyBind::Key(match player_index {
                PlayerIndex::Player1 => KeyCode::Space,
                PlayerIndex::Player2 => KeyCode::ControlRight,
            }),
            secondary: KeyBind::ControllerAxis((GamepadAxisType::RightZ, AxisDirection::Positive)),
        },
    }
}

fn default_texture_pack_key() -> String {
    "default".to_string()
}

fn default_texture_pack() -> HashMap<String, TexturePack> {
    maplit::hashmap! {
        default_texture_pack_key() => TexturePack::default(),
    }
}

fn default_team_colour() -> Color {
    Color::rgb_linear(0.6, 0.1, 0.6)
}

pub const CONFIG_FILENAME: &'static str = "play_nice.toml";

/// Searches for `filename` in `directory` and parent directories until found or root is reached.
pub fn find_config(directory: &Path, filename: &Path) -> anyhow::Result<PathBuf> {
    let candidate = directory.join(filename);

    match std::fs::metadata(&candidate) {
        Ok(metadata) => {
            if metadata.is_file() {
                return Ok(candidate);
            }
        }
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(anyhow::anyhow!(error));
            }
        }
    }

    if let Some(parent) = directory.parent() {
        find_config(parent, filename)
    } else {
        Err(anyhow::anyhow!("path not found",))
    }
}

pub fn read_config(config_path: Option<PathBuf>) -> anyhow::Result<Config> {
    let config_path = if let Some(path) = config_path {
        Ok(path.to_path_buf())
    } else {
        find_config(&std::env::current_dir()?, Path::new(CONFIG_FILENAME))
    };

    match config_path {
        Ok(config_path) => {
            let config_file = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&config_file)?;
            Ok(config)
        }
        Err(_) => {
            // error finding config file, create a default config and write out to file
            let default_config = Config::default();
            let default_config_str = toml::to_string_pretty(&default_config)?;
            let default_config_path = std::env::current_dir()?.join(CONFIG_FILENAME);
            std::fs::write(default_config_path, default_config_str)?;
            Ok(default_config)
        }
    }
}
