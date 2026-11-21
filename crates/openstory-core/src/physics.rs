//! Fixed-step MapleStory movement and foothold collision rules.

use crate::foothold::{Foothold, FootholdTree};

const GRAVITY: f64 = 0.14;
const SWIM_GRAVITY: f64 = 0.03;
const GROUND_FRICTION: f64 = 0.3;
const SLOPE_FACTOR: f64 = 0.1;
const GROUND_SLIP: f64 = 3.0;
const FLY_FRICTION: f64 = 0.05;
const SWIM_FRICTION: f64 = 0.08;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhysicsMode {
    #[default]
    Normal,
    Flying,
    Swimming,
    Fixated,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PhysicsFlags {
    pub no_gravity: bool,
    pub turn_at_edges: bool,
    pub check_below: bool,
}

/// Mutable state for a simulated character or map object.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsBody {
    pub x: f64,
    pub y: f64,
    pub previous_x: f64,
    pub previous_y: f64,
    pub horizontal_speed: f64,
    pub vertical_speed: f64,
    pub horizontal_force: f64,
    pub vertical_force: f64,
    pub horizontal_acceleration: f64,
    pub vertical_acceleration: f64,
    pub foothold_id: u16,
    pub foothold_slope: f64,
    pub foothold_layer: u8,
    pub ground_below: f64,
    pub on_ground: bool,
    pub jump_down_enabled: bool,
    pub traction: f64,
    pub mode: PhysicsMode,
    pub flags: PhysicsFlags,
}

