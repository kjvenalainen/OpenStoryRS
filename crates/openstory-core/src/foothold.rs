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

    /// Evaluates the foothold's line without checking its horizontal extent.
    /// Legacy physics calls `Foothold::ground_below` directly, which extrapolates
    /// the segment at both endpoints during movement and foothold transitions.
    pub(crate) fn ground_y_extrapolated(self, x: f64) -> f64 {
        if self.is_floor() {
            return f64::from(self.y1);
        }
        self.slope() * (x - f64::from(self.x1)) + f64::from(self.y1)
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
    /// coordinate system. Equal-height matches select the last source candidate,
    /// matching the legacy lookup's overwrite-on-equal behavior.
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
            if ground_y >= y && nearest.is_none_or(|(_, best_ground_y)| ground_y <= best_ground_y) {
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
