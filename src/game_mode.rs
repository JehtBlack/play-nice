use bevy::prelude::*;
use enum_map::EnumMap;

use crate::{GameConfig, PlayerControls, PlayerIndex};

pub struct PlayerScoreData {
    pub score: f32,
    pub multiplier: f32,
    pub multiplier_decrement_freeze_timer: Timer,
}

#[derive(Resource)]
pub struct GameState {
    pub player_scores: EnumMap<PlayerIndex, PlayerScoreData>,
    pub package_wave_timer: Timer,
    pub player_controls: [PlayerControls; 2],
}

#[derive(Component)]
pub enum PlayerScoreTag {
    All,
    Player(PlayerIndex),
}



pub fn update_score_multipiers(
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    game_config: Res<GameConfig>,
) {
    for (_, player_data) in &mut game_state.player_scores {
        player_data
            .multiplier_decrement_freeze_timer
            .tick(time.delta());
        if player_data.multiplier_decrement_freeze_timer.finished() {
            player_data.multiplier = (player_data.multiplier
                - game_config.score_config.multiplier_decrease_per_second * time.delta_seconds())
            .clamp(1., f32::INFINITY);
        }
    }
}

pub fn update_scores(
    game_state: ResMut<GameState>,
    mut score_query: Query<(&mut Text, &PlayerScoreTag)>,
) {
    for (mut score, tag) in score_query.iter_mut() {
        match tag {
            PlayerScoreTag::All => {
                score.sections[1].value = (game_state
                    .player_scores
                    .iter()
                    .fold(0., |acc, (_, p)| acc + p.score)
                    .floor() as u64)
                    .to_string();
            }
            PlayerScoreTag::Player(player_index) => {
                score.sections[1].value = if game_state.player_scores[*player_index].multiplier > 1.
                {
                    format!(
                        "{} [x{:.1}]",
                        game_state.player_scores[*player_index].score as u64,
                        game_state.player_scores[*player_index].multiplier
                    )
                } else {
                    (game_state.player_scores[*player_index].score as u64).to_string()
                };
            }
        }
    }
}
