//! Map footholds and their spatial lookup structure.
//!
//! The legacy client indexed every pixel covered by every floor in a multimap.
//! This version places each floor in fixed-width horizontal buckets, keeping
//! map-load memory proportional to the number of footholds and making lookup
//! work proportional to the local map geometry.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

const BUCKET_WIDTH: i32 = 64;
const MAX_STAIR_STEP_DOWN: f64 = 24.0;
const CONTACT_EPSILON: f64 = 1e-7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapBounds {
    pub left_wall: i16,
    pub right_wall: i16,
    pub top: i16,
    pub bottom: i16,
}

impl Default for MapBounds {
    fn default() -> Self {
        Self {
            left_wall: i16::MIN,
            right_wall: i16::MAX,
            top: i16::MIN,
            bottom: i16::MAX,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Foothold {
    pub id: u16,
    pub previous: u16,
    pub next: u16,
    pub layer: u8,
    pub x1: i16,
    pub y1: i16,
    pub x2: i16,
    pub y2: i16,
}

impl Foothold {
    pub fn is_wall(self) -> bool {
        self.id != 0 && self.x1 == self.x2
    }

    pub fn is_floor(self) -> bool {
        self.id != 0 && self.y1 == self.y2
    }

    pub fn left(self) -> i16 {
        self.x1.min(self.x2)
    }

    pub fn right(self) -> i16 {
        self.x1.max(self.x2)
    }

    pub fn top(self) -> i16 {
        self.y1.min(self.y2)
    }

    pub fn bottom(self) -> i16 {
        self.y1.max(self.y2)
    }

    pub fn slope(self) -> f64 {
        if self.is_wall() {
            0.0
        } else {
            (f64::from(self.y2) - f64::from(self.y1)) / (f64::from(self.x2) - f64::from(self.x1))
        }
    }

    /// Returns this segment's ground height at `x`; vertical footholds are walls.
    pub fn ground_y(self, x: f64) -> Option<f64> {
        if self.is_wall() || x < f64::from(self.left()) || x > f64::from(self.right()) {
            return None;
        }
        Some(self.ground_y_extrapolated(x))
    }

    fn ground_y_extrapolated(self, x: f64) -> f64 {
        if x == f64::from(self.x2) {
            return f64::from(self.y2);
        }
        self.slope() * (x - f64::from(self.x1)) + f64::from(self.y1)
    }

    pub(crate) fn edge(self, left: bool) -> (f64, f64) {
        let x = f64::from(if left { self.left() } else { self.right() });
        (x, self.ground_y_extrapolated(x))
    }

    fn neighbor(self, left: bool) -> u16 {
        if left { self.previous } else { self.next }
    }

    fn blocks_vertical_range(self, top: i16, bottom: i16) -> bool {
        self.is_wall() && self.top() <= bottom && top <= self.bottom()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FootholdError {
    ZeroId,
    DuplicateId(u16),
}

impl fmt::Display for FootholdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroId => write!(f, "foothold id zero is reserved for no platform"),
            Self::DuplicateId(id) => write!(f, "map contains duplicate foothold id {id}"),
        }
    }
}

impl Error for FootholdError {}

pub(crate) struct GroundContact {
    pub floor: Foothold,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug)]
pub struct FootholdTree {
    footholds: Vec<Foothold>,
    id_to_index: HashMap<u16, usize>,
    floor_buckets: Vec<Vec<usize>>,
    bucket_origin: i32,
    bounds: MapBounds,
}

impl FootholdTree {
    pub fn new(footholds: Vec<Foothold>) -> Result<Self, FootholdError> {
        let mut id_to_index = HashMap::with_capacity(footholds.len());
        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        for (index, foothold) in footholds.iter().copied().enumerate() {
            if foothold.id == 0 {
                return Err(FootholdError::ZeroId);
            }
            if id_to_index.insert(foothold.id, index).is_some() {
                return Err(FootholdError::DuplicateId(foothold.id));
            }
            min_x = min_x.min(i32::from(foothold.left()));
            max_x = max_x.max(i32::from(foothold.right()));
            min_y = min_y.min(i32::from(foothold.top()));
            max_y = max_y.max(i32::from(foothold.bottom()));
        }

        let bounds = make_bounds(min_x, max_x, min_y, max_y);
        let bucket_origin = if footholds.is_empty() { 0 } else { min_x };
        let bucket_count = if footholds.is_empty() {
            1
        } else {
            ((max_x - bucket_origin).div_euclid(BUCKET_WIDTH) + 1) as usize
        };
        let mut floor_buckets = vec![Vec::new(); bucket_count];

        for (index, foothold) in footholds.iter().copied().enumerate() {
            // Vertical and degenerate segments are collision walls, never floors.
            if foothold.is_wall() {
                continue;
            }
            let first = bucket_index(i32::from(foothold.left()), bucket_origin);
            let last = bucket_index(i32::from(foothold.right()), bucket_origin);
            for bucket in first..=last {
                floor_buckets[bucket].push(index);
            }
        }

        Ok(Self {
            footholds,
            id_to_index,
            floor_buckets,
            bucket_origin,
            bounds,
        })
    }

