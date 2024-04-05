use bevy::prelude::*;

use crate::{GameConfig, PlayerControls};

pub struct PlayerScoreData {
    pub score: f32,
    pub multiplier: f32,
    pub multiplier_decrement_freeze_timer: Timer,
}

#[derive(Resource)]
pub struct GameState {
    pub player_scores: [PlayerScoreData; 2],
    pub package_wave_timer: Timer,
    pub player_controls: [PlayerControls; 2],
}

#[derive(Component)]
pub struct TeamScoreTag;

#[derive(Component)]
pub struct PlayerScoreTag;

pub fn update_score_multipiers(
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
    game_config: Res<GameConfig>,
) {
    for player_data in &mut game_state.player_scores {
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
