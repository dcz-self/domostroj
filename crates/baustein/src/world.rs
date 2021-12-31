/*! Voxel storage */
use feldspar_core::glam::IVec3;

use feldspar_map::chunk::{ Chunk, ChunkShape, SdfChunk, PaletteIdChunk };
use feldspar_map::palette::PaletteId8;
use feldspar_map::units::{ ChunkUnits, VoxelUnits };
use ndshape::ConstShape;
use std::collections::HashMap;
use crate::indices::to_i32_arr;
use crate::prefab::World;
use crate::traits::{Space, WorldIndex, MutChunk, Index, ChunkIndex};


/// A naive copy-on-write overlay over a World.
/// Its changes can be eventually applied to the underlying World.
pub struct Cow<'a> {
    base: &'a World,
    overlaid: HashMap<ChunkIndex, PaletteIdChunk>,
}

impl<'a> Cow<'a> {
    pub fn new(base: &'a World) -> Self {
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
    pub fn set(&mut self, offset: Index, value: PaletteId8) {
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

    /// Extracts changes ready for application on a mutable world
    pub fn into_changes(self) -> Overlay {
        Overlay(self.overlaid)
    }
}

pub struct Overlay(HashMap<ChunkIndex, PaletteIdChunk>);

impl Overlay {
    /// Applies changes to world. Caution: does not care if it applies to the correct world.
    pub fn apply(mut self, output: &mut World) {
        for (offset, chunk) in self.0.drain() {
            output.chunks.insert(offset, chunk);
        }
    }
}

use std::marker::PhantomData;

pub struct View<'a, S, Shape> {
    pub world: &'a S,
    pub offset: Index,
    pub shape: PhantomData<Shape>,
}

/// This is redundant, but derive(Clone) complains that the concrete type under S is not Clone.
/// Which is nonsense: S is bbehind a reference, and cannot be deep-cloned.
/// Where would the copy be stored? It'd be dropped instantly.
impl<'a, Shape, S, V> Clone for View<'a, S, Shape>
    where
    S: Space<Voxel=V>,
    Shape: ConstShape<3, Coord=u32> + ndshape::Shape<3, Coord=u32>,
{
    fn clone(&self) -> Self {
        Self::new(self.world.clone(), self.offset.clone())
    }
}

impl<'a, Shape, S, V> View<'a, S, Shape>
    where
        S: Space<Voxel=V>,
        Shape: ConstShape<3, Coord=u32> + ndshape::Shape<3, Coord=u32>,
{
    pub fn new(space: &'a S, offset: Index) -> Self {
        Self {
            world: space,
            offset,
            shape: Default::default(),
        }
    }
    pub fn into_vec(self) -> Vec<V> {
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
        self.world.get(offset + VoxelUnits(self.offset.into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cow_apply_neg() {
        let world = World::default();
        let mut cow = Cow::new(&world);
        cow.set([-1, -1, -1].into(), 1);
        let changes = cow.into_changes();
        let mut world = world;
        changes.apply(&mut world);
        assert_eq!(world.get([-1, -1, -1].into()), 1);
    }

    #[test]
    fn view_neg() {
        let world = World::default();
        let mut cow = Cow::new(&world);
        cow.set([-1, -1, -1].into(), 1);
        let changes = cow.into_changes();
        let mut world = world;
        changes.apply(&mut world);
        let view = View::<_, ndshape::ConstShape3u32<2, 2, 2>>::new(&world, [-2, -2, -2].into());
        assert_eq!(view.get([1, 1, 1].into()), 1);
    }
    
}
