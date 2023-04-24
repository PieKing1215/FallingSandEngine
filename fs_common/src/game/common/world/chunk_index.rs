use std::ops::{Deref, Index, IndexMut};

use crate::game::common::world::{CHUNK_AREA, CHUNK_SIZE};

/// Local pixel position from the top left of a chunk.
///
/// X and Y are within `0..`[`CHUNK_SIZE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkLocalPosition((u16, u16));

/// Local pixel index from the top left of a chunk.
///
/// Index is within `0..`[`CHUNK_AREA`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkLocalIndex(usize);

// ChunkLocalPosition impls

impl ChunkLocalPosition {
    #[inline]
    pub fn new(x: u16, y: u16) -> Result<Self, <Self as TryFrom<(u16, u16)>>::Error> {
        (x, y).try_into()
    }

    /// # Safety
    /// `x` and `y` must both be less than [`CHUNK_SIZE`] in order to uphold invariants.
    #[inline]
    pub const unsafe fn new_unchecked(x: u16, y: u16) -> Self {
        debug_assert!(x < CHUNK_SIZE);
        debug_assert!(y < CHUNK_SIZE);
        Self((x, y))
    }

    #[inline]
    pub fn x(&self) -> u16 {
        self.0 .0
    }

    #[inline]
    pub fn y(&self) -> u16 {
        self.0 .1
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        (0..CHUNK_SIZE).flat_map(|x| {
            (0..CHUNK_SIZE).map(move |y| {
                // Safety: x and y are generated from the valid range
                unsafe { Self::new_unchecked(x, y) }
            })
        })
    }
}

impl TryFrom<(u16, u16)> for ChunkLocalPosition {
    type Error = String; // TODO: custom error type

    #[inline]
    fn try_from(value: (u16, u16)) -> Result<Self, Self::Error> {
        if value.0 < CHUNK_SIZE && value.1 < CHUNK_SIZE {
            Ok(Self(value))
        } else {
            Err(format!("Invalid value for ChunkLocalPosition: {value:?}"))
        }
    }
}

impl From<ChunkLocalIndex> for ChunkLocalPosition {
    #[inline]
    fn from(value: ChunkLocalIndex) -> Self {
        Self((
            (value.idx() % (CHUNK_SIZE as usize)) as _,
            (value.idx() / (CHUNK_SIZE as usize)) as _,
        ))
    }
}

impl Deref for ChunkLocalPosition {
    type Target = (u16, u16);

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ChunkLocalIndex impls

impl ChunkLocalIndex {
    #[inline]
    pub fn new(idx: usize) -> Result<Self, <Self as TryFrom<usize>>::Error> {
        idx.try_into()
    }

    /// # Safety
    /// `idx` must be less than [`CHUNK_AREA`] in order to uphold invariants.
    #[inline]
    pub unsafe fn new_unchecked(idx: usize) -> Self {
        debug_assert!(idx < CHUNK_AREA);
        idx.try_into().unwrap_unchecked()
    }

    #[inline]
    pub fn idx(&self) -> usize {
        self.0
    }

    pub fn iter() -> impl Iterator<Item = Self> {
        (0..CHUNK_AREA).map(|idx| {
            // Safety: x and y are generated from the valid range
            unsafe { Self::new_unchecked(idx) }
        })
    }
}

impl TryFrom<usize> for ChunkLocalIndex {
    type Error = String; // TODO: custom error type

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value < CHUNK_AREA {
            Ok(Self(value))
        } else {
            Err(format!("Invalid value for ChunkLocalIndex: {value:?}"))
        }
    }
}

impl From<ChunkLocalPosition> for ChunkLocalIndex {
    #[inline]
    fn from(value: ChunkLocalPosition) -> Self {
        Self(value.x() as usize + (value.y() as usize) * (CHUNK_SIZE as usize))
    }
}

impl Deref for ChunkLocalIndex {
    type Target = usize;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Index<ChunkLocalIndex> for [T; CHUNK_AREA] {
    type Output = T;

    fn index(&self, index: ChunkLocalIndex) -> &Self::Output {
        // Safety: ChunkLocalIndex is guaranteed to be `0..CHUNK_AREA`
        unsafe { self.get_unchecked(index.idx()) }
    }
}

impl<T> IndexMut<ChunkLocalIndex> for [T; CHUNK_AREA] {
    fn index_mut(&mut self, index: ChunkLocalIndex) -> &mut Self::Output {
        // Safety: ChunkLocalIndex is guaranteed to be `0..CHUNK_AREA`
        unsafe { self.get_unchecked_mut(index.idx()) }
    }
}

impl<T> Index<ChunkLocalPosition> for [T; CHUNK_AREA] {
    type Output = T;

    fn index(&self, index: ChunkLocalPosition) -> &Self::Output {
        let i: ChunkLocalIndex = index.into();
        &self[i]
    }
}

impl<T> IndexMut<ChunkLocalPosition> for [T; CHUNK_AREA] {
    fn index_mut(&mut self, index: ChunkLocalPosition) -> &mut Self::Output {
        let i: ChunkLocalIndex = index.into();
        &mut self[i]
    }
}
