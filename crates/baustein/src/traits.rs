/*! Traits for easy access to voxels */
use feldspar_core::glam::IVec3;
use feldspar_map::units::{ChunkUnits, VoxelUnits};
use ndshape;

use std::ops;

// Traits
use ndshape::ConstShape;


/// Delberately not public inside,
/// to be able to replace it in the future with a chunk+voxel combo
#[derive(Clone, Copy, Debug)]
pub struct WorldIndex(IVec3);

impl WorldIndex {
    pub fn new(offset: VoxelUnits<IVec3>) -> Self {
        Self(offset.0)
    }
}

impl ops::Index<usize> for WorldIndex {
    type Output = i32;
    fn index(&self, offset: usize) -> &i32 {
        &self.0[offset]
    }
}

impl From<[i32; 3]> for WorldIndex {
    fn from(coords: [i32; 3]) -> Self {
        Self(coords.into())
    }
}

impl From<IVec3> for WorldIndex {
    fn from(coords: IVec3) -> Self {
        Self(coords.into())
    }
}

impl Into<[i32; 3]> for WorldIndex {
    fn into(self) -> [i32; 3] {
        self.0.into()
    }
}

impl Into<IVec3> for WorldIndex {
    fn into(self) -> IVec3 {
        self.0.into()
    }
}

impl ops::Sub<VoxelUnits<IVec3>> for WorldIndex {
    type Output = WorldIndex;
    fn sub(self, s: VoxelUnits<IVec3>) -> Self::Output {
        WorldIndex(self.0 - s.0)
    }
}

impl ops::Add<VoxelUnits<IVec3>> for WorldIndex {
    type Output = WorldIndex;
    fn add(self, s: VoxelUnits<IVec3>) -> Self::Output {
        WorldIndex(self.0 + s.0)
    }
}

pub type Index = WorldIndex;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct ChunkIndex(IVec3);

pub type ChunkShape = ndshape::ConstShape3u32<16, 16, 16>;

/// This is really slow, we already know chunk coords are pow2.
/// Thankfully the threshold is known, so this will likely optimize out.
fn trunc(v: i32, thr: u32) -> i32 {
    let r = v.rem_euclid(thr as i32);
    v - r
}

impl ChunkIndex {
    fn center() -> Self {
        Self(IVec3::new(0, 0, 0))
    }

    /// Returns the index of the chunk, to which the world index belongs.
    pub fn new_encompassing(index: WorldIndex) -> Self {
        Self(IVec3::new(
            trunc(index.0[0], ChunkShape::ARRAY[0]),
            trunc(index.0[1], ChunkShape::ARRAY[1]),
            trunc(index.0[2], ChunkShape::ARRAY[2]),
        ))
    }

    /// Returns the offset (minimum in all dimensions),
    /// at which this chunk begins.
    pub fn get_world_offset(&self) -> WorldIndex {
        self.0.into()
    }

    /// Offset relative to the beginning of the chunk.
    pub fn get_internal_offset(&self, index: WorldIndex) -> VoxelUnits<IVec3> {
        VoxelUnits(index.0 - Self::new_encompassing(index).0)
    }
}

/// Access to elements of a 3d cuboid region of voxels.
/// Should be used for arbitrary (small) cuboids, may be used for the world.
/// It cannot be modified in place, instead replace the entire chunk.
pub trait Space {
    type Voxel: Copy;
    
    fn get(&self, offset: Index) -> Self::Voxel;
    /// Mapping methods may skip some empty areas.
    fn map<'a, U: Copy, F: Fn(Self::Voxel) -> U>(&'a self, f: F) -> Map<&'a Self, F> {
        Map {
            space: self,
            f,
        }
    }

    fn map_index<'a, U: Copy, F>(&'a self, f: F) -> MapIndex<&'a Self, F>
        where F: Fn(Index, Self::Voxel) -> U,
    {
        MapIndex {
            extent: self,
            f,
        }
    }
}

impl<V, T> Space for &T
    where
    V: Copy,
    T: Space<Voxel=V>,
{
    type Voxel = V;
    fn get(&self, offset: Index) -> Self::Voxel {
        (*self).get(offset)
    }
}
/*
impl<T, V, N> Into<[V; N]> for View
    where T: Space<Item=V>
{
    fn into(self) -> [V; N] {
        self.
    }
}*/

pub struct Map<E, F> {
    space: E,
    f: F,
}

impl<T: Copy, E, F> Space for Map<E, F>
    where
    E: Space,
    F: Fn(E::Voxel) -> T,
{
    type Voxel = T;

    fn get(&self, offset: Index) -> Self::Voxel {
        (self.f)(self.space.get(offset))
    }
}


pub struct MapIndex<E, F> {
    extent: E,
    f: F,
}

impl<T: Copy, E: Space, F> Space for MapIndex<E, F>
    where F: Fn(Index, E::Voxel) -> T,
{
    type Voxel = T;

    fn get(&self, offset: Index) -> Self::Voxel {
        (self.f)(offset, self.extent.get(offset))
    }
}


pub trait MutChunk {
    type Voxel: Copy;
    fn set(&mut self, offset: Index, value: Self::Voxel);
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn neg_index() {
        assert_eq!(
            ChunkIndex::new_encompassing([-1, -1, -1].into()),
            ChunkIndex([-16, -16, -16].into()),
        );
    }
}
