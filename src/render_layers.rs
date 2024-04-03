use std::collections::BTreeSet;

use bevy::ecs::component::Component;

use crate::LayerIndex;

#[derive(Debug, Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub enum EntityLayer {
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
pub enum RenderLayers {
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
