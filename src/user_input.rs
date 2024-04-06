use crate::{GameConfig, GameState, KeyAction, KeyBind};
use bevy::{
    input::gamepad::{GamepadConnection, GamepadEvent},
    prelude::*,
};
use enum_map::{enum_map, EnumMap};

#[derive(Clone, Copy)]
pub struct ButtonState {
    pub pressed: bool,
    pub state_changed_this_frame: bool,
}

pub struct PlayerControls {
    pub pad: Option<Gamepad>,
    pub state: EnumMap<KeyAction, ButtonState>,
}

impl ButtonState {
    pub fn pressed(&self) -> bool {
        self.pressed
    }

    pub fn released(&self) -> bool {
        !self.pressed
    }

    pub fn just_pressed(&self) -> bool {
        self.pressed && self.state_changed_this_frame
    }

    pub fn just_released(&self) -> bool {
        !self.pressed && self.state_changed_this_frame
    }
}

pub fn gamepad_connected(
    mut game_state: ResMut<GameState>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.read() {
        match event {
            GamepadEvent::Connection(connection_event) => match connection_event.connection {
                GamepadConnection::Connected(_) => {
                    if let Some((_, player_control)) = game_state
                        .player_controls
                        .iter_mut()
                        .find(|(_, player_control)| player_control.pad.is_none())
                    {
                        player_control.pad = Some(connection_event.gamepad);
                    }
                }
                GamepadConnection::Disconnected => {
                    if let Some((_, player_control)) =
                        game_state
                            .player_controls
                            .iter_mut()
                            .find(|(_, player_control)| {
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

pub fn update_controller_mappings(
    mut game_state: ResMut<GameState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepad_buttons: Res<ButtonInput<GamepadButton>>,
    gamepad_axes: Res<Axis<GamepadAxis>>,
    game_config: Res<GameConfig>,
) {
    const GAMEPAD_AXIS_THRESHOLD: f32 = 0.5;

    for (player_index, player_control) in game_state.player_controls.iter_mut() {
        fn write_button_state(
            keybind: &KeyBind,
            button_state: &mut ButtonState,
            keyboard_input: &Res<ButtonInput<KeyCode>>,
            gamepad_buttons: &Res<ButtonInput<GamepadButton>>,
            gamepad_axes: &Res<Axis<GamepadAxis>>,
            pad: Option<Gamepad>,
        ) {
            match keybind {
                crate::KeyBind::Key(key_code) => {
                    button_state.pressed |= keyboard_input.pressed(*key_code);
                }
                crate::KeyBind::ControllerButton(pad_button) => {
                    if let Some(pad) = pad {
                        button_state.pressed |= gamepad_buttons.pressed(GamepadButton {
                            gamepad: pad,
                            button_type: *pad_button,
                        });
                    }
                }
                crate::KeyBind::ControllerAxis((pad_axis, axis_direction)) => {
                    if let Some(pad) = pad {
                        button_state.pressed |= gamepad_axes
                            .get(GamepadAxis {
                                gamepad: pad,
                                axis_type: *pad_axis,
                            })
                            .map_or(false, |v| match axis_direction {
                                crate::AxisDirection::Positive => v > GAMEPAD_AXIS_THRESHOLD,
                                crate::AxisDirection::Negative => v < -GAMEPAD_AXIS_THRESHOLD,
                            });
                    }
                }
            }
        }

        let prev_control_state = player_control.state.clone();
        let mut new_control_state = enum_map! {
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
        };

        let key_mapping = game_config.get_key_map(player_index);
        let pad = player_control.pad;

        for (key_action, key_bind) in key_mapping {
            let new_button_state = &mut new_control_state[key_action.clone()];
            write_button_state(
                &key_bind.priamry,
                new_button_state,
                &keyboard_input,
                &gamepad_buttons,
                &gamepad_axes,
                pad,
            );

            write_button_state(
                &key_bind.secondary,
                new_button_state,
                &keyboard_input,
                &gamepad_buttons,
                &gamepad_axes,
                pad,
            );

            new_button_state.state_changed_this_frame =
                new_button_state.pressed != prev_control_state[key_action].pressed;
        }

        player_control.state = new_control_state;
    }
}
