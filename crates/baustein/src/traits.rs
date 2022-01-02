/*! Traits for easy access to voxels */

use crate::indices::Index;


/// Access to elements of a 3d cuboid region of voxels.
/// Should be used for arbitrary (small) cuboids, may be used for the world.
/// It cannot be modified in place, instead replace the entire chunk.
pub trait Space {
    type Voxel: Copy;
    
    fn get(&self, offset: Index) -> Self::Voxel;
    /// Mapping methods may skip some empty areas.
    fn map<'a, U: Copy, F: Fn(Self::Voxel) -> U>(&'a self, f: F) -> Map<&'a Self, F> {
        Map {
            space: self,
            f,
        }
    }

    fn map_index<'a, U: Copy, F>(&'a self, f: F) -> MapIndex<&'a Self, F>
        where F: Fn(Index, Self::Voxel) -> U,
    {
        MapIndex {
            extent: self,
            f,
        }
    }

    fn zip<'a, 'b, T: Copy, S>(&'a self, other: &'b S) -> Zip<&'a Self, &'b S>
        where S: Space<Voxel=T>
    {
        Zip {
            left: self,
            right: other,
        }
    }
}

impl<V, T> Space for &T
    where
    V: Copy,
    T: Space<Voxel=V>,
{
    type Voxel = V;
    fn get(&self, offset: Index) -> Self::Voxel {
        (*self).get(offset)
    }
}
/*
impl<T, V, N> Into<[V; N]> for View
    where T: Space<Item=V>
{
    fn into(self) -> [V; N] {
        self.
    }
}*/

pub struct Map<E, F> {
    space: E,
    f: F,
}

impl<T: Copy, E, F> Space for Map<E, F>
    where
    E: Space,
    F: Fn(E::Voxel) -> T,
{
    type Voxel = T;

    fn get(&self, offset: Index) -> Self::Voxel {
        (self.f)(self.space.get(offset))
    }
}


pub struct MapIndex<E, F> {
    extent: E,
    f: F,
}

impl<T: Copy, E: Space, F> Space for MapIndex<E, F>
    where F: Fn(Index, E::Voxel) -> T,
{
    type Voxel = T;

    fn get(&self, offset: Index) -> Self::Voxel {
        (self.f)(offset, self.extent.get(offset))
    }
}

impl<E, F> IterableSpace for MapIndex<E, F>
    where E: IterableSpace,
{
    fn visit_indices<G: FnMut(Index)>(&self, f: G) {
        self.extent.visit_indices(f)
    }
}


pub struct Zip<E, F> {
    left: E,
    right: F,
}

impl<T: Copy, U: Copy, E, F> Space for Zip<E, F>
    where
    E: Space<Voxel=T>,
    F: Space<Voxel=U>,
{
    type Voxel = (T, U);

    fn get(&self, offset: Index) -> Self::Voxel {
        (self.left.get(offset), self.right.get(offset))
    }
}

// FIXME: this totally ignores the second space. Should be okay for now.
impl<E, F> IterableSpace for Zip<E, F>
    where E: IterableSpace,
{
    fn visit_indices<G: FnMut(Index)>(&self, f: G) {
        self.left.visit_indices(f)
    }
}

pub trait MutChunk {
    type Voxel: Copy;
    fn set(&mut self, offset: Index, value: Self::Voxel);
}

// TODO: fold into Space.
// The folding will need some extra logic to align with underlyning chunks
pub trait IterableSpace {
    // Can't be arsed to code an iterator.
    // Waiting for generators, and maybe monads.
    fn visit_indices<F: FnMut(Index)>(&self, f: F);
}

/*
impl<T> IterableSpace for &T
    where
    T: IterableSpace,
{
    fn visit_indices<F: FnMut(Index)>(&self, f: F) {
        (*self).visit_indices(f)
    }
}*/

/* TODO: this is overkill. Yes, Zip is broken without it.
/// For structures which can cheaply tell the corners
/// between which all their voxels lie
pub trait Extent {
    /// The corner with lowest indices in each dimension
    fn get_offset(&self) -> Index;
    /// The corner with highest indices in each dimension, plus [1,1,1]
    fn get_beyond_opposite_corner(&self) -> Index;
}

impl IterableSpace for Extent {
    fn visit_indices<F: FnMut(Index)>(&self, f: F) {
        let start = self.get_offset();
        let end = self.get_beyond_opposite_corner();
        for 
    }
}
*/

// TODO: a Chunk trait should include the shape.
// a World trait should include the grid

