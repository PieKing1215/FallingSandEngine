pub mod networking;
pub mod world;

mod settings;
use std::ops::Range;

use serde::{Deserialize, Serialize};
pub use settings::*;
pub mod commands;

mod file_helper;
pub use file_helper::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    #[inline]
    pub fn new(x: impl Into<i32>, y: impl Into<i32>, w: impl Into<u32>, h: impl Into<u32>) -> Self {
        Self { x: x.into(), y: y.into(), w: w.into(), h: h.into() }
    }

    #[inline]
    pub fn new_points(
        x1: impl Into<i32>,
        y1: impl Into<i32>,
        x2: impl Into<i32>,
        y2: impl Into<i32>,
    ) -> Self {
        let x1 = x1.into();
        let y1 = y1.into();
        let x2 = x2.into();
        let y2 = y2.into();

        Self {
            x: x1.min(x2),
            y: y1.min(y2),
            w: (x1.max(x2) - x1.min(x2)) as u32,
            h: (y1.max(y2) - y1.min(y2)) as u32,
        }
    }

    pub const fn left(&self) -> i32 {
        self.x
    }

    #[allow(clippy::cast_possible_wrap)]
    pub const fn right(&self) -> i32 {
        self.x + self.w as i32
    }

    pub const fn top(&self) -> i32 {
        self.y
    }

    #[allow(clippy::cast_possible_wrap)]
    pub const fn bottom(&self) -> i32 {
        self.y + self.h as i32
    }

    pub fn range_lr(&self) -> Range<i32> {
        self.left()..self.right()
    }

    pub fn range_tb(&self) -> Range<i32> {
        self.top()..self.bottom()
    }

    pub const fn intersects(&self, other: &Self) -> bool {
        !(self.bottom() < other.top()
            || self.top() > other.bottom()
            || self.right() < other.left()
            || self.left() > other.right())
    }

    pub fn contains_point(&self, point: (impl Into<i32>, impl Into<i32>)) -> bool {
        let (x, y) = (point.0.into(), point.1.into());
        x >= self.left() && y >= self.top() && x <= self.right() && y <= self.bottom()
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        Self {
            x,
            y,
            w: (self.right().max(other.right()) - x) as u32,
            h: (self.bottom().max(other.bottom()) - y) as u32,
        }
    }
}
