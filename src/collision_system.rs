use bevy::prelude::*;

use crate::{AppState, DebugSkipPlayerAction, GameEvent, GameObjectType, Player, PlayerEnd};

/// A 3D Axis-Aligned Bounding Box component.
#[derive(Component, Debug, Copy, Clone, PartialEq)] // Added PartialEq for assert_eq!
pub struct LLAabb3d {
    pub min: Vec3,
    pub max: Vec3,
}

impl LLAabb3d {
    /// Creates a new AABB.
    ///
    /// # Arguments
    ///
    /// * `center` - The center point of the AABB.
    /// * `half_extents` - Half the size of the AABB along each axis.
    pub fn new(center: Vec3, half_extents: Vec3) -> Self {
        // Ensure half_extents are non-negative to avoid inverted AABBs
        let positive_half_extents = half_extents.abs();
        Self {
            min: center - positive_half_extents,
            max: center + positive_half_extents,
        }
    }

    /// Checks if this AABB intersects with another AABB.
    ///
    /// # Arguments
    ///
    /// * `other` - The other AABB to test against.
    ///
    /// # Returns
    ///
    /// `true` if the AABBs intersect, `false` otherwise.
    pub fn intersects(&self, other: &LLAabb3d) -> bool {
        // Check for overlap on each axis
        let x_overlap = self.min.x <= other.max.x && self.max.x >= other.min.x;
        let y_overlap = self.min.y <= other.max.y && self.max.y >= other.min.y;
        let z_overlap = self.min.z <= other.max.z && self.max.z >= other.min.z;

        // Intersection occurs if there is overlap on all axes
        x_overlap && y_overlap && z_overlap
    }

    /// Returns the center of the AABB.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    /// Returns the extents (size) of the AABB.
    #[allow(dead_code)]
    pub fn extents(&self) -> Vec3 {
        self.max - self.min
    }

    /// Returns the half-extents of the AABB.
    pub fn half_extents(&self) -> Vec3 {
        (self.max - self.min) / 2.0
    }

    /// Updates the center of the AABB, preserving its size.
    ///
    /// # Arguments
    ///
    /// * `new_center` - The new desired center point for the AABB.
    pub fn update_center(&mut self, new_center: Vec3) {
        let half_extents = self.half_extents();
        self.min = new_center - half_extents;
        self.max = new_center + half_extents;
    }
}

pub fn player_collision_handling_system(
    mut query: Query<Entity, With<Player>>,
    mut collision_event: EventReader<CollisionEvent>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut commands: Commands,
    debug: Res<DebugSkipPlayerAction>,
) {
    if debug.skip_player_collision {
        return;
    }
    for _ev in collision_event.read() {
        let Ok(entity) = query.single_mut() else {
            return;
        };
        commands
            .entity(entity)
            .insert_if_new(PlayerEnd(Timer::from_seconds(1.6, TimerMode::Once)));
        commands.trigger(GameEvent::Over);
        next_app_state.set(AppState::EndGame);
    }
}

#[derive(Event)]
pub enum CollisionEvent {
    PlayerLog,
    // LogRock,
}

pub fn collision_detection_system(
    query: Query<(&GameObjectType, &LLAabb3d)>,
    mut collision_event: EventWriter<CollisionEvent>,
) {
    let mut combinations = query.iter_combinations();
    while let Some([(got_a, aabb_a), (got_b, aabb_b)]) = combinations.fetch_next() {
        if aabb_a.intersects(aabb_b) && got_a != got_b {
            // Handle collision
            info!(
                "Intersection detected between {:?} and {:?}",
                (got_a, aabb_a),
                (got_b, aabb_b)
            );
            match got_a {
                GameObjectType::Player => {
                    collision_event.write(CollisionEvent::PlayerLog);
                }
                _ => (),
            }
            match got_b {
                GameObjectType::Player => {
                    collision_event.write(CollisionEvent::PlayerLog);
                }
                _ => (),
            }
        }
    }
}

// This section is for tests
#[cfg(test)]
mod tests {
    use super::*; // Import everything from the outer module (Aabb3d, Vec3 etc.)

