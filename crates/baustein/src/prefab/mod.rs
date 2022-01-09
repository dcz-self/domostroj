/*! Prebult components that lose a lot of flexibility, but are useful for prototyping.
 * At some point you will want to build your own based on those. */
mod feldspar;
mod serialize;

use block_mesh::{ MergeVoxel, Voxel };
use ndshape::ConstPow2Shape3u32;
use ndshape::ConstPow2Shape3usize;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fmt;

use crate::indices::{ to_i32_arr, to_u32_arr, usize_to_i32_arr, ChunkIndex, Index, WorldIndex };
use crate::traits::{ IterableSpace, MutChunk, Space };
use crate::world::Cow;

// traits
use ndshape::ConstShape;


pub type ChunkShape = ConstPow2Shape3u32<4, 4, 4>;


/// The voxel that maps to a palette entry.
#[derive(Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PaletteVoxel(pub u8);

impl PaletteVoxel {
    pub const EMPTY: PaletteVoxel = PaletteVoxel(0);
}

impl fmt::Debug for PaletteVoxel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl Voxel for PaletteVoxel {
    fn is_empty(&self) -> bool {
        self.0 == 0
    }
    fn is_opaque(&self) -> bool {
        self.0 != 0
    }
}

impl MergeVoxel for PaletteVoxel {
    type MergeValue = u8;
    fn merge_value(&self) -> Self::MergeValue {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct PaletteIdChunk([PaletteVoxel; 4096]);

impl PaletteIdChunk {
    const EMPTY: PaletteIdChunk = PaletteIdChunk([PaletteVoxel::EMPTY; 4096]);
}

impl Default for PaletteIdChunk {
    fn default() -> Self {
        Self([PaletteVoxel::EMPTY; 4096])
    }
}

impl Space for PaletteIdChunk {
    type Voxel = PaletteVoxel;

    fn get(&self, offset: Index) -> Self::Voxel {
        self.0[ChunkShape::linearize(to_u32_arr(offset.into())) as usize]
    }
}

impl MutChunk for PaletteIdChunk {
    type Voxel = PaletteVoxel;
    
    fn set(&mut self, offset: Index, value: Self::Voxel) {
        self.0[ChunkShape::linearize(to_u32_arr(offset.into())) as usize] = value;
    }
}

impl IterableSpace for PaletteIdChunk {
    fn visit_indices<F: FnMut(Index)>(&self, mut f: F) {
        (0..ChunkShape::SIZE)
            .map(|i| <ChunkShape as ConstShape<3>>::delinearize(i))
            .map(|index| to_i32_arr(index).into())
            .for_each(f);
    }
}

// TODO: implement a macro for this

/*
impl<T> IterableSpace for T where T: ConstShape<3, Coord=u32> {
    fn visit_indices<F: FnMut(Index)>(&self, f: F) {
        (0..T::SIZE)
            .map(|i| T::delinearize(i))
            .map(|index| to_i32_arr(index).into())
            .for_each(f);
    }
}*/

/// A really terrible, simple world type
/// What do I want from the world?
/// Definitely not direct mutability. Use the Cow.
#[derive(Default)]
pub struct World {
    // This is public because of Cow being coupled with World.
    // New interfaces to decouple might be needed.
    pub(crate) chunks: HashMap<ChunkIndex, PaletteIdChunk>,
}

/// This is really slow, we already know chunk coords are pow2.
fn trunc(v: i32, thr: i32) -> i32 {
    let r = v % thr;
    v - r
}

impl World {
    pub fn get_chunk(&self, offset: ChunkIndex) -> PaletteIdChunk {
        *self.chunks.get(&offset).clone().unwrap_or(&PaletteIdChunk::EMPTY)
    }
    
    pub fn get_chunk_ref(&self, offset: ChunkIndex) -> &PaletteIdChunk {
        self.chunks.get(&offset).clone().unwrap_or(&PaletteIdChunk::EMPTY)
    }

    pub fn iter_chunks(&self) -> impl Iterator<Item=(ChunkIndex, &PaletteIdChunk)> {
        self.chunks.iter().map(|(offset, chunk)| (offset.clone(), chunk))
    }

    pub fn iter_chunk_indices<'a>(&'a self) -> impl Iterator<Item=ChunkIndex> + 'a {
        self.chunks.keys().cloned()
    }

    fn cow<'a>(&'a self) -> Cow<'a> {
        Cow::new(&self)
    }
}

impl Space for World {
    type Voxel = PaletteVoxel;
    fn get(&self, offset: WorldIndex) -> Self::Voxel {
        let ci = ChunkIndex::new_encompassing(offset);
        match self.chunks.get(&ci) {
            Some(chunk) => chunk.get(Index::new(ci.get_internal_offset(offset))),
            None => Default::default(),
        }
    }
}

impl<const X: usize, const Y: usize, const Z: usize> IterableSpace for ConstPow2Shape3usize<X, Y, Z> {
    fn visit_indices<F: FnMut(Index)>(&self, f: F) {
        (0..Self::SIZE)
            .map(|i| Self::delinearize(i))
            .map(|index| usize_to_i32_arr(index).into())
            .for_each(f);
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use maplit::hashset;
    use std::collections::HashSet;

    #[test]
    fn test_iterable_chunk() {
        type Shape = ConstPow2Shape3usize<1, 1, 1>;
        let shape = Shape {};
        let mut indices = HashSet::new();
        shape.visit_indices(|i| { indices.insert(i);     });
        assert_eq!(
            indices,
            hashset!(
                [0, 0, 0].into(),
                [0, 0, 1].into(),
                [0, 1, 0].into(),
                [0, 1, 1].into(),
                [1, 0, 0].into(),
                [1, 0, 1].into(),
                [1, 1, 0].into(),
                [1, 1, 1].into(),
            ),
        );
    }
}
