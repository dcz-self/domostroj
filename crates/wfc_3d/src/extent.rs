/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */

use baustein::indices::{usize_to_i32_arr, to_usize_arr, Index, VoxelUnits};
use baustein::re::{ConstShape, RuntimeShape, Shape};
use baustein::world::FlatPaddedGridCuboid;
use std::cmp;

// used traits
use baustein::traits::Cuboid;

#[derive(Clone, Copy, Debug)]
pub struct Extent {
    /// Each dimension is strictly lower than in end,
    /// or both are [0, 0, 0],
    /// in which case, the extent is empty.
    start: Index,
    end: Index,
}

impl Extent {
    pub fn new(start: Index, end: Index) -> Self {
        if start.x() < end.x()
            && start.y() < end.y()
            && start.z() < end.z()
        {
            Self { start, end }
        } else {
            Self {
                start: [0, 0, 0].into(),
                end: [0, 0, 0].into(),
            }
        }
    }
    pub fn iter(&self) -> Iter {
        Iter {
            i: 0,
            shape: RuntimeShape::new(to_usize_arr((self.end - self.start).0)),
            offset: self.start,
        }
    }
    pub fn intersection(&self, other: &Extent) -> Extent {
        let start = [
            cmp::max(self.start.x(), other.start.x()),
            cmp::max(self.start.y(), other.start.y()),
            cmp::max(self.start.z(), other.start.z()),
        ].into();
        let end = [
            cmp::min(self.end.x(), other.end.x()),
            cmp::min(self.end.y(), other.end.y()),
            cmp::min(self.end.z(), other.end.z()),
        ].into();
        Extent::new(
            start,
            end,
        )
    }
    pub fn get_dimensions(&self) -> [usize; 3] {
        to_usize_arr((self.end - self.start).0)
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
    
    fn get_stamps_containing<StampShape: ConstShape>(&self, index: Index) -> Extent {
        self.get_stamps_extent::<StampShape>()
            .intersection(&Extent::new(
                index
                    - VoxelUnits(usize_to_i32_arr(<StampShape as ConstShape>::ARRAY))
                    + VoxelUnits([1, 1, 1].into()),
                index + VoxelUnits([1, 1, 1].into()),
            ))
    }
}

impl<V, Shape> Stamped for FlatPaddedGridCuboid<V, Shape>
    where Shape: ConstShape,
{
    fn get_stamps_extent<StampShape: ConstShape>(&self) -> Extent {
        Extent::new(
            self.get_offset(),
            self.get_beyond_opposite_corner()
                - VoxelUnits(usize_to_i32_arr(<StampShape as ConstShape>::ARRAY))
                + VoxelUnits([1, 1, 1].into()),
        )
    }
}


impl Stamped for Extent {
    fn get_stamps_extent<StampShape: ConstShape>(&self) -> Extent {
        Extent::new(
            self.start,
            self.end
                - VoxelUnits(usize_to_i32_arr(<StampShape as ConstShape>::ARRAY))
                + VoxelUnits([1, 1, 1].into()),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use baustein::re::{ConstAnyShape, ConstPow2Shape};

    #[test]
    fn edge() {
        type Cuboid<V> = FlatPaddedGridCuboid<V, ConstAnyShape<4, 4, 4>>;
        let mut cube = Cuboid::<()>::new([0, 0, 0].into());
        let stamp_extent = cube.get_stamps_extent::<ConstAnyShape<2, 2, 2>>();
        stamp_extent.iter()
            .for_each(|i| {
                cube.set(i, ()).unwrap();
                cube.set(i + VoxelUnits([1, 1, 1].into()), ()).unwrap();
            });
        assert_eq!(stamp_extent.iter().count(), 3*3*3);
    }

    
    #[test]
    fn containing() {
        type StampShape = ConstAnyShape<2, 2, 2>;
        let extent = Extent {
            start: [0, 0, 0].into(),
            end: [5, 5, 5].into(),
        };
        let target = [2, 2, 2].into();
        extent
            .get_stamps_containing::<StampShape>(target)
            .iter()
            .find(|i| *i == target)
            .unwrap();
    }
}
