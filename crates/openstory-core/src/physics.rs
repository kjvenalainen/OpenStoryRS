//! Fixed-step MapleStory movement and foothold collision rules.

use crate::foothold::{FootholdTree, MapBounds};

const GRAVITY: f64 = 0.14;
const SWIM_GRAVITY: f64 = 0.03;
const GROUND_FRICTION: f64 = 0.3;
const SLOPE_FACTOR: f64 = 0.1;
const GROUND_SLIP: f64 = 3.0;
const FLY_FRICTION: f64 = 0.05;
const SWIM_FRICTION: f64 = 0.08;
const MAX_STAIR_STEP_DOWN: f64 = 24.0;

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
        self.update_foothold(tree, body);
        match body.mode {
            PhysicsMode::Normal => {
                self.move_normal(body);
                self.limit_movement(tree, body);
            }
            PhysicsMode::Flying => {
                self.move_flying(body);
                self.limit_movement(tree, body);
            }
            PhysicsMode::Swimming => {
                self.move_swimming(body);
                self.limit_movement(tree, body);
            }
            PhysicsMode::Fixated => {}
        }

        body.previous_x = body.x;
        body.previous_y = body.y;
        body.x += body.horizontal_speed;
        body.y += body.vertical_speed;
    }

    fn update_foothold(self, tree: &FootholdTree, body: &mut PhysicsBody) {
        let Some(current) = tree.get(body.foothold_id).copied() else {
            body.foothold_id = tree
                .find_below(body.x, body.y)
                .map(|(id, _)| id)
                .unwrap_or(0);
            self.update_ground_state(tree, body, false, 0.0);
            return;
        };
        if body.mode == PhysicsMode::Fixated && body.foothold_id != 0 {
            return;
        }

        // A character can move several ticks along one diagonal foothold before
        // reaching its linked neighbor. Keep following that same ground line
        // while grounded instead of only snapping when the foothold id changes.
        let check_slope = body.on_ground;
        if body.on_ground {
            if body.x.floor() > f64::from(current.right()) {
                body.foothold_id = current.next;
                if !foothold_covers_x(tree, body.foothold_id, body.x) {
                    body.foothold_id = transition_foothold(tree, current, body).unwrap_or(0);
                }
            } else if body.x.ceil() < f64::from(current.left()) {
                body.foothold_id = current.previous;
                if !foothold_covers_x(tree, body.foothold_id, body.x) {
                    body.foothold_id = transition_foothold(tree, current, body).unwrap_or(0);
                }
            }
            if body.foothold_id == 0 {
                body.foothold_id = tree
                    .find_below(body.x, body.y)
                    .map(|(id, _)| id)
                    .unwrap_or(0);
            }
        } else {
            body.foothold_id = tree
                .find_below(body.x, body.y)
                .map(|(id, _)| id)
                .unwrap_or(0);
        }
        self.update_ground_state(tree, body, check_slope, current.slope());
    }

    fn update_ground_state(
        self,
        tree: &FootholdTree,
        body: &mut PhysicsBody,
        check_slope: bool,
        previous_slope: f64,
    ) {
        let current = tree.get(body.foothold_id).copied();
        let ground = current
            .map(|foothold| foothold.ground_y_extrapolated(body.x))
            .unwrap_or(f64::from(tree.bounds().bottom));
        body.foothold_slope = current.map_or(0.0, |foothold| foothold.slope());

        if body.vertical_speed == 0.0 && check_slope && body.horizontal_speed != 0.0 {
            let step_x = (body.x - body.previous_x).abs();
            let max_slope = body.foothold_slope.abs().max(previous_slope.abs());
            // The legacy floor/ceil edge checks can defer a transition by one
            // pixel, so include that pixel when estimating the ramp's height
            // change across this step.
            let max_ground_delta = max_slope * (step_x + 1.0) + 1.0;
            if (ground - body.y).abs() <= max_ground_delta {
                body.y = ground;
            }
        }

        body.on_ground = body.y == ground;
        if body.jump_down_enabled || body.flags.check_below {
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

        if body.foothold_layer == 0 || body.on_ground {
            body.foothold_layer = current.map_or(0, |foothold| foothold.layer);
        }
        if body.foothold_id == 0 {
            if let Some(current) = current {
                body.foothold_id = current.id;
                body.x = f64::from(current.x1);
                body.horizontal_speed = 0.0;
            }
        }
    }

    fn limit_movement(self, tree: &FootholdTree, body: &mut PhysicsBody) {
        if body.horizontal_speed != 0.0 {
            let current_x = body.x;
            let next_x = body.x + body.horizontal_speed;
            let left = body.horizontal_speed < 0.0;
            let mut wall = tree.wall_for(body.foothold_id, left, body.y + body.vertical_speed);
            let mut collision = crosses(current_x, next_x, wall, left);
            if !collision && body.flags.turn_at_edges {
                wall = tree.edge_for(body.foothold_id, left);
                collision = crosses(current_x, next_x, wall, left);
            }
            if collision {
                body.x = wall;
                body.horizontal_speed = 0.0;
                body.flags.turn_at_edges = false;
            }
        }

        if body.mode == PhysicsMode::Normal && body.vertical_speed == 0.0 && body.on_ground {
            self.follow_grounded_path(tree, body);
        }

        if body.vertical_speed == 0.0 {
            return;
        }
        let current_x = body.x;
        let next_x = body.x + body.horizontal_speed;
        let current_y = body.y;
        let next_y = body.y + body.vertical_speed;
        let ground_current = tree
            .get(body.foothold_id)
            .map(|foothold| foothold.ground_y_extrapolated(body.x))
            .unwrap_or(f64::from(tree.bounds().bottom));
        let ground_next = tree
            .get(body.foothold_id)
            .map(|foothold| foothold.ground_y_extrapolated(body.x + body.horizontal_speed))
            .unwrap_or(f64::from(tree.bounds().bottom));
        let top_ground = ground_current.min(ground_next);
        let bottom_ground = ground_current.max(ground_next);
        if current_y <= top_ground && next_y >= bottom_ground {
            body.y = bottom_ground;
            body.vertical_speed = 0.0;
            self.limit_movement(tree, body);
        } else if let Some((foothold_id, contact_y)) =
            tree.find_swept_ground_contact(current_x, current_y, next_x, next_y)
        {
            let foothold = tree.get(foothold_id).copied();
            body.foothold_id = foothold_id;
            body.foothold_slope = foothold.map_or(0.0, |foothold| foothold.slope());
            body.foothold_layer = foothold.map_or(body.foothold_layer, |foothold| foothold.layer);
            if let Some(ground_y) = foothold.and_then(|foothold| foothold.ground_y(next_x)) {
                body.y = ground_y;
                body.on_ground = true;
            } else {
                body.y = contact_y;
                body.on_ground = true;
                if !self.follow_grounded_path(tree, body) {
                    body.on_ground = false;
                }
            }
            body.vertical_speed = 0.0;
            self.limit_movement(tree, body);
        } else {
            let MapBounds { top, bottom, .. } = tree.bounds();
            if next_y < f64::from(top) {
                body.y = f64::from(top);
                body.vertical_speed = 0.0;
            } else if next_y > f64::from(bottom) {
                body.y = f64::from(bottom);
                body.vertical_speed = 0.0;
            }
        }
    }

    /// Follows linked, non-rising footholds when one horizontal step crosses
    /// more than one segment. This keeps momentum from carrying the body past
    /// narrow stair surfaces before gravity can bring it down onto them.
    fn follow_grounded_path(self, tree: &FootholdTree, body: &mut PhysicsBody) -> bool {
        if body.horizontal_speed == 0.0 {
            return false;
        }

        let left = body.horizontal_speed < 0.0;
        let target_x = body.x + body.horizontal_speed;
        let Some(mut current) = tree.get(body.foothold_id).copied() else {
            return false;
        };
        if current.ground_y(target_x).is_some() {
            return false;
        }

        // Foothold links can contain several short stair segments, so walk the
        // links crossed by this tick instead of checking only one neighbor.
        for _ in 0..tree.footholds().len() {
            let next_id = if left { current.previous } else { current.next };
            let Some(next) = tree.get(next_id).copied() else {
                return false;
            };
            if next.is_wall() {
                return false;
            }
            let current_edge = if left {
                current.left()
            } else {
                current.right()
            };
            let next_edge = if left { next.right() } else { next.left() };
            let horizontal_gap = if left {
                f64::from(current_edge) - f64::from(next_edge)
            } else {
                f64::from(next_edge) - f64::from(current_edge)
            };
            if horizontal_gap.abs() > 1.0 {
                return false;
            }

            let height_change = next.ground_y_extrapolated(f64::from(next_edge))
                - current.ground_y_extrapolated(f64::from(current_edge));
            if height_change < -1.0 || height_change > MAX_STAIR_STEP_DOWN {
                return false;
            }

            if let Some(ground_y) = next.ground_y(target_x) {
                if ground_y >= body.y {
                    body.foothold_id = next.id;
                    body.foothold_slope = next.slope();
                    body.foothold_layer = next.layer;
                    body.y = ground_y;
                    body.on_ground = true;
                    return true;
                }
                return false;
            }

            // Continue only when this segment lies between the starting
            // foothold and the body's destination for this tick.
            if (left && target_x >= f64::from(next.left()))
                || (!left && target_x <= f64::from(next.right()))
            {
                return false;
            }
            current = next;
        }
        false
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

fn foothold_covers_x(tree: &FootholdTree, foothold_id: u16, x: f64) -> bool {
    tree.get(foothold_id)
        .is_some_and(|foothold| foothold.ground_y(x).is_some())
}

/// Finds a nearby floor when a grounded foothold ends without a usable link.
/// Only accept a surface close enough to be continuous over the last movement
/// step, so a nearby platform at another height does not become a false ramp.
fn transition_foothold(
    tree: &FootholdTree,
    previous: crate::foothold::Foothold,
    body: &PhysicsBody,
) -> Option<u16> {
    let step_x = (body.x - body.previous_x).abs();
    tree.footholds()
        .iter()
        .copied()
        .filter_map(|foothold| {
            let ground = foothold.ground_y(body.x)?;
            let gap = (ground - body.y).abs();
            let max_slope = previous.slope().abs().max(foothold.slope().abs());
            let max_gap = max_slope * (step_x + 1.0) + 1.0;
            (gap <= max_gap).then_some((foothold.id, gap))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(foothold_id, _)| foothold_id)
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
    fn linked_slope_transition_keeps_body_on_the_new_ground() {
        let tree = FootholdTree::new(vec![
            floor(1, 0, 2, 0, 0, 10, 10),
            floor(2, 1, 0, 10, 10, 20, 20),
        ])
        .unwrap();
        let mut body = PhysicsBody::new(11.0, 10.0);
        body.foothold_id = 1;
        body.on_ground = true;
        body.horizontal_speed = 5.0;

        PhysicsEngine.move_body(&tree, &mut body);

        assert_eq!(body.foothold_id, 2);
        assert_eq!(body.y, 11.0);
        assert!(body.on_ground);
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
                .map(|foothold| foothold.ground_y_extrapolated(body.previous_x))
                .expect("grounded body should have a foothold");
            assert!(
                (body.y - ground).abs() < 1e-6,
                "body y={} did not match ground={} at x={} on foothold {}",
                body.y,
                ground,
                body.previous_x,
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
