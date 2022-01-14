/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
/*! Stamps and stamp collections.
 * 
 * A stamp is an allowed arrangement of voxels.
 *
 * The `ViewStamp` type serves a double role of also being a stamp-comparison view for the Wave.
 * It's useful for checking if the area can collapse into a stamp.
 */
use crate::{Superposition, VoxelId};
use crate::extent::Stamped;
use baustein::indices::{ usize_to_i32_arr, Index, VoxelUnits };
use baustein::re::ConstShape;
use baustein::traits::Space;
use baustein::world::FlatPaddedGridCuboid;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::hash::{ Hash, Hasher };
use std::marker::PhantomData;

/// Wrapping modes: 
pub struct Wrapping;

/// The dafault space used for stamps.
/// `FlatPaddedGridCuboid` is just an array in memory, which should make it fast.
type StampSpace<Shape> = FlatPaddedGridCuboid<VoxelId, Shape>;
/// Stamp type
pub type ST<'a, StampShape, Shape> = ViewStamp<'a, StampShape, StampSpace<Shape>>;

/// Returns all stamps fitting within the cuboid,
/// given wrapping modes.
/// Stamps are of a static size. Not sure if that's the best idea.
pub fn gather_stamps<Shape, StampShape>(
    cuboid: &StampSpace<Shape>,
    wrapping: Wrapping,
) -> HashMap<ViewStamp<StampShape, StampSpace<Shape>>, usize>
where
    Shape: ConstShape,
    StampShape: ConstShape,
{
    let views = cuboid
        .get_stamps_extent::<StampShape>()
        .iter()
        .map(|idx| ViewStamp::<StampShape, StampSpace<Shape>>::new(cuboid, idx));
    popcount(views)
}

fn popcount<T: Hash + Eq>(i: impl Iterator<Item=T>) -> HashMap<T, usize> {
    i.fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}

/// The collection of Stamps from a single source.
///
/// Currently stamps are just a &[ViewStamp] where each refers to a Space.
/// Because a Space will usually have large dimensions,
/// the each vertical slice of a ViewStamp is in a different cache line.
/// If stamps are few, it might be faster to put their voxels in a single cache line
/// by keeping each in a contiguous buffer.
/// If stamps are small and they are iterated consecutively (they usually are),
/// try keeping them all in a single large buffer.
/// This needs to get benchmarked:
/// `perf stat` or https://stackoverflow.com/questions/49242919/profiling-cache-evicition
/// https://stackoverflow.com/questions/18172353/how-to-catch-the-l3-cache-hits-and-misses-by-perf-tool-in-linux
pub struct StampCollection<'a, StampShape: ConstShape, SourceShape: ConstShape>(
    Vec<(ST<'a, StampShape, SourceShape>, usize)>,
);

impl<'a, StampShape: ConstShape, SourceShape: ConstShape> StampCollection<'a, StampShape, SourceShape> {
    pub fn new(stamps: Vec<(ST<'a, StampShape, SourceShape>, usize)>) -> Self {
        Self(stamps)
    }

    fn get_total_occurrences(&self) -> usize {
        self.0.iter().map(|(_s, v)| *v).sum()
    }

    pub fn get_collapse_outcomes<S, const C: u8>(&'a self, view: &ViewStamp<StampShape, S>)
        -> CollapseOutcomes<'a, StampShape, StampSpace<SourceShape>>
    where
        S: Space<Voxel=Superposition<C>>,
    {
        let matches = self.0.iter()
            .map(|(stamp, _occurrences)| stamp)
            .filter(|stamp| view.allows(stamp));
        let mut outcome = CollapseOutcomes::None;
        for stamp in matches {
            match outcome {
                CollapseOutcomes::None => {
                    outcome = CollapseOutcomes::One(stamp);
                },
                CollapseOutcomes::One(_) => {
                    outcome = CollapseOutcomes::Multiple;
                    break;
                },
                _ => { unreachable!(); },
            }
        }
        outcome
    }
}

pub enum CollapseOutcomes<'a, Shape: ConstShape, S: Space> {
    One(&'a ViewStamp<'a, Shape, S>),
    None,
    Multiple,
}

/// An index starting from 0
#[derive(Copy, Clone)]
pub struct StampIndex(pub [usize; 3]);

/// A 0-indexed fragment of a space, with static dimensions.
/// Compared by its contents.
/// Hashing and comparison allocate :/
/// This is stored in 8 + 3*4(index) = 20 bytes.
/// 20 bytes is still 2×2×5 voxels. Still less than 3×3×3.
/// So don't use this for small stamps.
/// If index is limited to 0..256 in each dimension,
/// then 8 + 3*1 = 11 bytes (+1 padding?).
/// Less than 2×2×3 voxels.
pub struct ViewStamp<'a, Shape: ConstShape, S: Space + 'a> {
    space: &'a S,
    pub offset: Index,
    shape: PhantomData<Shape>,
}

impl<'a, V, Shape, S> ViewStamp<'a, Shape, S>
    where
    V: Copy,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    pub fn new(space: &'a S, offset: Index) -> Self {
        Self {
            space,
            offset,
            shape: Default::default(),
        }
    }
    