    #[test]
    fn test_aabb_creation_and_properties() {
        let center = Vec3::new(1.0, 2.0, 3.0);
        let half_extents = Vec3::new(0.5, 1.0, 1.5);
        let aabb = LLAabb3d::new(center, half_extents);

        assert_eq!(aabb.min, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(aabb.max, Vec3::new(1.5, 3.0, 4.5));
        assert_eq!(aabb.center(), center);
        assert_eq!(aabb.half_extents(), half_extents);
        assert_eq!(aabb.extents(), half_extents * 2.0);

        // Test with negative half_extents (should be abs'd)
        let negative_half_extents = Vec3::new(-0.5, -1.0, -1.5);
        let aabb_neg_ext = LLAabb3d::new(center, negative_half_extents);
        assert_eq!(aabb_neg_ext.min, Vec3::new(0.5, 1.0, 1.5));
        assert_eq!(aabb_neg_ext.max, Vec3::new(1.5, 3.0, 4.5));
    }

    #[test]
    fn test_aabb_intersection() {
        let aabb1 = LLAabb3d {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(2.0, 2.0, 2.0),
        };

        // Case 1: Overlapping AABB
        let aabb2_overlapping = LLAabb3d {
            min: Vec3::new(1.0, 1.0, 1.0),
            max: Vec3::new(3.0, 3.0, 3.0),
        };
        assert!(
            aabb1.intersects(&aabb2_overlapping),
            "aabb1 should intersect with aabb2_overlapping"
        );
        assert!(
            aabb2_overlapping.intersects(&aabb1),
            "aabb2_overlapping should intersect with aabb1 (commutative)"
        );

        // Case 2: Non-overlapping AABB (separate on X)
        let aabb3_separate_x = LLAabb3d {
            min: Vec3::new(3.0, 0.0, 0.0),
            max: Vec3::new(5.0, 2.0, 2.0),
        };
        assert!(
            !aabb1.intersects(&aabb3_separate_x),
            "aabb1 should not intersect with aabb3_separate_x"
        );

        // Case 3: Non-overlapping AABB (separate on Y)
        let aabb4_separate_y = LLAabb3d {
            min: Vec3::new(0.0, 3.0, 0.0),
            max: Vec3::new(2.0, 5.0, 2.0),
        };
        assert!(
            !aabb1.intersects(&aabb4_separate_y),
            "aabb1 should not intersect with aabb4_separate_y"
        );

        // Case 4: Non-overlapping AABB (separate on Z)
        let aabb5_separate_z = LLAabb3d {
            min: Vec3::new(0.0, 0.0, 3.0),
            max: Vec3::new(2.0, 2.0, 5.0),
        };
        assert!(
            !aabb1.intersects(&aabb5_separate_z),
            "aabb1 should not intersect with aabb5_separate_z"
        );

        // Case 5: Touching AABB (should be considered an intersection)
        let aabb6_touching_face = LLAabb3d {
            min: Vec3::new(2.0, 0.0, 0.0), // Touches aabb1 on the x-max face
            max: Vec3::new(4.0, 2.0, 2.0),
        };
        assert!(
            aabb1.intersects(&aabb6_touching_face),
            "aabb1 should intersect with aabb6_touching_face (face touch)"
        );

        let aabb7_touching_edge = LLAabb3d {
            min: Vec3::new(2.0, 2.0, 0.0), // Touches aabb1 on an edge
            max: Vec3::new(3.0, 3.0, 1.0),
        };
        assert!(
            aabb1.intersects(&aabb7_touching_edge),
            "aabb1 should intersect with aabb7_touching_edge (edge touch)"
        );

        let aabb8_touching_corner = LLAabb3d {
            min: Vec3::new(2.0, 2.0, 2.0), // Touches aabb1 at a corner
            max: Vec3::new(3.0, 3.0, 3.0),
        };
        assert!(
            aabb1.intersects(&aabb8_touching_corner),
            "aabb1 should intersect with aabb8_touching_corner (corner touch)"
        );

        // Case 6: One AABB completely inside another
        let aabb9_inside = LLAabb3d {
            min: Vec3::new(0.5, 0.5, 0.5),
            max: Vec3::new(1.5, 1.5, 1.5),
        };
        assert!(
            aabb1.intersects(&aabb9_inside),
            "aabb1 should intersect with aabb9_inside (aabb9 is inside aabb1)"
        );
        assert!(
            aabb9_inside.intersects(&aabb1),
            "aabb9_inside should intersect with aabb1 (aabb9 is inside aabb1)"
        );

        // Case 7: Identical AABBs
        let aabb1_clone = LLAabb3d {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(2.0, 2.0, 2.0),
        };
        assert!(
            aabb1.intersects(&aabb1_clone),
            "aabb1 should intersect with its identical clone"
        );
    }

    #[test]
    fn test_aabb_no_volume() {
        // AABB with no volume (min == max)
        let point_aabb = LLAabb3d {
            min: Vec3::new(1.0, 1.0, 1.0),
            max: Vec3::new(1.0, 1.0, 1.0),
        };

        let aabb1 = LLAabb3d {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(2.0, 2.0, 2.0),
        };
        assert!(
            point_aabb.intersects(&aabb1),
            "Point AABB inside a larger AABB should intersect"
        );

        let point_aabb_outside = LLAabb3d {
            min: Vec3::new(3.0, 3.0, 3.0),
            max: Vec3::new(3.0, 3.0, 3.0),
        };
        assert!(
            !point_aabb_outside.intersects(&aabb1),
            "Point AABB outside a larger AABB should not intersect"
        );

        let point_aabb_on_boundary = LLAabb3d {
            min: Vec3::new(2.0, 2.0, 2.0),
            max: Vec3::new(2.0, 2.0, 2.0),
        };
        assert!(
            point_aabb_on_boundary.intersects(&aabb1),
            "Point AABB on the boundary should intersect"
        );
    }

    #[test]
    fn test_update_center() {
        let initial_center = Vec3::new(1.0, 2.0, 3.0);
        let half_extents = Vec3::new(0.5, 1.0, 1.5);
        let mut aabb = LLAabb3d::new(initial_center, half_extents);

        let new_center = Vec3::new(10.0, 20.0, 30.0);
        aabb.update_center(new_center);

        assert_eq!(aabb.center(), new_center, "Center should be updated");
        assert_eq!(
            aabb.half_extents(),
            half_extents,
            "Half-extents should remain the same"
        );

        let expected_min = new_center - half_extents;
        let expected_max = new_center + half_extents;
        assert_eq!(aabb.min, expected_min, "Min should be updated correctly");
        assert_eq!(aabb.max, expected_max, "Max should be updated correctly");
    }
}
