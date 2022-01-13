/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */

use baustein::indices::{usize_to_i32_arr, to_usize_arr, Index, VoxelUnits};
use baustein::re::{ConstShape, RuntimeShape, Shape};
use baustein::world::FlatPaddedGridCuboid;

// used traits
use baustein::traits::Extent as _;

pub struct Extent {
    /// Each dimension is strictly lower than in end.
    start: Index,
    end: Index,
}

impl Extent {
    pub fn new(start: Index, end: Index) -> Self {
        Self { start, end }
    }
    pub fn iter(&self) -> Iter {
        Iter {
            i: 0,
            shape: RuntimeShape::new(to_usize_arr((self.end - self.start).0)),
            offset: self.start,
        }
    }
}


pub struct Iter {
    i: usize,
    shape: RuntimeShape,
    offset: Index,
}

impl Iterator for Iter {
    type Item = Index;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.shape.size() { None }
        else {
            let ret = self.offset + VoxelUnits(usize_to_i32_arr(self.shape.delinearize(self.i)));
            self.i += 1;
            Some(ret)
        }
    }
}

pub trait Stamped {
    fn get_stamps_extent<StampShape: ConstShape>(&self) -> Extent;
}

impl<V, Shape> Stamped for FlatPaddedGridCuboid<V, Shape>
    where Shape: ConstShape,
{
    fn get_stamps_extent<StampShape: ConstShape>(&self) -> Extent {
        Extent {
            start: self.get_offset(),
            end: self.get_beyond_opposite_corner()
                - VoxelUnits(usize_to_i32_arr(<StampShape as ConstShape>::ARRAY)),
        }
    }
}