    pub fn get(&self, index: StampIndex) -> V {
        self.space.get(self.offset + VoxelUnits(usize_to_i32_arr(index.0).into()))
    }

    fn get_samples(&self) -> Vec<V> {
        let mut out = Vec::with_capacity(Shape::SIZE);
        self.visit_indices(|i| out.push(self.get(i)));
        out
    }

    pub fn visit_indices<F: FnMut(StampIndex)>(&self, mut f: F) {
        for i in 0..Shape::SIZE {
            f(StampIndex(Shape::delinearize(i)));
        }
    }
}

impl<'a, Shape, S, const C: u8> ViewStamp<'a, Shape, S>
    where
    Shape: ConstShape,
    S: Space<Voxel=Superposition<C>> + 'a,
{
    pub fn allows<U>(&self, stamp: &ViewStamp<Shape, U>) -> bool
        where U: Space<Voxel=VoxelId>
    {
        for i in 0..Shape::SIZE {
            let index = StampIndex(Shape::delinearize(i));
            if !self.get(index).allows(stamp.get(index)) {
                return false;
            }
        }
        return true;
    }
}

impl<'a, Shape, S> Clone for ViewStamp<'a, Shape, S>
    where
    Shape: ConstShape,
    S: Space + 'a,
{
    fn clone(&self) -> Self {
        Self {
            space: self.space,
            offset: self.offset,
            shape: Default::default(),
        }
    }
}

impl<'a, V, Shape, S> Hash for ViewStamp<'a, Shape, S>
    where
    V: Copy + Hash,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.get_samples().hash(hasher)
    }
}

impl<'a, V, Shape, S> cmp::PartialEq for ViewStamp<'a, Shape, S>
    where
    V: Copy + PartialEq,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn eq(&self, other: &Self) -> bool {
        self.get_samples() == other.get_samples()
    }
}

impl<'a, V, Shape, S> cmp::Eq for ViewStamp<'a, Shape, S>
    where
    V: Copy + Eq,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{}

impl<'a, V, Shape, S> fmt::Debug for ViewStamp<'a, Shape, S>
where
    V: fmt::Debug + Copy,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        (self.offset, self.get_samples()).fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use baustein::re::ConstAnyShape;

    #[test]
    fn stamps() {
        type Shape = ConstAnyShape<8, 8, 8>;
        type StampShape = ConstAnyShape<2, 2, 2>;
        let world = FlatPaddedGridCuboid::<VoxelId, Shape>::new([0, 0, 0].into());
        let stamps = gather_stamps::<_, StampShape>(&world, Wrapping);
        assert_eq!(stamps.len(), 1);
        assert_eq!(stamps.into_values().collect::<Vec<_>>(), vec![7*7*7]);
    }

    #[test]
    fn stamps2() {
        type Shape = ConstAnyShape<4, 4, 4>;
        type StampShape = ConstAnyShape<2, 2, 2>;
        
        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Split into 2 areas
        let world = extent.map_index(|i, _| {
            if i.y() < 2 { 1 }
            else { 0 }
        });
        let world: FlatPaddedGridCuboid<u8, Shape> = world.into();
        
        let stamps = gather_stamps::<_, StampShape>(&world, Wrapping);
        assert_eq!(dbg!(&stamps).len(), 3);
        // all 1
        assert_eq!(stamps.get(&ViewStamp::new(&world, [0, 0, 0].into())).map(|x| *x), Some(3*3));
        // bottom 1, top 0
        assert_eq!(stamps.get(&ViewStamp::new(&world, [0, 1, 0].into())).map(|x| *x), Some(3*3));
        // all 0
        assert_eq!(stamps.get(&ViewStamp::new(&world, [0, 2, 0].into())).map(|x| *x), Some(3*3));
    }
}
