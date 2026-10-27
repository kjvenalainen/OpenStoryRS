//! Fixed-step MapleStory movement and foothold collision rules.

use crate::foothold::{FootholdTree, MapBounds};

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
            self.update_ground_state(tree, body, false);
            return;
        };
        if body.mode == PhysicsMode::Fixated && body.foothold_id != 0 {
            return;
        }

        let mut check_slope = false;
        if body.on_ground {
            if body.x.floor() > f64::from(current.right()) {
                body.foothold_id = current.next;
            } else if body.x.ceil() < f64::from(current.left()) {
                body.foothold_id = current.previous;
            }
            if body.foothold_id == 0 {
                body.foothold_id = tree
                    .find_below(body.x, body.y)
                    .map(|(id, _)| id)
                    .unwrap_or(0);
            } else {
                check_slope = true;
            }
        } else {
            body.foothold_id = tree
                .find_below(body.x, body.y)
                .map(|(id, _)| id)
                .unwrap_or(0);
        }
        self.update_ground_state(tree, body, check_slope);
    }

    fn update_ground_state(self, tree: &FootholdTree, body: &mut PhysicsBody, check_slope: bool) {
        let current = tree.get(body.foothold_id).copied();
        let ground = current
            .map(|foothold| foothold.ground_y_extrapolated(body.x))
            .unwrap_or(f64::from(tree.bounds().bottom));
        body.foothold_slope = current.map_or(0.0, |foothold| foothold.slope());

        if body.vertical_speed == 0.0 && check_slope && body.horizontal_speed != 0.0 {
            let vertical_delta = body.foothold_slope.abs() * (ground - body.y).abs();
            if body.foothold_slope != 0.0 && vertical_delta <= body.horizontal_speed.abs() {
                // The C++ negative-direction comparison mixed signed speed and
                // positive distance, preventing slope following while walking left.
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

        if body.vertical_speed == 0.0 {
            return;
        }
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