    pub fn bounds(&self) -> MapBounds {
        self.bounds
    }

    pub fn footholds(&self) -> &[Foothold] {
        &self.footholds
    }

    pub fn get(&self, id: u16) -> Option<&Foothold> {
        self.id_to_index
            .get(&id)
            .map(|index| &self.footholds[*index])
    }

    /// Finds the closest floor at or below a point in Maple's downward-positive
    /// coordinate system. Equal-height matches prefer a flat floor over a ramp;
    /// ties between floors of the same type select the last source candidate.
    pub fn find_below(&self, x: f64, y: f64) -> Option<(u16, f64)> {
        if !x.is_finite() || !y.is_finite() || x < f64::from(i16::MIN) || x > f64::from(i16::MAX) {
            return None;
        }
        let x_pixel = x.trunc() as i32;
        let bucket = bucket_index(x_pixel, self.bucket_origin);
        let mut nearest: Option<(u16, f64)> = None;
        for index in self.floor_buckets.get(bucket)?.iter().copied() {
            let foothold = self.footholds[index];
            let Some(ground_y) = foothold.ground_y(x) else {
                continue;
            };
            if ground_y >= y
                && nearest.is_none_or(|(best_id, best_ground_y)| {
                    ground_y < best_ground_y
                        || ground_y == best_ground_y
                            && (foothold.is_floor()
                                || !self.get(best_id).is_some_and(|best| best.is_floor()))
                })
            {
                nearest = Some((foothold.id, ground_y));
            }
        }
        nearest
    }

    /// Finds the closest floor at or above a point in Maple's downward-positive
    /// coordinate system. Equal-height matches retain the first source candidate.
    pub fn find_above(&self, x: f64, y: f64) -> Option<(u16, f64)> {
        if !x.is_finite() || !y.is_finite() || x < f64::from(i16::MIN) || x > f64::from(i16::MAX) {
            return None;
        }
        let x_pixel = x.trunc() as i32;
        let bucket = bucket_index(x_pixel, self.bucket_origin);
        let mut nearest: Option<(u16, f64)> = None;
        for index in self.floor_buckets.get(bucket)?.iter().copied() {
            let foothold = self.footholds[index];
            let Some(ground_y) = foothold.ground_y(x) else {
                continue;
            };
            if ground_y <= y && nearest.is_none_or(|(_, best_ground_y)| ground_y > best_ground_y) {
                nearest = Some((foothold.id, ground_y));
            }
        }
        nearest
    }

    pub(crate) fn has_wall_at_edge(&self, floor: Foothold, left: bool) -> bool {
        let (x, y) = floor.edge(left);
        self.get(floor.neighbor(left)).is_some_and(|wall| {
            f64::from(wall.x1) == x
                && wall.blocks_vertical_range(vertical_top(y), vertical_bottom(y))
        })
    }

