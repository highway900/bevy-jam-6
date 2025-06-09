use bevy::prelude::*;

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct Active;

#[derive(Component, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_position_and_size(position: Vec3, size: Vec3) -> Self {
        let half_size = size / 2.0;
        Self {
            min: position - half_size,
            max: position + half_size,
        }
    }

    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x 
            && self.max.x >= other.min.x 
            && self.min.y <= other.max.y 
            && self.max.y >= other.min.y 
            && self.min.z <= other.max.z 
            && self.max.z >= other.min.z
    }
}

#[derive(Event, Debug)]
pub struct CollisionEvent {
    pub entity_a: Entity,
    pub entity_b: Entity,
}

pub fn aabb_collision_system(
    mut collision_events: EventWriter<CollisionEvent>,
    query: Query<(Entity, &Transform, &Aabb)>,
) {
    // Get all entities with Transform and Aabb components
    let entities: Vec<(Entity, &Transform, &Aabb)> = query.iter().collect();
    
    // Check each pair of entities for collisions
    for i in 0..entities.len() {
        let (entity_a, transform_a, aabb_a) = entities[i];
        
        // Calculate world-space AABB for entity A
        let aabb_a_world = Aabb::new(
            transform_a.transform_point(aabb_a.min),
            transform_a.transform_point(aabb_a.max),
        );
        
        for j in (i + 1)..entities.len() {
            let (entity_b, transform_b, aabb_b) = entities[j];
            
            // Calculate world-space AABB for entity B
            let aabb_b_world = Aabb::new(
                transform_b.transform_point(aabb_b.min),
                transform_b.transform_point(aabb_b.max),
            );
            
            // Check for collision
            if aabb_a_world.intersects(&aabb_b_world) {
                info!("Collision detected between entities: {:?} and {:?}", entity_a, entity_b);
                collision_events.write(CollisionEvent {
                    entity_a,
                    entity_b,
                });
            }
        }
    }
}
