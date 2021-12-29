/*! Traits for easy access to voxels */
use feldspar_core::glam::IVec3;

use feldspar_map::chunk::CHUNK_SIZE;
use feldspar_map::units::{ ChunkUnits, VoxelUnits };

pub type Index = VoxelUnits<IVec3>;
pub type ChunkIndex = ChunkUnits<IVec3>;

/// Actually just a chunk.
/// TODO:
/// Access to elements of a 3d dense cuboid region of voxels.
/// Should be used for chunks, may be used for the world.
/// It cannot be modified in place, instead replace the entire chunk.
pub trait Extent {
    type Voxel: Copy;
    
    fn get(&self, offset: Index) -> Self::Voxel;
    /// Mapping methods may skip some empty areas.
    fn map<'a, U: Copy, F: Fn(Self::Voxel) -> U>(&'a self, f: F) -> Map<&'a Self, F> {
        Map {
            extent: self,
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

pub struct Map<E, F> {
    extent: E,
    f: F,
}

impl<T: Copy, E: Extent, F> Extent for Map<E, F>
    where F: Fn(E::Voxel) -> T,
{
    type Voxel = T;

    fn get(&self, offset: Index) -> Self::Voxel {
        (self.f)(self.extent.get(offset))
    }
}


pub struct MapIndex<E, F> {
    extent: E,
    f: F,
}

impl<T: Copy, E: Extent, F> Extent for MapIndex<E, F>
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

/*
/// Iterate over chunks.
trait World<C: Extent> {
    fn get(&self, offset: VoxelUnits) -> &C;
    fn set(&mut self, offset: VoxelUnits, C);
    fn chunks(&self) -> ChunkIter<C>;
}*/
