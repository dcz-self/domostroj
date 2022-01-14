/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
/*! Wave containers.
 */

use crate::{Superposition, VoxelId};
use crate::extent::Extent;
use crate::stamp::{CollapseOutcomes, StampCollection, ViewStamp};

use baustein::indices::{usize_to_i32_arr, Index, VoxelUnits};
use baustein::re::ConstShape;
use baustein::world::{FlatPaddedGridCuboid, OutOfBounds};

// Used traits
use crate::extent::Stamped;
use baustein::traits::Space;
use baustein::traits::Cuboid;

/// Doesn't do anything special for you. Just a dumb container.
/// Like all waves, it handles propagating collapses.
pub struct Naive<S: ConstShape, const C: u8> {
    world: FlatPaddedGridCuboid<Superposition<C>, S>,
}

impl<S: ConstShape, const C: u8> Naive<S, C> {
    pub fn new<StampShape: ConstShape, SourceShape: ConstShape>(
        world: FlatPaddedGridCuboid<Superposition<C>, S>,
        stamps: &StampCollection<StampShape, SourceShape>,
    ) -> Self {
        let mut new = Self { world };
        // `world` is not constrained in any way, so before forcing collapse,
        // let's try to follow the constraints it already sets.
        new.collapse(&new.get_extent(), &stamps);
        new
    }

    fn get_offset(&self) -> Index {
        self.world.get_offset()
    }
    fn get_beyond_opposite_corner(&self) -> Index {
        self.world.get_beyond_opposite_corner()
    }
    fn get_extent(&self) -> Extent {
        Extent::new(self.get_offset(), self.get_beyond_opposite_corner())
    }

    pub fn get_world(&self) -> &FlatPaddedGridCuboid<Superposition<C>, S> {
        &self.world
    }

    fn get(&self, index: Index) -> Superposition<C> {
        self.world.get(index)
    }

    /// This can either lock or unlock possibilities.
    /// This is intentional to allow interactivity.
    fn set<StampShape: ConstShape, SourceShape: ConstShape>(
        &mut self,
        index: Index,
        value: Superposition<C>,
        stamps: &StampCollection<StampShape, SourceShape>,
    ) -> Result<(), OutOfBounds> {
        if self.world.get(index) == value {
            return Ok(())
        }
        self.world.set(index, value)?;
        self.collapse(
            &self.get_extent().get_stamps_containing::<StampShape>(index),
            stamps,
        );
        Ok(())
    }

    /// Logical AND to apply to a voxel.
    fn limit<StampShape: ConstShape, SourceShape: ConstShape>(
        &mut self,
        index: Index,
        value: Superposition<C>,
        stamps: &StampCollection<StampShape, SourceShape>,
    ) -> Result<(), OutOfBounds> {
        //if self.get(
        // FIXME
        self.set(index, value, stamps)
    }

    pub fn limit_stamp<StampShape: ConstShape, SourceSpace: Space<Voxel=VoxelId>, SourceShape: ConstShape>(
        &mut self,
        index: Index,
        stamp: &ViewStamp<StampShape, SourceSpace>,
        stamps: &StampCollection<StampShape, SourceShape>,
    ) -> Result<(), OutOfBounds> {
        stamp.visit_indices(|stamp_index| {
            let voxel = stamp.get(stamp_index);
            let new = Superposition::<C>::only(voxel);
            let index = index + VoxelUnits(usize_to_i32_arr(stamp_index.0));
            if new != self.get(index) {
                println!("Collapsing {:?} to {}", index, voxel);
                self.limit(index, new, stamps)?;
            }
            Ok(())
        })
    }

    /// Propagates collapse. Totally naive approach, depth-first.
    pub fn collapse<StampShape: ConstShape, SourceShape: ConstShape>(
        &mut self,
        extent: &Extent,
        stamps: &StampCollection<StampShape, SourceShape>,
    ) {
        let stamp_extent = self.get_extent().get_stamps_extent::<StampShape>();
        for index in extent.intersection(&stamp_extent).iter() {
            let collapse = {
                let view = ViewStamp::<StampShape, _>::new(&self.world, index);
                stamps.get_collapse_outcomes(&view)
            };
            if let CollapseOutcomes::One(stamp) = collapse {
                // This can only fail if the stamp is out of bounds,
                // but we check it.
                self.limit_stamp(index, stamp, stamps).unwrap();
            }
        }
    }

    pub fn into_space(self) -> FlatPaddedGridCuboid<Superposition<C>, S> {
        self.world
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::stamp::{gather_stamps, Wrapping};
    use baustein::re::ConstAnyShape;

    #[test]
    fn cllapse_one() {
        type Shape = ConstAnyShape<4, 4, 4>;
        type StampShape = ConstAnyShape<2, 2, 2>;

        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Split into 2 areas
        let world = extent.map_index(|i, _| {
            if i.y() < 2 { 1 }
            else { 0 }
        });
        let world: FlatPaddedGridCuboid<u8, Shape> = world.into();
        let stamps = StampCollection::new(
            gather_stamps::<_, StampShape>(&world, Wrapping)
                .into_iter()
                .collect()
        );

        let mut wave = Naive {
            world: FlatPaddedGridCuboid::<Superposition<2>, Shape>::new([0, 0, 0].into())
        };
        // Collapse the middle of the edge.
        wave.set([0, 1, 0].into(), Superposition::only(1), &stamps).unwrap();
        assert_eq!(wave.get([0, 0, 0].into()), Superposition::only(1));
        assert_eq!(wave.get([0, 3, 0].into()), Superposition::FREE);
        assert_eq!(wave.get([3, 3, 3].into()), Superposition::FREE);
        wave.set([0, 2, 0].into(), Superposition::only(0), &stamps).unwrap();
        assert_eq!(wave.get([0, 3, 0].into()), Superposition::only(0));
        assert_eq!(wave.get([3, 3, 3].into()), Superposition::only(0));
    }

    #[test]
    fn collapse_impossible() {
        type Shape = ConstAnyShape<4, 4, 4>;
        type StampShape = ConstAnyShape<2, 2, 2>;

        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Split into 2 areas
        let world = extent.map_index(|i, _| {
            if i.y() < 2 { 1 }
            else { 0 }
        });
        let world: FlatPaddedGridCuboid<u8, Shape> = world.into();
        let stamps = StampCollection::new(
            gather_stamps::<_, StampShape>(&world, Wrapping)
                .into_iter()
                .collect()
        );

        let mut wave = Naive {
            world: FlatPaddedGridCuboid::<Superposition<2>, Shape>::new([0, 0, 0].into())
        };
        // Collapse all below
        wave.set([0, 2, 0].into(), Superposition::only(1), &stamps).unwrap();
        assert_eq!(wave.get([0, 0, 0].into()), Superposition::only(1));
        assert_eq!(wave.get([3, 3, 3].into()), Superposition::FREE);
        // And then alter what's below. But that gives an impossible outcome!
        wave.set([0, 1, 0].into(), Superposition::only(0), &stamps).unwrap();
        // This gets stopped early, as there's nothing to change due to the new information.
        assert_eq!(wave.get([0, 3, 0].into()), Superposition::FREE);
        assert_eq!(wave.get([3, 3, 3].into()), Superposition::FREE);
    }
}
