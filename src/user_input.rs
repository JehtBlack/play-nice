use bevy::{
    input::gamepad::{GamepadConnection, GamepadEvent},
    prelude::*,
};

use crate::GameState;

pub struct ButtonMapping {
    pub keyboard_key: KeyCode,
    pub gamepad_button: Option<GamepadButtonType>,
    pub gamepad_axis: Option<GamepadAxisType>,
}

pub struct ControlMapping {
    pub move_up: ButtonMapping,
    pub move_down: ButtonMapping,
    pub move_left: ButtonMapping,
    pub move_right: ButtonMapping,
    pub sprint: ButtonMapping,
    pub pickup_or_throw: ButtonMapping,
}

#[derive(Clone, Copy)]
pub struct ButtonState {
    pub pressed: bool,
    pub state_changed_this_frame: bool,
}

#[derive(Clone, Copy)]
pub struct ControlState {
    pub move_up: ButtonState,
    pub move_down: ButtonState,
    pub move_left: ButtonState,
    pub move_right: ButtonState,
    pub sprint: ButtonState,
    pub pickup_or_throw: ButtonState,
}

pub struct PlayerControls {
    pub pad: Option<Gamepad>,
    pub mapping: ControlMapping,
    pub state: ControlState,
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

pub fn update_controller_mappings(
    mut game_state: ResMut<GameState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    gamepad_buttons: Res<ButtonInput<GamepadButton>>,
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
