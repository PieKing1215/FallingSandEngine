use std::ops::Range;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rect<T> {
    pub x1: T,
    pub y1: T,
    pub x2: T,
    pub y2: T,
}

impl<T: Copy + std::ops::Add<Output = T> + std::cmp::PartialOrd + std::ops::Sub<Output = T>>
    Rect<T>
{
    #[inline]
    pub fn new(x1: impl Into<T>, y1: impl Into<T>, x2: impl Into<T>, y2: impl Into<T>) -> Self {
        Self {
            x1: x1.into(),
            y1: y1.into(),
            x2: x2.into(),
            y2: y2.into(),
        }
    }

    #[inline]
    pub fn new_wh(x1: impl Into<T>, y1: impl Into<T>, w: impl Into<T>, h: impl Into<T>) -> Self {
        let x1 = x1.into();
        let y1 = y1.into();
        let x2 = x1 + w.into();
        let y2 = y1 + h.into();

        Self { x1, y1, x2, y2 }
    }

    #[inline]
    pub fn left(&self) -> T {
        self.x1
    }

    #[allow(clippy::cast_possible_wrap)]
    #[inline]
    pub fn right(&self) -> T {
        self.x2
    }

    #[inline]
    pub fn top(&self) -> T {
        self.y1
    }

    #[allow(clippy::cast_possible_wrap)]
    #[inline]
    pub fn bottom(&self) -> T {
        self.y2
    }

    #[inline]
    pub fn width(&self) -> T {
        self.x2 - self.x1
    }

    #[inline]
    pub fn height(&self) -> T {
        self.y2 - self.y1
    }

    #[inline]
    pub fn range_lr(&self) -> Range<T> {
        self.left()..self.right()
    }

    #[inline]
    pub fn range_tb(&self) -> Range<T> {
        self.top()..self.bottom()
    }

    #[inline]
    #[must_use]
    pub fn inflated(&self, add: T) -> Self {
        Self {
            x1: self.x1 - add,
            y1: self.y1 - add,
            x2: self.x2 + add,
            y2: self.y2 + add,
        }
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        !(self.bottom() < other.top()
            || self.top() > other.bottom()
            || self.right() < other.left()
            || self.left() > other.right())
    }

    #[inline]
    pub fn contains_point(&self, point: (impl Into<T>, impl Into<T>)) -> bool {
        let (x, y) = (point.0.into(), point.1.into());
        x >= self.left() && y >= self.top() && x <= self.right() && y <= self.bottom()
    }

    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let x1 = partial_min(self.x1, other.x1);
        let y1 = partial_min(self.y1, other.y1);
        let x2 = partial_max(self.x2, other.x2);
        let y2 = partial_max(self.y2, other.y2);
        Self { x1, y1, x2, y2 }
    }
}

fn partial_max<T: PartialOrd>(a: T, b: T) -> T {
    if b > a {
        b
    } else {
        a
    }
}

fn partial_min<T: PartialOrd>(a: T, b: T) -> T {
    if b > a {
        a
    } else {
        b
    }
}

impl Rect<i32> {
    pub fn into_f32(self) -> Rect<f32> {
        Rect::new(
            self.x1 as f32,
            self.y1 as f32,
            self.x2 as f32,
            self.y2 as f32,
        )
    }
}
