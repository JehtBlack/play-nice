use bevy::{
    math::bounding::{Aabb2d, BoundingVolume, IntersectsVolume},
    prelude::*,
};
use bevy_rapier2d::{
    pipeline::{CollisionEvent, ContactForceEvent},
    plugin::RapierContext,
};

use crate::{Conveyor, Package};

pub enum SimpleCollision {
    Left,
    Right,
    Top,
    Bottom,
    Inside,
}

#[derive(Default, Component)]
pub struct SimpleCollider {
    pub size: Vec2,
}

#[derive(Event)]
pub struct SimpleCollisionEvent {
    pub collision: SimpleCollision,
    pub entity_a: Entity,
    pub entity_b: Entity,
}

#[derive(Component)]
pub struct WallTag;

pub fn check_for_collisions(
    collider_query: Query<(Entity, &SimpleCollider, &GlobalTransform, Option<&Parent>)>,
    mut collision_events: EventWriter<SimpleCollisionEvent>,
) {
    for (entity_a, collider_a, transform_a, _parent_a) in collider_query.iter() {
        for (entity_b, collider_b, transform_b, _parent_b) in collider_query.iter() {
            if entity_a == entity_b {
                continue;
            }

            let aabb_a = Aabb2d::new(transform_a.translation().truncate(), collider_a.size / 2.);
            let aabb_b = Aabb2d::new(transform_b.translation().truncate(), collider_b.size / 2.);

            if aabb_a.intersects(&aabb_b) {
                let closest = aabb_b.closest_point(aabb_a.center());
                let offset = aabb_a.center() - closest;
                let side = if offset.x.abs() > offset.y.abs() {
                    if offset.x > 0. {
                        SimpleCollision::Right
                    } else {
                        SimpleCollision::Left
                    }
                } else if offset.y.abs() > offset.x.abs() {
                    if offset.y > 0. {
                        SimpleCollision::Top
                    } else {
                        SimpleCollision::Bottom
                    }
                } else {
                    SimpleCollision::Inside
                };
                collision_events.send(SimpleCollisionEvent {
                    collision: side,
                    entity_a: entity_a,
                    entity_b: entity_b,
                });
            }
        }
    }
}

pub fn react_to_basic_collisions(
    mut collision_events: EventReader<SimpleCollisionEvent>,
    mut query: Query<(Entity, &mut Transform, &SimpleCollider, Option<&Parent>)>,
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
                SimpleCollision::Left => {
                    transform_a.translation.x = transform_b.translation.x
                        - (collider_b.size.x / 2.)
                        - (collider_a.size.x / 2.);
                }
                SimpleCollision::Right => {
                    transform_a.translation.x = transform_b.translation.x
                        + (collider_b.size.x / 2.)
                        + (collider_a.size.x / 2.);
                }
                SimpleCollision::Top => {
                    transform_a.translation.y = transform_b.translation.y
                        + (collider_b.size.y / 2.)
                        + (collider_a.size.y / 2.);
                }
                SimpleCollision::Bottom => {
                    transform_a.translation.y = transform_b.translation.y
                        - (collider_b.size.y / 2.)
                        - (collider_a.size.y / 2.);
                }
                SimpleCollision::Inside => {}
            }
        }
    }
}

pub fn clear_frame_collisions(mut collision_events: EventReader<SimpleCollisionEvent>) {
    collision_events.clear();
}

pub fn display_physics_events(
    mut collision_events: EventReader<CollisionEvent>,
    mut contact_force_events: EventReader<ContactForceEvent>,
) {
    for collision_event in collision_events.read() {
        println!("Received collision event: {:?}", collision_event);
    }

    for contact_force_event in contact_force_events.read() {
        println!("Received contact force event: {:?}", contact_force_event);
    }
}

pub fn display_physics_contacts(rapier_context: Res<RapierContext>) {
    for contact in rapier_context.contact_pairs() {
        println!(
            "Contact between {:?} and {:?}",
            contact.collider1(),
            contact.collider2()
        );
    }
}
