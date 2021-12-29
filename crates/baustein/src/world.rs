/*! Voxel storage */
use feldspar_core::glam::IVec3;

use feldspar_map::{palette::PaletteId8, sdf::Sd8};
use feldspar_map::chunk::{ Chunk, ChunkShape, SdfChunk, PaletteIdChunk };
use feldspar_map::units::{ ChunkUnits, VoxelUnits };
use ndshape::{ConstPow2Shape3i32, ConstShape};
use std::collections::HashMap;
use crate::traits::{Map, Index, ChunkIndex};

use crate::traits::Extent;

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

impl Extent for SdfChunk {
    type Voxel = Sd8;

    fn get(&self, offset: Index) -> Self::Voxel {
        self[ChunkShape::linearize(offset.0.to_array()) as usize]
    }
}

impl Extent for PaletteIdChunk {
    type Voxel = PaletteId8;

    fn get(&self, offset: Index) -> Self::Voxel {
        self[ChunkShape::linearize(offset.0.to_array()) as usize]
    }
}


/// A really terrible, simple world type
/// What do I want from the world?
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
    chunks: &'a World,
    overlaid: HashMap<ChunkIndex, PaletteIdChunk>,
}

impl<'a> Cow<'a> {
    fn new(chunks: &'a World) -> Self {
        Cow {
            chunks,
            overlaid: Default::default(),
        }
    }

    fn get(&self, offset: Index) -> PaletteId8 {
        let ci = World::to_chunk_index(offset);
        self.get_chunk(ci).get(VoxelUnits(offset.0 - ci.0))
    }

    fn get_chunk(&self, offset: ChunkIndex) -> PaletteIdChunk {
        let ci = World::truncate_chunk_index(offset);
        *self.overlaid.get(&ci)
            .unwrap_or(&self.chunks.get_chunk(ci))
    }

    fn set_chunk(&mut self, offset: ChunkIndex, chunk: PaletteIdChunk) {
        let ci = World::truncate_chunk_index(offset);
        self.overlaid.insert(ci, chunk);
    }
}
