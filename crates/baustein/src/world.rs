/*! Voxel storage */
use feldspar_core::glam::IVec3;

use feldspar_map::{palette::PaletteId8, sdf::Sd8};
use feldspar_map::chunk::{ Chunk, ChunkShape, SdfChunk, PaletteIdChunk };
use feldspar_map::units::{ ChunkUnits, VoxelUnits };
use ndshape::ConstShape;
use std::collections::HashMap;
use crate::traits::{Space, WorldIndex, MutChunk, Index, ChunkIndex};


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

/// A really terrible, simple world type
/// What do I want from the world?
/// Definitely not direct mutability. Use the Cow.
#[derive(Default)]
pub struct World {
    chunks: HashMap<ChunkIndex, PaletteIdChunk>,
}

/// This is really slow, we already know chunk coords are pow2.
fn trunc(v: i32, thr: i32) -> i32 {
    let r = v % thr;
    v - r
}

impl World {
    fn get_chunk(&self, offset: ChunkIndex) -> PaletteIdChunk {
        *self.chunks.get(&offset).clone().unwrap_or(&[0; 4096])
    }
    
    fn get_chunk_ref(&self, offset: ChunkIndex) -> &PaletteIdChunk {
        self.chunks.get(&offset).clone().unwrap_or(&[0; 4096])
    }

    fn iter_chunks(&self) -> impl Iterator<Item=(ChunkIndex, &PaletteIdChunk)> {
        self.chunks.iter().map(|(offset, chunk)| (offset.clone(), chunk))
    }

    fn iter_chunk_indices<'a>(&'a self) -> impl Iterator<Item=ChunkIndex> + 'a {
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
        let ci = ChunkIndex::new_encompassing(offset);
        self.get_chunk(ci).get(Index::new(ci.get_internal_offset(offset)))
    }

    // Not sure if this is the right place to do this, but let's try.
    fn set(&mut self, offset: Index, value: PaletteId8) {
        let ci = ChunkIndex::new_encompassing(offset);
        let i = Index::new(ci.get_internal_offset(offset));
        let mut chunk = self.get_chunk_mut(ci);
        chunk.set(i, value);
    }

    fn get_chunk(&self, offset: ChunkIndex) -> PaletteIdChunk {
        *self.overlaid.get(&offset)
            .unwrap_or(&self.base.get_chunk(offset))
    }

    fn get_chunk_ref(&self, offset: ChunkIndex) -> &PaletteIdChunk {
        self.overlaid.get(&offset)
            .unwrap_or(self.base.get_chunk_ref(offset))
    }

    fn get_chunk_mut(&mut self, offset: ChunkIndex) -> &mut PaletteIdChunk {
        self.overlaid.entry(offset)
            .or_insert_with(|| self.base.get_chunk(offset))
    }

    fn set_chunk(&mut self, offset: ChunkIndex, chunk: PaletteIdChunk) {
        self.overlaid.insert(offset, chunk);
    }

    fn iter_chunks(&self) -> impl Iterator<Item=(ChunkIndex, &PaletteIdChunk)> {
        self.iter_chunk_indices()
            .map(|offset| (offset, self.get_chunk_ref(offset)))
    }

    fn iter_chunk_indices<'b>(&'b self) -> impl Iterator<Item=ChunkIndex> + 'b {
        self.base.chunks.keys()
            .filter(|offset| !self.overlaid.contains_key(offset))
            .chain(self.overlaid.keys())
            .cloned()
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

use std::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct View<'a, S, Shape> {
    pub world: &'a S,
    pub offset: Index,
    pub shape: PhantomData<Shape>,
}

fn to_i32_arr(a: [u32; 3]) -> [i32; 3] {
    [a[0] as i32, a[1] as i32, a[2] as i32]
}

impl<'a, Shape, S> View<'a, S, Shape>
    where
        S: Space<Voxel=PaletteId8>,
        Shape: ConstShape<3, Coord=u32> + ndshape::Shape<3, Coord=u32>,
{
    pub fn new(space: &'a S, offset: Index) -> Self {
        Self {
            world: space,
            offset,
            shape: Default::default(),
        }
    }
    pub fn into_vec(self) -> Vec<PaletteId8> {
        (0..Shape::SIZE)
            .map(|i| <Shape as ConstShape<3>>::delinearize(i))
            .map(|index| self.get(to_i32_arr(index).into()))
            .collect()
    }

    pub fn opposite_corner(&self) -> Index {
        self.offset + VoxelUnits(to_i32_arr(Shape::ARRAY).into())
    }
}

impl<'a, S: Space, Shape> Space for View<'a, S, Shape> {
    type Voxel = S::Voxel;
    fn get(&self, offset: Index) -> Self::Voxel {
        self.world.get(offset - VoxelUnits(self.offset.into()))
    }
}

