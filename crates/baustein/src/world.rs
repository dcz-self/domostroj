/*! Voxel storage */
use feldspar_core::glam::IVec3;

use feldspar_map::{palette::PaletteId8, sdf::Sd8};
use feldspar_map::chunk::{ Chunk, ChunkShape, SdfChunk, PaletteIdChunk };
use feldspar_map::units::{ ChunkUnits, VoxelUnits };
use ndshape::ConstShape;
use std::collections::HashMap;
use crate::traits::{Extent, MutChunk, Index, ChunkIndex};


type Voxel = (Sd8, PaletteId8);

impl Extent for Chunk {
    type Voxel = Voxel;

    fn get(&self, offset: Index) -> Self::Voxel {
        (
            self.sdf.get(offset),
            self.palette_ids.get(offset),
        )
    }
}

impl MutChunk for Chunk {
    type Voxel = Voxel;
    
    fn set(&mut self, offset: Index, value: Self::Voxel) {
        self.sdf.set(offset, value.0);
        self.palette_ids.set(offset, value.1);
    }
}

impl Extent for SdfChunk {
    type Voxel = Sd8;

    fn get(&self, offset: Index) -> Self::Voxel {
        self[ChunkShape::linearize(offset.0.to_array()) as usize]
    }
}

impl MutChunk for SdfChunk {
    type Voxel = Sd8;
    
    fn set(&mut self, offset: Index, value: Self::Voxel) {
        self[ChunkShape::linearize(offset.0.to_array()) as usize] = value;
    }
}

impl Extent for PaletteIdChunk {
    type Voxel = PaletteId8;

    fn get(&self, offset: Index) -> Self::Voxel {
        self[ChunkShape::linearize(offset.0.to_array()) as usize]
    }
}


impl MutChunk for PaletteIdChunk {
    type Voxel = PaletteId8;
    
    fn set(&mut self, offset: Index, value: Self::Voxel) {
        self[ChunkShape::linearize(offset.0.to_array()) as usize] = value;
    }
}

/// A really terrible, simple world type
/// What do I want from the world?
/// Definitely not direct mutability. Use the Cow.
#[derive(Default)]
struct World {
    chunks: HashMap<ChunkIndex, PaletteIdChunk>,
}

/// This is really slow, we already know chunk coords are pow2.
fn trunc(v: i32, thr: i32) -> i32 {
    let r = v % thr;
    v - r
}

impl World {
    fn to_chunk_index(index: Index) -> ChunkIndex {
        ChunkUnits(IVec3::new(
            trunc(index.0[0], ChunkShape::ARRAY[0]),
            trunc(index.0[1], ChunkShape::ARRAY[1]),
            trunc(index.0[2], ChunkShape::ARRAY[2]),
        ))
    }

    fn truncate_chunk_index(index: ChunkIndex) -> ChunkIndex {
        ChunkUnits(IVec3::new(
            trunc(index.0[0], ChunkShape::ARRAY[0]),
            trunc(index.0[1], ChunkShape::ARRAY[1]),
            trunc(index.0[2], ChunkShape::ARRAY[2]),
        ))
    }
    
    fn get(&self, offset: Index) -> PaletteId8 {
        let ci = Self::to_chunk_index(offset);
        match self.chunks.get(&ci) {
            Some(chunk) => chunk.get(VoxelUnits(offset.0 - ci.0)),
            None => Default::default(),
        }
    }

    fn get_chunk(&self, offset: ChunkIndex) -> PaletteIdChunk {
        let ci = Self::truncate_chunk_index(offset);
        *self.chunks.get(&ci).clone().unwrap_or(&[0; 4096])
    }
/*
    fn iter_chunks(&self) -> impl Iterator<Item=(ChunkIndex, PaletteIdChunk)> {
        chunks.
    }
*/
    fn cow<'a>(&'a self) -> Cow<'a> {
        Cow::new(&self)
    }
}

struct Cow<'a> {
    base: &'a World,
    overlaid: HashMap<ChunkIndex, PaletteIdChunk>,
}

impl<'a> Cow<'a> {
    fn new(base: &'a World) -> Self {
        Cow {
            base,
            overlaid: Default::default(),
        }
    }

    fn get(&self, offset: Index) -> PaletteId8 {
        let ci = World::to_chunk_index(offset);
        self.get_chunk(ci).get(VoxelUnits(offset.0 - ci.0))
    }

    // Not sure if this is the right place to do this, but let's try.
    fn set(&mut self, offset: Index, value: PaletteId8) {
        let ci = World::to_chunk_index(offset);
        let i = VoxelUnits(offset.0 - ci.0);
        let mut chunk = self.get_chunk_mut(ci);
        chunk.set(i, value);
    }

    fn get_chunk(&self, offset: ChunkIndex) -> PaletteIdChunk {
        let ci = World::truncate_chunk_index(offset);
        *self.overlaid.get(&ci)
            .unwrap_or(&self.base.get_chunk(ci))
    }

    fn get_chunk_mut(&mut self, offset: ChunkIndex) -> &mut PaletteIdChunk {
        let ci = World::truncate_chunk_index(offset);
        self.overlaid.entry(ci).or_insert_with(|| self.base.get_chunk(ci))
    }

    fn set_chunk(&mut self, offset: ChunkIndex, chunk: PaletteIdChunk) {
        let ci = World::truncate_chunk_index(offset);
        self.overlaid.insert(ci, chunk);
    }

    /* Nice idea, but we need to implement a struct that will hold the overlaid while it's being drained.
    fn iter_overlay(self) -> impl Iterator<Item=(ChunkIndex, PaletteIdChunk)> {
        self.overlaid.drain()
    }*/

    /// Applies changes to world. Caution: does not care if it applies to the correct world.
    fn apply(mut self, output: &mut World) {
        for (offset, chunk) in self.overlaid.drain() {
            output.chunks.insert(offset, chunk);
        }
    }
}
