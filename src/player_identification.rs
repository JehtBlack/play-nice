use bevy::render::color::Color;
use enum_map::Enum;

#[derive(Enum, PartialEq, Eq, Clone, Copy)]
pub enum PlayerIndex {
    Player1,
    Player2,
}

impl PlayerIndex {
    pub fn index(&self) -> usize {
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
