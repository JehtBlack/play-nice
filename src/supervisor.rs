use crate::{
    random::*, AnimationData, AppSettings, EntityLayer, FacingDirection, GameSettings, Player,
    RenderLayers, PLAYER_SIZE, PLAYER_SPRITE_SIZE, SUPERVISOR_SPRITES,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct Supervisor {
    pub field_of_view: f32,
    pub monitoring_timer: Timer,
    pub distracted_timer: Timer,
}

pub fn spawn_supervisor(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    supervisor_start_pos: Vec3,
    rng: &mut ResMut<Rand>,
) {
    let tone_sprite = rng.gen_range(0..SUPERVISOR_SPRITES.len());
    let texture_handle: Handle<Image> = asset_server.load(SUPERVISOR_SPRITES[tone_sprite]);
    let atlas_layout = TextureAtlasLayout::from_grid(PLAYER_SPRITE_SIZE, 2, 1, None, None);
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
            sprite: Sprite {
                custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
                ..default()
            },
            atlas: TextureAtlas {
                layout: texture_atlas_layouts.add(atlas_layout),
                index: animation_indices.start_frame,
            },
            texture: texture_handle,
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

pub fn update_supervisor(
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
            let t = supervisor.monitoring_timer.fraction() / 0.4;
            supervisor_transform.translation.y = supervisor_transform
                .translation
                .y
                .lerp(game_settings.supervisor_monitoring_y_pos, t.clamp(0., 1.));
            supervisor_anim_data.facing_direction = FacingDirection::Down;
        } else {
            // supervisor monitoring complete, "distract" them
            let t = supervisor.distracted_timer.fraction() / 0.4;
            supervisor_transform.translation.y = supervisor_transform
                .translation
                .y
                .lerp(supervisor_offscreen_distraction_pos, t.clamp(0., 1.));
            supervisor_anim_data.facing_direction = FacingDirection::Up;
        }
    }
}

pub fn check_supervisor_can_see_players(
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
