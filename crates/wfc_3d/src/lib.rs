use baustein::indices::{ usize_to_i32_arr, Index, VoxelUnits };
use baustein::re::ConstShape;
use baustein::traits::Space;
use std::cmp;
use std::collections::HashMap;
use std::hash::{ Hash, Hasher };
use std::marker::PhantomData;


fn popcount<T: Hash + Eq>(i: impl Iterator<Item=T>) -> HashMap<T, usize> {
    i.fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}


/// An index starting from 0
struct StampIndex([usize; 3]);

/// A 0-indexed fragment of a space, with static dimensions.
/// Compared by its contents.
/// Hashing and comparison allocate :/
struct ViewStamp<'a, Shape: ConstShape, S: Space + 'a> {
    space: &'a S,
    offset: Index,
    shape: PhantomData<Shape>,
}

impl<'a, V, Shape, S> ViewStamp<'a, Shape, S>
    where
    V: Copy,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn new(space: &'a S, offset: Index) -> Self {
        Self {
            space,
            offset,
            shape: Default::default(),
        }
    }
    
    fn get(&self, index: StampIndex) -> V {
        self.space.get(self.offset + VoxelUnits(usize_to_i32_arr(index.0).into()))
    }

    fn get_samples(&self) -> Vec<V> {
        let mut out = Vec::with_capacity(Shape::SIZE);
        for i in 0..Shape::SIZE {
            out.push(self.get(StampIndex(Shape::delinearize(i))));
        }
        out
    }
}

impl<'a, V, Shape, S> Hash for ViewStamp<'a, Shape, S>
    where
    V: Copy + Hash,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.get_samples().hash(hasher)
    }
}

impl<'a, V, Shape, S> cmp::PartialEq for ViewStamp<'a, Shape, S>
    where
    V: Copy + PartialEq,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn eq(&self, other: &Self) -> bool {
        self.get_samples() == other.get_samples()
    }
}