impl PhysicsBody {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            previous_x: x,
            previous_y: y,
            horizontal_speed: 0.0,
            vertical_speed: 0.0,
            horizontal_force: 0.0,
            vertical_force: 0.0,
            horizontal_acceleration: 0.0,
            vertical_acceleration: 0.0,
            foothold_id: 0,
            foothold_slope: 0.0,
            foothold_layer: 0,
            ground_below: 0.0,
            on_ground: false,
            jump_down_enabled: false,
            traction: 1.0,
            mode: PhysicsMode::Normal,
            flags: PhysicsFlags::default(),
        }
    }

    pub fn apply_force(&mut self, horizontal: f64, vertical: f64) {
        self.horizontal_force += horizontal;
        self.vertical_force += vertical;
    }

    fn stop_horizontal(&mut self) {
        self.horizontal_speed = 0.0;
        self.flags.turn_at_edges = false;
    }

    pub fn interpolate(&self, alpha: f64) -> (f64, f64) {
        let alpha = alpha.clamp(0.0, 1.0);
        (
            self.previous_x + (self.x - self.previous_x) * alpha,
            self.previous_y + (self.y - self.previous_y) * alpha,
        )
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicsEngine;

impl PhysicsEngine {
    /// Advances one legacy 8 ms simulation tick. Call from the shared fixed-step
    /// clock, not once per rendered frame.
    pub fn move_body(self, tree: &FootholdTree, body: &mut PhysicsBody) {
        body.previous_x = body.x;
        body.previous_y = body.y;
        let refresh_support =
            body.mode != PhysicsMode::Fixated || tree.get(body.foothold_id).is_none();
        if refresh_support {
            self.update_support(tree, body);
        }
        match body.mode {
            PhysicsMode::Normal => self.move_normal(body),
            PhysicsMode::Flying => self.move_flying(body),
            PhysicsMode::Swimming => self.move_swimming(body),
            PhysicsMode::Fixated => {
                if refresh_support {
                    self.update_jump_down(tree, body);
                }
                body.x += body.horizontal_speed;
                body.y += body.vertical_speed;
                return;
            }
        }

        if body.mode == PhysicsMode::Normal
            && body.on_ground
            && body.vertical_speed == 0.0
            && let Some(floor) = tree.get(body.foothold_id).copied()
        {
            self.move_along_ground(tree, body, floor, body.x + body.horizontal_speed);
        } else {
            self.move_through_air(tree, body);
        }
        self.update_support(tree, body);
        self.update_jump_down(tree, body);
    }

    fn update_support(self, tree: &FootholdTree, body: &mut PhysicsBody) {
        let supported = tree
            .get(body.foothold_id)
            .filter(|floor| body.on_ground && floor.ground_y(body.x) == Some(body.y));
        let (id, ground) = supported
            .map(|floor| (floor.id, body.y))
            .or_else(|| tree.find_below(body.x, body.y))
            .unwrap_or((0, f64::from(tree.bounds().bottom)));
        let floor = tree.get(id);
        body.foothold_id = id;
        body.foothold_slope = floor.map_or(0.0, |floor| floor.slope());
        body.on_ground = body.y == ground;
        if body.foothold_layer == 0 || body.on_ground {
            body.foothold_layer = floor.map_or(0, |floor| floor.layer);
        }
    }

    fn update_jump_down(self, tree: &FootholdTree, body: &mut PhysicsBody) {
        if body.jump_down_enabled || body.flags.check_below {
            let ground = tree
                .get(body.foothold_id)
                .and_then(|floor| floor.ground_y(body.x))
                .unwrap_or(f64::from(tree.bounds().bottom));
            body.jump_down_enabled = tree
                .find_below(body.x, ground + 1.0)
                .is_some_and(|(_, next_ground)| next_ground - ground < 600.0);
            body.ground_below = if body.jump_down_enabled {
                ground + 1.0
            } else {
                0.0
            };
            body.flags.check_below = false;
        }
    }

    fn horizontal_limit(
        self,
        tree: &FootholdTree,
        body: &PhysicsBody,
        target_x: f64,
        linked_walls: bool,
    ) -> Option<f64> {
        if target_x == body.x {
            return None;
        }

        let left = target_x < body.x;
        let bounds = tree.bounds();
        let mut wall = if linked_walls {
            tree.wall_for(body.foothold_id, left, body.y + body.vertical_speed)
        } else if left {
            f64::from(bounds.left_wall)
        } else {
            f64::from(bounds.right_wall)
        };
        if !crosses(body.x, target_x, wall, left) && body.flags.turn_at_edges {
            wall = tree.edge_for(body.foothold_id, left);
        }
        crosses(body.x, target_x, wall, left).then_some(wall)
    }

    fn move_along_ground(
        self,
        tree: &FootholdTree,
        body: &mut PhysicsBody,
        mut floor: Foothold,
        target_x: f64,
    ) {
        if target_x == body.x {
            return;
        }

        let left = target_x < body.x;
        let target_x = if let Some(limit) = self.horizontal_limit(tree, body, target_x, false) {
            body.stop_horizontal();
            limit
        } else {
            target_x
        };

        loop {
            body.foothold_id = floor.id;
            body.foothold_layer = floor.layer;
            let (edge_x, edge_y) = floor.edge(left);
            let reaches_edge = if left {
                target_x <= edge_x
            } else {
                target_x >= edge_x
            };
            if reaches_edge && tree.has_wall_at_edge(floor, left) {
                body.x = edge_x;
                body.y = edge_y;
                body.stop_horizontal();
                return;
            }
            if let Some(y) = floor.ground_y(target_x) {
                body.x = target_x;
                body.y = y;
                return;
            }

            // Resolve each seam at its actual height, even when a tick crosses
            // an entire short ramp before arriving on the next floor.
            body.y = edge_y;
            let Some(next) = tree.floor_across_edge(floor, left, target_x) else {
                body.x = target_x;
                body.on_ground = false;
                return;
            };
            floor = next;
        }
    }

    fn move_through_air(self, tree: &FootholdTree, body: &mut PhysicsBody) {
        let start = (body.x, body.y);
        let end_x = body.x + body.horizontal_speed;
        let limit = self.horizontal_limit(tree, body, end_x, true);
        let target_x = limit.unwrap_or(end_x);
        let target_y = body.y + body.vertical_speed;

        // Keep the original diagonal up to a wall, then finish vertically.
        let wall_y = limit.map_or(target_y, |wall| {
            let wall_time = (wall - body.x) / body.horizontal_speed;
            body.y + body.vertical_speed * wall_time
        });
        let contact = tree
            .find_swept_ground_contact(start, (target_x, wall_y))
            .or_else(|| {
                limit.and_then(|_| {
                    tree.find_swept_ground_contact((target_x, wall_y), (target_x, target_y))
                })
            });
        if let Some(contact) = contact {
            body.x = contact.x;
            body.y = contact.y;
            body.on_ground = true;
            body.foothold_id = contact.floor.id;
            body.foothold_layer = contact.floor.layer;
            body.vertical_speed = 0.0;

            // Re-evaluate horizontal movement at the landing height unless
            // the body already reached the wall during the airborne path.
            if limit == Some(contact.x) {
                body.stop_horizontal();
            } else {
                self.move_along_ground(tree, body, contact.floor, end_x);
            }
        } else {
            if limit.is_some() {
                body.stop_horizontal();
            }
            let bounds = tree.bounds();
            body.x = target_x;
            body.y = target_y.clamp(f64::from(bounds.top), f64::from(bounds.bottom));
            body.on_ground = false;
            if body.y != target_y {
                body.vertical_speed = 0.0;
            }
        }
    }

    fn move_normal(self, body: &mut PhysicsBody) {
        body.vertical_acceleration = 0.0;
        body.horizontal_acceleration = 0.0;
        if body.on_ground {
            body.vertical_acceleration += body.vertical_force;
            body.horizontal_acceleration += body.horizontal_force;
            if body.horizontal_acceleration == 0.0
                && body.horizontal_speed > -0.1
                && body.horizontal_speed < 0.1
            {
                body.horizontal_speed = 0.0;
            } else {
                let inertia = body.horizontal_speed / GROUND_SLIP;
                let slope = body.foothold_slope.clamp(-0.5, 0.5);
                let friction = GROUND_FRICTION * body.traction;
                body.horizontal_acceleration -=
                    (friction + SLOPE_FACTOR * (1.0 - slope * inertia)) * inertia;
            }
        } else if !body.flags.no_gravity {
            body.vertical_acceleration += GRAVITY;
        }
        body.horizontal_force = 0.0;
        body.vertical_force = 0.0;
        body.horizontal_speed += body.horizontal_acceleration;
        body.vertical_speed += body.vertical_acceleration;
    }

    fn move_flying(self, body: &mut PhysicsBody) {
        body.horizontal_acceleration = body.horizontal_force - FLY_FRICTION * body.horizontal_speed;
        body.vertical_acceleration = body.vertical_force - FLY_FRICTION * body.vertical_speed;
        body.horizontal_force = 0.0;
        body.vertical_force = 0.0;
        body.horizontal_speed += body.horizontal_acceleration;
        body.vertical_speed += body.vertical_acceleration;
        snap_small_speed(body.horizontal_acceleration, &mut body.horizontal_speed);
        snap_small_speed(body.vertical_acceleration, &mut body.vertical_speed);
    }

    fn move_swimming(self, body: &mut PhysicsBody) {
        body.horizontal_acceleration =
            body.horizontal_force - SWIM_FRICTION * body.horizontal_speed;
        body.vertical_acceleration = body.vertical_force - SWIM_FRICTION * body.vertical_speed;
        if !body.flags.no_gravity {
            body.vertical_acceleration += SWIM_GRAVITY;
        }
        body.horizontal_force = 0.0;
        body.vertical_force = 0.0;
        body.horizontal_speed += body.horizontal_acceleration;
        body.vertical_speed += body.vertical_acceleration;
        snap_small_speed(body.horizontal_acceleration, &mut body.horizontal_speed);
        snap_small_speed(body.vertical_acceleration, &mut body.vertical_speed);
    }
}

fn crosses(current_x: f64, next_x: f64, wall: f64, left: bool) -> bool {
    if left {
        current_x >= wall && next_x <= wall
    } else {
        current_x <= wall && next_x >= wall
    }
}

fn snap_small_speed(acceleration: f64, speed: &mut f64) {
    if acceleration == 0.0 && *speed > -0.1 && *speed < 0.1 {
        *speed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::{PhysicsBody, PhysicsEngine, PhysicsFlags};
    use crate::foothold::{Foothold, FootholdTree};

    fn floor(id: u16, previous: u16, next: u16, x1: i16, y1: i16, x2: i16, y2: i16) -> Foothold {
        Foothold {
            id,
            previous,
            next,
            layer: 1,
            x1,
            y1,
            x2,
            y2,
        }
    }

    #[test]
    fn normal_mode_applies_legacy_gravity_per_tick() {
        let tree = FootholdTree::new(vec![floor(1, 0, 0, 0, 100, 100, 100)]).unwrap();
        let mut body = PhysicsBody::new(50.0, 20.0);
        body.foothold_id = 1;

        PhysicsEngine.move_body(&tree, &mut body);

        assert!((body.vertical_speed - 0.14).abs() < f64::EPSILON);
        assert!((body.y - 20.14).abs() < 1e-9);
        assert!(!body.on_ground);
    }

    #[test]
    fn grounded_vertical_force_applies_contact_knockback_without_replacing_velocity() {
        let tree = FootholdTree::new(vec![floor(1, 0, 0, 0, 100, 100, 100)]).unwrap();
        let mut body = PhysicsBody::new(50.0, 100.0);
        body.foothold_id = 1;
        body.on_ground = true;
        body.vertical_speed = 0.5;
        body.vertical_force = -3.5;

        PhysicsEngine.move_body(&tree, &mut body);

        assert_eq!(body.vertical_speed, -3.0);
        assert_eq!(body.y, 97.0);
    }

    #[test]
    fn crosses_a_short_uphill_segment_and_lands_on_the_next_floor_in_one_tick() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, -100, 100, 10, 100),
            floor(2, 1, 3, 10, 100, 12, 90),
            floor(3, 2, 0, 12, 90, 100, 90),
        ])
        .unwrap();

        for (x, y, id, direction, end_y, end_id) in [
            (9.5, 100.0, 1, 1.0, 90.0, 3),
            (12.5, 90.0, 3, -1.0, 100.0, 1),
        ] {
            let mut body = PhysicsBody::new(x, y);
            body.foothold_id = id;
            body.on_ground = true;
            body.horizontal_speed = direction * 4.0;

            PhysicsEngine.move_body(&tree, &mut body);

            assert_eq!(body.foothold_id, end_id);
            assert_eq!(body.y, end_y);
            assert!(body.on_ground);
            assert_eq!(body.interpolate(0.0), (x, y));
            assert_eq!(body.interpolate(1.0), (body.x, end_y));
        }
    }

    #[test]
    fn airborne_landing_follows_the_rest_of_a_short_stair_chain() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, 40, 100, 44, 100),
            floor(2, 1, 3, 44, 100, 46, 90),
            floor(3, 2, 0, 46, 90, 100, 90),
            floor(4, 0, 0, -100, 103, 100, 103),
        ])
        .unwrap();
        let mut body = PhysicsBody::new(38.0, 96.0);
        body.horizontal_speed = 12.0;
        body.vertical_speed = 9.0;

        PhysicsEngine.move_body(&tree, &mut body);

        assert_eq!((body.x, body.y), (50.0, 90.0));
        assert_eq!(body.foothold_id, 3);
        assert_eq!(body.vertical_speed, 0.0);
        assert!(body.on_ground);
    }

    #[test]
    fn walking_off_a_short_ramp_leaves_from_its_endpoint() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, -100, 90, 10, 90),
            floor(2, 1, 0, 10, 90, 12, 100),
            floor(3, 0, 0, -100, 150, 100, 150),
        ])
        .unwrap();
        let mut body = PhysicsBody::new(9.5, 90.0);
        body.horizontal_speed = 4.0;

        PhysicsEngine.move_body(&tree, &mut body);

        assert!(body.x > 12.0);
        assert_eq!(body.y, 100.0);
        assert_eq!(body.foothold_id, 3);
        assert!(!body.on_ground);

        PhysicsEngine.move_body(&tree, &mut body);
        assert!(body.y > 100.0);
    }

    #[test]
    fn airborne_sweep_does_not_land_on_a_floor_extension_or_from_below() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 0, -100, 100, 10, 100),
            floor(2, 0, 0, -100, 150, 100, 150),
        ])
        .unwrap();

        let mut past_edge = PhysicsBody::new(9.0, 97.0);
        past_edge.horizontal_speed = 8.0;
        past_edge.vertical_speed = 6.0;
        PhysicsEngine.move_body(&tree, &mut past_edge);
        assert!(past_edge.y > 100.0);
        assert!(!past_edge.on_ground);
        assert_eq!(past_edge.foothold_id, 2);

        let mut jumping = PhysicsBody::new(0.0, 103.0);
        jumping.vertical_speed = -8.0;
        PhysicsEngine.move_body(&tree, &mut jumping);
        assert!(jumping.y < 100.0);
        assert!(!jumping.on_ground);
    }

    fn assert_walk_stays_on_ground(
        tree: &FootholdTree,
        body: &mut PhysicsBody,
        direction: f64,
        ticks: usize,
    ) {
        for _ in 0..ticks {
            body.apply_force(direction * 0.4, 0.0);
            PhysicsEngine.move_body(tree, body);
            assert!(
                body.on_ground,
                "body fell at x={}, y={}, foothold={}",
                body.x, body.y, body.foothold_id
            );
            let ground = tree
                .get(body.foothold_id)
                .and_then(|foothold| foothold.ground_y(body.x))
                .expect("grounded body should have a foothold");
            assert!(
                (body.y - ground).abs() < 1e-6,
                "body y={} did not match ground={} at x={} on foothold {}",
                body.y,
                ground,
                body.x,
                body.foothold_id
            );
        }
    }

    #[test]
    fn flat_ramp_top_and_bottom_transitions_stay_grounded_in_both_directions() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, -100, 100, 10, 100),
            floor(2, 1, 3, 10, 100, 20, 85),
            floor(3, 2, 0, 20, 85, 100, 85),
        ])
        .unwrap();

        let mut moving_up = PhysicsBody::new(9.0, 100.0);
        moving_up.foothold_id = 1;
        moving_up.on_ground = true;
        moving_up.horizontal_speed = 2.0;
        assert_walk_stays_on_ground(&tree, &mut moving_up, 1.0, 8);
        assert!(moving_up.x > 20.0, "walk should pass the ramp top");

        let mut moving_down = PhysicsBody::new(31.0, 85.0);
        moving_down.foothold_id = 3;
        moving_down.on_ground = true;
        moving_down.horizontal_speed = -2.0;
        assert_walk_stays_on_ground(&tree, &mut moving_down, -1.0, 12);
        assert!(moving_down.x < 10.0, "walk should pass the ramp bottom");
    }

    #[test]
    fn unlinked_ramp_endpoints_recover_the_adjacent_floor() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 0, -100, 100, 10, 100),
            floor(2, 0, 0, 10, 100, 20, 85),
            floor(3, 0, 0, 20, 85, 100, 85),
        ])
        .unwrap();

        let mut moving_up = PhysicsBody::new(9.0, 100.0);
        moving_up.foothold_id = 1;
        moving_up.on_ground = true;
        moving_up.horizontal_speed = 2.0;
        assert_walk_stays_on_ground(&tree, &mut moving_up, 1.0, 8);
        assert!(moving_up.x > 20.0, "walk should pass the unlinked ramp top");

        let mut moving_down = PhysicsBody::new(31.0, 85.0);
        moving_down.foothold_id = 3;
        moving_down.on_ground = true;
        moving_down.horizontal_speed = -2.0;
        assert_walk_stays_on_ground(&tree, &mut moving_down, -1.0, 12);
        assert!(
            moving_down.x < 10.0,
            "walk should pass the unlinked ramp bottom"
        );
    }

    #[test]
    fn wall_collision_clamps_horizontal_motion_to_the_foothold_edge() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, 0, 100, 60, 100),
            floor(2, 1, 0, 60, 20, 60, 100),
        ])
        .unwrap();
        let mut body = PhysicsBody::new(50.0, 100.0);
        body.foothold_id = 1;
        body.on_ground = true;
        body.horizontal_speed = 20.0;

        PhysicsEngine.move_body(&tree, &mut body);

        assert_eq!(body.x, 60.0);
        assert_eq!(body.horizontal_speed, 0.0);
    }

    #[test]
    fn walking_checks_the_wall_after_multiple_short_segments() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, -100, 100, 10, 100),
            floor(2, 1, 3, 10, 100, 12, 90),
            floor(3, 2, 4, 12, 90, 14, 90),
            floor(4, 3, 0, 14, 50, 14, 90),
            floor(5, 0, 0, -100, 200, 100, 200),
        ])
        .unwrap();
        let mut body = PhysicsBody::new(9.5, 100.0);
        body.horizontal_speed = 9.0;

        PhysicsEngine.move_body(&tree, &mut body);

        assert_eq!((body.x, body.y), (14.0, 90.0));
        assert_eq!(body.foothold_id, 3);
        assert_eq!(body.horizontal_speed, 0.0);
        assert!(body.on_ground);
    }

    #[test]
    fn jump_down_requires_a_lower_floor_within_600_pixels() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 0, 0, 100, 100, 100),
            floor(2, 0, 0, 0, 300, 100, 300),
        ])
        .unwrap();
        let mut body = PhysicsBody::new(50.0, 100.0);
        body.foothold_id = 1;
        body.on_ground = true;
        body.flags.check_below = true;

        PhysicsEngine.move_body(&tree, &mut body);

        assert!(body.jump_down_enabled);
        assert_eq!(body.ground_below, 101.0);
        assert!(!body.flags.check_below);

        let distant_tree = FootholdTree::new(vec![
            floor(1, 0, 0, 0, 100, 100, 100),
            floor(2, 0, 0, 0, 701, 100, 701),
        ])
        .unwrap();
        let mut distant_body = PhysicsBody::new(50.0, 100.0);
        distant_body.foothold_id = 1;
        distant_body.on_ground = true;
        distant_body.flags = PhysicsFlags {
            check_below: true,
            ..Default::default()
        };

        PhysicsEngine.move_body(&distant_tree, &mut distant_body);

        assert!(!distant_body.jump_down_enabled);
        assert_eq!(distant_body.ground_below, 0.0);
    }
}
