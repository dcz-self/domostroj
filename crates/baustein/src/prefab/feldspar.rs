/*! Prefabricated pieces that shouldn't be used unless you're sticking with feldspar's needs */

use feldspar_map::{palette::PaletteId8, sdf::Sd8};
use feldspar_map::chunk::{ Chunk, ChunkShape, SdfChunk, PaletteIdChunk };
use ndshape::ConstShape;

use crate::traits::{ Index, IterableSpace, MutChunk, Space };


type Voxel = (Sd8, PaletteId8);


impl Space for Chunk {
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

impl Space for SdfChunk {
    type Voxel = Sd8;

    fn get(&self, offset: Index) -> Self::Voxel {
        self[ChunkShape::linearize(offset.into()) as usize]
    }
}

impl MutChunk for SdfChunk {
    type Voxel = Sd8;
    
    fn set(&mut self, offset: Index, value: Self::Voxel) {
        self[ChunkShape::linearize(offset.into()) as usize] = value;
    }
}

impl Space for PaletteIdChunk {
    type Voxel = PaletteId8;

    fn get(&self, offset: Index) -> Self::Voxel {
        self[ChunkShape::linearize(offset.into()) as usize]
    }
}

impl MutChunk for PaletteIdChunk {
    type Voxel = PaletteId8;
    
    fn set(&mut self, offset: Index, value: Self::Voxel) {
        self[ChunkShape::linearize(offset.into()) as usize] = value;
    }
}

impl IterableSpace for PaletteIdChunk {
    fn visit_indices<F: FnMut(Index)>(&self, mut f: F) {
        (0..ChunkShape::SIZE)
            .map(|i| <ChunkShape as ConstShape<3>>::delinearize(i))
            .map(|index| index.into())
            .map(f);
    }
}
