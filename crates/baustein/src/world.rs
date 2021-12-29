/*! Voxel storage */
use feldspar_map::{palette::PaletteId8, sdf::Sd8};
use feldspar_map::chunk::{ Chunk, ChunkShape, SdfChunk, PaletteIdChunk };
use feldspar_map::units::VoxelUnits;
use ndshape::{ConstPow2Shape3i32, ConstShape};
use crate::traits::{Map, Index};

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
