/*! Traits for easy access to voxels */

use feldspar_map::chunk::CHUNK_SIZE;
use feldspar_map::units::VoxelUnits;

use feldspar_core::glam::IVec3;

pub type Index = VoxelUnits<IVec3>;

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
    //fn map_index<'a, U: Copy>(&'a self, f: impl F(VoxelUnits, Self::Voxel) -> U) -> MappedIndexedExtent<'a, U>;
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

    /*
    fn map_index<'a, U: Copy>(&'a self, f: impl F(VoxelUnits, Self::Voxel) -> U) -> MappedIndexedExtent<'a, U> {
        panic!();
    }*/
}

/*
/// Iterate over chunks.
trait World<C> {
    fn get(&self, offset: VoxelUnits) -> &C;
    fn set(&mut self, offset: VoxelUnits, C);
    fn chunks(&self) -> ChunkIter<C>;
}*/
