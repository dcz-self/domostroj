/*! Voxel storage */
use feldspar_map::units::VoxelUnits;
use ndshape::ConstShape;
use std::collections::HashMap;
use crate::indices::{to_i32_arr, to_u32_arr, ChunkIndex, Index};
use crate::prefab::{ PaletteIdChunk, PaletteVoxel, World };
use crate::traits::{Space, IterableSpace};

// Used traits
use crate::traits::MutChunk;

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

    fn get(&self, offset: Index) -> PaletteVoxel {
        let ci = ChunkIndex::new_encompassing(offset);
        self.get_chunk(ci).get(Index::new(ci.get_internal_offset(offset)))
    }

    // Not sure if this is the right place to do this, but let's try.
    pub fn set(&mut self, offset: Index, value: PaletteVoxel) {
        let ci = ChunkIndex::new_encompassing(offset);
        let i = Index::new(ci.get_internal_offset(offset));
        let chunk = self.get_chunk_mut(ci);
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
    world: &'a S,
    offset: Index,
    shape: PhantomData<Shape>,
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

impl<'a, S: IterableSpace, Shape> IterableSpace for View<'a, S, Shape> {
    fn visit_indices<F: FnMut(Index)>(&self, mut f: F) {
        self.world.visit_indices(|i| f(i - VoxelUnits(self.offset.into())))
    }
}

/// Flat 3d array, out-of-bounds gives default voxel.
/// This should be pretty fast, but not suitable for any large space.
struct FlatPaddedCuboid<V, Shape: ConstShape<3>> {
    data: Vec<V>,
    offset: Index,
    shape: PhantomData<Shape>,
}

/* Const expressions not in stable yet
struct FlatPaddedCuboid<V, const X: usize, const Y: usize, const Z: usize> {
    data: [V; X*Y*Z],
}
*/

struct OutOfBounds;

impl<V: Default, Shape: ConstShape<3, Coord=u32>> FlatPaddedCuboid<V, Shape> {
    /// Dimensions are determined by the compile-time Shape.
    /// Offset is the lowest point of this cuboid portion.
    fn new_from_space<S: Space<Voxel=V>>(space: &S, offset: Index) -> Self {
        let mut data = Vec::with_capacity(Shape::SIZE as usize);
        for i in 0..Shape::SIZE {
            let idx = <Shape as ConstShape<3>>::delinearize(i);
            let idx: Index = to_i32_arr(idx).into();
            data[i as usize] = space.get(idx + VoxelUnits(offset.0.into()));
        }
        Self {
            data,
            offset,
            shape: Default::default(),
        }
    }

    /// Returns the index that's actuallly the corner, e.g. not 1 unit beyond
    fn opposite_corner(&self) -> Index {
        self.offset + VoxelUnits(to_i32_arr(Shape::ARRAY).into()) - VoxelUnits([1, 1, 1].into())
    }

    fn contains(&self, index: Index) -> bool {
        if index.x() < self.offset.x() || index.y() < self.offset.y() || index.z() < self.offset.z() {
            return false;
        }
        let opposite = self.opposite_corner();
        if index.x() > opposite.x() || index.y() > opposite.y() || index.z() > opposite.z() {
            return false;
        }
        true
    }

    fn set(&mut self, index: Index, value: V) -> Result<(), OutOfBounds> {
        if self.contains(index) {
            self.data[Shape::linearize(to_u32_arr(index.into())) as usize] = value;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }
}

impl<V: Default + Copy, Shape: ConstShape<3, Coord=u32>> Space for FlatPaddedCuboid<V, Shape> {
    type Voxel = V;
    fn get(&self, offset: Index) -> Self::Voxel {
        if self.contains(offset) {
            self.data[Shape::linearize(to_u32_arr(offset.into())) as usize]
        } else {
            Default::default()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn cow_apply_neg() {
        let world = World::default();
        let mut cow = Cow::new(&world);
        cow.set([-1, -1, -1].into(), PaletteVoxel(1));
        let changes = cow.into_changes();
        let mut world = world;
        changes.apply(&mut world);
        assert_eq!(world.get([-1, -1, -1].into()), PaletteVoxel(1));
    }

    #[test]
    fn view_neg() {
        let world = World::default();
        let mut cow = Cow::new(&world);
        cow.set([-1, -1, -1].into(), PaletteVoxel(1));
        let changes = cow.into_changes();
        let mut world = world;
        changes.apply(&mut world);
        let view = View::<_, ndshape::ConstShape3u32<2, 2, 2>>::new(&world, [-2, -2, -2].into());
        assert_eq!(view.get([1, 1, 1].into()), PaletteVoxel(1));
    }
    
}
