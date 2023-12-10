// reimplementation of https://github.com/deifactor/extol_sprite_layer because I'm using bevy 0.12.0
use bevy::{
    prelude::*,
    render::{Extract, RenderApp},
    sprite::SpriteSystem,
    sprite::{ExtractedSprite, ExtractedSprites},
};
use ordered_float::OrderedFloat;
use rayon::slice::ParallelSliceMut;
use std::{cmp::Reverse, collections::HashMap, marker::PhantomData};

pub struct SpriteLayerPlugin<Layer> {
    _phantom: PhantomData<Layer>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, SystemSet)]
struct SpriteLayerSet;

#[derive(Debug, Resource, Reflect)]
pub struct SpriteLayerOptions {
    pub y_sort: bool,
}

pub trait LayerIndex: Component + Send + Sync + 'static {
    fn as_z_coordinate(&self) -> f32;
}

impl Default for SpriteLayerOptions {
    fn default() -> Self {
        Self { y_sort: true }
    }
}

impl<Layer> Default for SpriteLayerPlugin<Layer> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<Layer> Plugin for SpriteLayerPlugin<Layer>
where
    Layer: LayerIndex,
{
    fn build(&self, app: &mut App) {
        app.init_resource::<SpriteLayerOptions>();
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            ExtractSchedule,
            update_sprite_z_coordinate::<Layer>
                .in_set(SpriteSystem::ExtractSprites)
                .in_set(SpriteLayerSet)
                .after(bevy::sprite::extract_sprites)
                .before(bevy::sprite::queue_sprites),
        );
    }
}

fn update_sprite_z_coordinate<Layer: LayerIndex>(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    options: Extract<Res<SpriteLayerOptions>>,
    transform_query: Extract<Query<(Entity, &GlobalTransform), With<Layer>>>,
    layer_query: Extract<Query<&Layer>>,
) {
    if options.y_sort {
        let z_index_map = map_z_indices(transform_query, layer_query);
        for (sprite_entity, sprite) in extracted_sprites.sprites.iter_mut() {
            if let Some(z) = z_index_map.get(&sprite_entity) {
                set_sprite_coordinate(sprite, *z);
            }
        }
    } else {
        for (sprite_entity, sprite) in extracted_sprites.sprites.iter_mut() {
            if let Ok(layer) = layer_query.get(*sprite_entity) {
                set_sprite_coordinate(sprite, layer.as_z_coordinate());
            }
        }
    }
}

fn set_sprite_coordinate(sprite: &mut ExtractedSprite, z: f32) {
    if sprite.transform.translation().z != 0.0 {
        warn!(
            "Entity {:?} has a LabelLayer *and* a nonzero z-coordinate {}; this is probably not what you want!",
            sprite.original_entity,
            sprite.transform.translation().z
        );
    }
    let mut affine = sprite.transform.affine();
    affine.translation.z = z;
    sprite.transform = GlobalTransform::from(affine);
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ZIndexSortKey(Reverse<OrderedFloat<f32>>);

impl ZIndexSortKey {
    fn new(transform: &GlobalTransform) -> Self {
        Self(Reverse(OrderedFloat(transform.translation().y)))
    }
}

fn map_z_indices<Layer: LayerIndex>(
    transform_query: Extract<Query<(Entity, &GlobalTransform), With<Layer>>>,
    layer_query: Extract<Query<&Layer>>,
) -> HashMap<Entity, f32> {
    let mut all_entities = transform_query
        .iter()
        .map(|(entity, transform)| (ZIndexSortKey::new(transform), entity))
        .collect::<Vec<_>>();

    all_entities.par_sort_unstable();

    let scale_factor = 1.0 / all_entities.len() as f32;
    all_entities
        .into_iter()
        .enumerate()
        .map(|(i, (_, entity))| {
            (
                entity,
                layer_query.get(entity).unwrap().as_z_coordinate() + i as f32 * scale_factor,
            )
        })
        .collect()
}
