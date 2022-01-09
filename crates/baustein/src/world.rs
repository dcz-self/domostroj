/*! Voxel storage */
use feldspar_map::units::VoxelUnits;
use ndshape::{ ConstShape, RuntimeShape };
use std::collections::HashMap;
use crate::indices::{to_i32_arr, to_u32_arr, to_usize_arr, usize_to_i32_arr, ChunkIndex, Index};
use crate::prefab::{ PaletteIdChunk, PaletteVoxel, World };
use crate::re;
use crate::traits::{Extent, Space, IterableSpace, Map, MapIndex, Zip};

// Used traits
use ndshape::Shape;
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
    Shape: re::ConstShape // + ndshape::Shape<3, Coord=u32>,
{
    fn clone(&self) -> Self {
        Self::new(self.world.clone(), self.offset.clone())
    }
}

impl<'a, Shape, S, V> View<'a, S, Shape>
    where
        S: Space<Voxel=V>,
        Shape: ConstShape<3, Coord=usize> + ndshape::Shape<3, Coord=usize>,
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
            .map(|index| self.get(usize_to_i32_arr(index).into()))
            .collect()
    }

    pub fn opposite_corner(&self) -> Index {
        self.offset + VoxelUnits(usize_to_i32_arr(Shape::ARRAY).into())
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
#[derive(Clone)]
pub struct FlatPaddedGridCuboid<V, Shape: ConstShape<3>> {
    pub(crate) data: Vec<V>,
    pub(crate) offset: Index,
    shape: PhantomData<Shape>,
}

/* Const expressions not in stable yet
struct FlatPaddedGridCuboid<V, const X: usize, const Y: usize, const Z: usize> {
    data: [V; X*Y*Z],
}
*/

#[derive(Debug)]
pub struct OutOfBounds;

impl<V: Default + Copy, Shape: ConstShape<3, Coord=usize>> FlatPaddedGridCuboid<V, Shape> {
    /// Creates a new one filled with emptiness
    pub fn new(offset: Index) -> Self {
        let mut data = Vec::with_capacity(Shape::SIZE as usize);
        data.resize(Shape::SIZE, V::default());
        Self {
            data,
            offset,
            shape: Default::default(),
        }
    }
    /// Dimensions are determined by the compile-time Shape.
    /// Offset is the lowest point of this cuboid portion.
    pub fn new_from_space<S: Space<Voxel=V>>(space: &S, offset: Index) -> Self {
        let mut data = Vec::with_capacity(Shape::SIZE as usize);
        for i in 0..Shape::SIZE {
            let idx = <Shape as ConstShape<3>>::delinearize(i);
            let idx: Index = usize_to_i32_arr(idx).into();
            data.push(space.get(idx + VoxelUnits(offset.0.into())));
        }
        Self {
            data,
            offset,
            shape: Default::default(),
        }
    }

    /// Returns the index that's actuallly the corner, e.g. not 1 unit beyond
    fn opposite_corner(&self) -> Index {
        self.offset + VoxelUnits(usize_to_i32_arr(Shape::ARRAY).into()) - VoxelUnits([1, 1, 1].into())
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

    pub fn set(&mut self, index: Index, value: V) -> Result<(), OutOfBounds> {
        if self.contains(index) {
            let offset = index - VoxelUnits(self.offset.0);
            self.data[Shape::linearize(to_usize_arr(offset.into()))] = value;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }

    /// Caution, samples are aligned according to the Shape.
    pub fn get_samples(&self) -> &[V] {
        self.data.as_ref()
    }
}

impl<V, Shape> Space for FlatPaddedGridCuboid<V, Shape>
    where
    V: Default + Copy,
    Shape: ConstShape<3, Coord=usize>,
{
    type Voxel = V;
    fn get(&self, index: Index) -> Self::Voxel {
        if self.contains(index) {
            let offset = index - VoxelUnits(self.offset.0);
            self.data[Shape::linearize(to_usize_arr(offset.into()))]
        } else {
            Default::default()
        }
    }
}

impl<V, Shape> IterableSpace for FlatPaddedGridCuboid<V, Shape>
    where
    V: Copy,
    Shape: ConstShape<3, Coord=usize>,
{
    fn visit_indices<F: FnMut(Index)>(&self, mut f: F) {
        for i in 0..Shape::SIZE {
            let idx = <Shape as ConstShape<3>>::delinearize(i);
            let idx: Index = usize_to_i32_arr(idx).into();
            f(idx + VoxelUnits(self.offset.0.into()))
        }
    }
}

impl<V, Shape> Extent for FlatPaddedGridCuboid<V, Shape>
    where Shape: re::ConstShape {
    fn get_offset(&self) -> Index {
        self.offset
    }
    fn get_dimensions(&self) -> [usize; 3] {
        <Shape as re::ConstShape>::ARRAY
    }
}

impl<V, S, Shape> From<S> for FlatPaddedGridCuboid<V, Shape>
    where
    Shape: re::ConstShape,
    V: Default + Copy,
    S: Space<Voxel=V> + IterableSpace + Extent + IntoCuboid,
{
    fn from(space: S) -> Self {
        let offset = space.get_offset();
        Self::new_from_space(&space, offset)
    }
}


/// Flat 3d array, out-of-bounds gives default voxel.
/// This should be pretty fast, but not suitable for any large space.
pub struct FlatPaddedCuboid<V> {
    data: Vec<V>,
    offset: Index,
    dimensions: [usize; 3],
}

impl<V: Default> FlatPaddedCuboid<V> {
    /// Offset is the lowest point of this cuboid portion.
    pub fn new_from_space<S>(space: &S, offset: Index, dimensions: [usize; 3]) -> Self
        where S: Space<Voxel=V>
    {
        let shape = RuntimeShape::<usize, 3>::new(dimensions);
        let mut data = Vec::with_capacity(shape.size());
        for i in 0..(shape.size()) {
            let idx = shape.delinearize(i);
            let idx: Index = usize_to_i32_arr(idx).into();
            data.push(space.get(idx + VoxelUnits(offset.0.into())));
        }
        Self {
            data,
            offset,
            dimensions,
        }
    }

    fn get_shape(&self) -> RuntimeShape<usize, 3> {
        RuntimeShape::<usize, 3>::new(self.dimensions)
    }

    /// Returns the index that's actuallly the corner, e.g. not 1 unit beyond
    fn opposite_corner(&self) -> Index {
        self.offset
            + VoxelUnits(usize_to_i32_arr(self.dimensions).into())
            - VoxelUnits([1, 1, 1].into())
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
            let i = self.get_shape().linearize(to_usize_arr(index.into()));
            self.data[i] = value;
            Ok(())
        } else {
            Err(OutOfBounds)
        }
    }
}

impl<V> Space for FlatPaddedCuboid<V>
    where V: Default + Copy,
{
    type Voxel = V;
    fn get(&self, offset: Index) -> Self::Voxel {
        if self.contains(offset) {
            self.data[self.get_shape().linearize(to_usize_arr(offset.into()))]
        } else {
            Default::default()
        }
    }
}

impl<V> IterableSpace for FlatPaddedCuboid<V>
    where V: Default + Copy
{
    fn visit_indices<F: FnMut(Index)>(&self, mut f: F) {
        let shape = self.get_shape();
        for i in 0..(shape.size()) {
            let idx = shape.delinearize(i);
            let idx: Index = usize_to_i32_arr(idx).into();
            f(idx + VoxelUnits(self.offset.0.into()))
        }
    }
}

impl<V> Extent for FlatPaddedCuboid<V> {
    fn get_offset(&self) -> Index {
        self.offset
    }
    fn get_dimensions(&self) -> [usize; 3] {
        self.dimensions
    }
}

impl<V, S> From<S> for FlatPaddedCuboid<V>
    where
    V: Default,
    S: Space<Voxel=V> + IterableSpace + Extent + IntoCuboid,
{
    fn from(space: S) -> Self {
        let offset = space.get_offset();
        let dimensions = space.get_dimensions();
        Self::new_from_space(&space, offset, dimensions)
    }
}

/// A marker to denote that `Into<Cuboid>` can be derived for this type.
/// This is needed because `Into<Cuboid>` needs traits that are already defined on `Cuboid`,
/// so conflicts with `impl<T> From<T> for T;` from core.
/// Requiring an extra trait defined for all but Cuboid resolves that problem.
pub trait IntoCuboid {}

impl<E, F> IntoCuboid for MapIndex<E, F>{}

impl<E, F> IntoCuboid for Map<E, F>{}

impl<E, F> IntoCuboid for Zip<E, F>{}

#[cfg(test)]
mod test {
    use super::*;

    use crate::re::ConstPow2Shape;

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
        let view = View::<_, ndshape::ConstShape3usize<2, 2, 2>>::new(&world, [-2, -2, -2].into());
        assert_eq!(view.get([1, 1, 1].into()), PaletteVoxel(1));
    }

    #[test]
    fn copy_offset() {
        type Cuboid<V> = FlatPaddedGridCuboid<V, ConstPow2Shape<5, 5, 5>>;
        let extent = Cuboid::<()>::new([0, -10, 0].into());
        let world = extent.map_index(|i, _| i.y() < 0);
        let other: Cuboid<bool> = world.into();
    }

    #[test]
    fn set_offset() {
        type Cuboid<V> = FlatPaddedGridCuboid<V, ConstPow2Shape<5, 5, 5>>;
        let mut extent = Cuboid::<bool>::new([0, -10, 0].into());
        assert!(extent.set([0, -9, 0].into(), true).is_ok());
    }
}