    /// Follows a linked stair seam or an unlinked, continuous endpoint,
    /// preferring a flat continuation when a ramp branches from the same seam.
    pub(crate) fn floor_across_edge(
        &self,
        floor: Foothold,
        left: bool,
        target_x: f64,
    ) -> Option<Foothold> {
        let (edge_x, edge_y) = floor.edge(left);
        let direction = if left { -1.0 } else { 1.0 };
        let linked_id = floor.neighbor(left);
        // Some maps omit links at ramp endpoints. Compare linked and unlinked
        // candidates together so a linked ramp cannot hide a flat continuation.
        let bucket = self
            .floor_buckets
            .get(bucket_index(edge_x as i32, self.bucket_origin))?;
        let target_distance = (target_x - edge_x) * direction;
        std::iter::once(self.get(linked_id).copied())
            .flatten()
            .chain(bucket.iter().map(|&index| self.footholds[index]))
            .filter_map(|next| {
                let is_linked = next.id == linked_id;
                if next.is_wall() || (!is_linked && next.layer != floor.layer) {
                    return None;
                }
                let (entry_x, _) = next.edge(!left);
                let (exit_x, _) = next.edge(left);
                let entry_gap = (entry_x - edge_x) * direction;
                let seam_x = if entry_gap > 0.0 { entry_x } else { edge_x };
                let height = next.ground_y_extrapolated(seam_x) - edge_y;
                let continues_in_direction = (exit_x - edge_x) * direction > 0.0;
                let valid = if is_linked {
                    entry_gap.abs() <= 1.0
                        && entry_gap <= target_distance
                        && continues_in_direction
                        && (-1.0..=MAX_STAIR_STEP_DOWN).contains(&height)
                } else {
                    entry_x == edge_x && height.abs() <= 1.0
                };
                valid.then_some((
                    next,
                    height.abs(),
                    next.slope().abs(),
                    entry_gap.abs(),
                    is_linked,
                ))
            })
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then(left.2.total_cmp(&right.2))
                    .then(left.3.total_cmp(&right.3))
                    .then(right.4.cmp(&left.4))
            })
            .map(|(next, _, _, _, _)| next)
    }

    /// Finds the first floor crossed while moving between two points. This
    /// sweeps every floor bucket touched by the path, so a fast diagonal step
    /// cannot pass through a neighboring foothold between simulation ticks.
    pub(crate) fn find_swept_ground_contact(
        &self,
        start: (f64, f64),
        end: (f64, f64),
    ) -> Option<GroundContact> {
        let (start_x, start_y) = start;
        let (end_x, end_y) = end;
        if start == end {
            return None;
        }
        if !start_x.is_finite() || !start_y.is_finite() || !end_x.is_finite() || !end_y.is_finite()
        {
            return None;
        }

        let min_x = start_x.min(end_x).max(f64::from(i16::MIN)).floor() as i32;
        let max_x = start_x.max(end_x).min(f64::from(i16::MAX)).ceil() as i32;
        if min_x > max_x || max_x < self.bucket_origin {
            return None;
        }
        let first_bucket = if min_x <= self.bucket_origin {
            0
        } else {
            bucket_index(min_x, self.bucket_origin)
        };
        let last_bucket = bucket_index(max_x, self.bucket_origin).min(self.floor_buckets.len() - 1);
        let mut first_contact: Option<(f64, GroundContact)> = None;

        for bucket in first_bucket..=last_bucket {
            for &index in &self.floor_buckets[bucket] {
                let floor = self.footholds[index];
                let floor_bucket = bucket_index(i32::from(floor.left()), self.bucket_origin);
                if bucket != floor_bucket.max(first_bucket) {
                    continue;
                }
                if let Some(time) = floor_path_contact(floor, start, end)
                    && first_contact.as_ref().is_none_or(|(first_time, first)| {
                        time + CONTACT_EPSILON < *first_time
                            || (time - *first_time).abs() <= CONTACT_EPSILON
                                && floor.is_floor()
                                && !first.floor.is_floor()
                    })
                {
                    let x = if time == 1.0 {
                        end_x
                    } else {
                        start_x + (end_x - start_x) * time
                    };
                    let x = x.clamp(f64::from(floor.left()), f64::from(floor.right()));
                    let y = floor.ground_y_extrapolated(x);
                    first_contact = Some((time, GroundContact { floor, x, y }));
                }
            }
        }

        first_contact.map(|(_, contact)| contact)
    }

    /// Returns the legacy spawn height: one pixel above the nearest floor,
    /// or one pixel above the map's lower border when no floor is below.
    pub fn y_below(&self, x: i16, y: i16) -> i16 {
        self.find_below(f64::from(x), f64::from(y))
            .map(|(_, ground)| (ground as i16).saturating_sub(1))
            .unwrap_or_else(|| self.bounds.bottom.saturating_sub(1))
    }

    /// Resolves a linked foothold wall for horizontal movement. A wall on the
    /// current segment or its next two neighbors blocks movement; map bounds
    /// provide the final clamp, as in the v83 client.
    pub fn wall_for(&self, foothold_id: u16, left: bool, y: f64) -> f64 {
        let Some(current) = self.get(foothold_id).copied() else {
            return if left {
                f64::from(self.bounds.left_wall)
            } else {
                f64::from(self.bounds.right_wall)
            };
        };
        let (first_id, fallback) = if left {
            (current.previous, f64::from(current.left()))
        } else {
            (current.next, f64::from(current.right()))
        };
        let outer_bound = if left {
            f64::from(self.bounds.left_wall)
        } else {
            f64::from(self.bounds.right_wall)
        };
        let Some(first) = self.get(first_id).copied() else {
            return outer_bound;
        };
        if first.blocks_vertical_range(vertical_top(y), vertical_bottom(y)) {
            return fallback;
        }
        let second_id = if left { first.previous } else { first.next };
        if let Some(second) = self.get(second_id).copied() {
            if second.blocks_vertical_range(vertical_top(y), vertical_bottom(y)) {
                return if left {
                    f64::from(first.left())
                } else {
                    f64::from(first.right())
                };
            }
        }
        outer_bound
    }

    pub fn edge_for(&self, foothold_id: u16, left: bool) -> f64 {
        let Some(current) = self.get(foothold_id).copied() else {
            return if left {
                f64::from(self.bounds.left_wall)
            } else {
                f64::from(self.bounds.right_wall)
            };
        };
        let first_id = if left { current.previous } else { current.next };
        let Some(first) = self.get(first_id).copied() else {
            return if left {
                f64::from(current.left())
            } else {
                f64::from(current.right())
            };
        };
        let second_id = if left { first.previous } else { first.next };
        if self.get(second_id).is_none() {
            if left {
                f64::from(first.left())
            } else {
                f64::from(first.right())
            }
        } else if left {
            f64::from(self.bounds.left_wall)
        } else {
            f64::from(self.bounds.right_wall)
        }
    }
}

