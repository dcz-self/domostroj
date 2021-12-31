/*! Prebult components that lose a lot of flexibility, but are useful for prototyping.
 * At some point you will want to build your own based on those. */
mod feldspar;

use feldspar_map::{palette::PaletteId8, sdf::Sd8};
use feldspar_map::chunk::PaletteIdChunk;
use std::collections::HashMap;
use crate::traits::{ ChunkIndex, Space, WorldIndex };
use crate::traits::Index;
use crate::world::Cow;


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
        *self.chunks.get(&offset).clone().unwrap_or(&[0; 4096])
    }
    
    pub fn get_chunk_ref(&self, offset: ChunkIndex) -> &PaletteIdChunk {
        self.chunks.get(&offset).clone().unwrap_or(&[0; 4096])
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
    type Voxel = PaletteId8;
    fn get(&self, offset: WorldIndex) -> Self::Voxel {
        let ci = ChunkIndex::new_encompassing(offset);
        match self.chunks.get(&ci) {
            Some(chunk) => chunk.get(Index::new(ci.get_internal_offset(offset))),
            None => Default::default(),
        }
    }
}