fn bucket_index(x: i32, origin: i32) -> usize {
    (x - origin).div_euclid(BUCKET_WIDTH) as usize
}

/// Returns the time where the motion crosses the floor from its upper side.
fn floor_path_contact(foothold: Foothold, start: (f64, f64), end: (f64, f64)) -> Option<f64> {
    let (start_x, start_y) = start;
    let (end_x, end_y) = end;
    let delta_x = end_x - start_x;
    let approach = end_y - start_y - foothold.slope() * delta_x;
    if approach <= 0.0 {
        return None;
    }

    let (entry, exit) = if delta_x == 0.0 {
        foothold.ground_y(start_x)?;
        (0.0, 1.0)
    } else {
        let left_time = (f64::from(foothold.left()) - start_x) / delta_x;
        let right_time = (f64::from(foothold.right()) - start_x) / delta_x;
        (
            left_time.min(right_time).max(0.0),
            left_time.max(right_time).min(1.0),
        )
    };
    if entry > exit {
        return None;
    }

    let entry_x = start_x + delta_x * entry;
    let entry_y = start_y + (end_y - start_y) * entry;
    let separation = entry_y - foothold.ground_y_extrapolated(entry_x);
    let exit_separation = separation + approach * (exit - entry);
    if separation > CONTACT_EPSILON || exit_separation < -CONTACT_EPSILON {
        return None;
    }
    Some((entry - separation / approach).clamp(entry, exit))
}

fn make_bounds(min_x: i32, max_x: i32, min_y: i32, max_y: i32) -> MapBounds {
    if min_x == i32::MAX {
        return MapBounds::default();
    }
    let mut left_wall = (min_x + 25).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    let mut right_wall = (max_x - 25).clamp(i32::from(i16::MIN), i32::from(i16::MAX));
    if left_wall > right_wall {
        let center = min_x + (max_x - min_x) / 2;
        left_wall = center;
        right_wall = center;
    }
    MapBounds {
        left_wall: left_wall as i16,
        right_wall: right_wall as i16,
        top: (min_y - 300).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
        bottom: (max_y + 100).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16,
    }
}

fn vertical_top(y: f64) -> i16 {
    (y.trunc() as i32 - 50).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

fn vertical_bottom(y: f64) -> i16 {
    (y.trunc() as i32 - 1).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}
