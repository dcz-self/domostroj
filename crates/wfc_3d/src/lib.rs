use baustein::indices::{ usize_to_i32_arr, Index, VoxelUnits };
use baustein::re::ConstShape;
use baustein::traits::Space;
use baustein::world::FlatPaddedGridCuboid;
use std::cmp;
use std::collections::HashMap;
use std::hash::{ Hash, Hasher };
use std::marker::PhantomData;


use baustein::traits::{Extent, IterableSpace};


fn popcount<T: Hash + Eq>(i: impl Iterator<Item=T>) -> HashMap<T, usize> {
    i.fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}


/// An index starting from 0
#[derive(Copy, Clone)]
struct StampIndex([usize; 3]);

/// A 0-indexed fragment of a space, with static dimensions.
/// Compared by its contents.
/// Hashing and comparison allocate :/
/// This is stored in 8 + 3*4(index) = 20 bytes.
/// 20 bytes is still 2×2×5 voxels. Still less than 3×3×3.
/// So don't use this for small stamps.
/// If index is limited to 0..256 in each dimension,
/// then 8 + 3*1 = 11 bytes (+1 padding?).
/// Less than 2×2×3 voxels.
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

impl<'a, Shape, S> ViewStamp<'a, Shape, S>
    where
    Shape: ConstShape,
    S: Space<Voxel=Superposition> + 'a,
{
    fn contains<U>(&self, stamp: ViewStamp<Shape, U>) -> bool
        where U: Space<Voxel=VoxelId>
    {
        for i in 0..Shape::SIZE {
            let index = StampIndex(Shape::delinearize(i));
            if !self.get(index).contains(stamp.get(index)) {
                return false;
            }
        }
        return true;
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

impl<'a, V, Shape, S> cmp::Eq for ViewStamp<'a, Shape, S>
    where
    V: Copy + Eq,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{}

/// This should be enough for all relevant voxel types: 256.
/// If more is actually used, this should represent categories,
/// and another pass of generation used for specializing them.
type VoxelId = u8;

/// Not instantiable. A mapping between u64 and an actual type of voxel.
trait Palette<V> {
    fn get(id: VoxelId) -> V;
    fn default() -> VoxelId;
}

/// A voxel ID that can be resolved to the Voxel
/// without storing extra memory.
/// Probably premature, this blows up type signatures.
#[derive(Copy, Clone, Debug)]
struct PaletteVoxel<V, P: Palette<V>> {
    id: VoxelId,
    palette: PhantomData<(P, V)>,
}

/// The impls follow here to get rid of requiring them on PhantomData.
/// Try removing those in favor of #[derive(...)] to see.
impl<V, P: Palette<V>> PartialEq for PaletteVoxel<V, P> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<V, P: Palette<V>> Eq for PaletteVoxel<V, P> {}

impl<V, P: Palette<V>> Hash for PaletteVoxel<V, P> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.id.hash(hasher)
    }
}

impl<V: Copy, P: Palette<V>> PaletteVoxel<V, P> {
    fn get(&self) -> V {
        P::get(self.id)
    }
}

impl<V: Default, P: Palette<V>> Default for PaletteVoxel<V, P> {
    fn default() -> Self {
        PaletteVoxel {
            id: P::default(),
            palette: Default::default(),
        }
    }
}

type FP<V, P, S> = FlatPaddedGridCuboid<PaletteVoxel<V, P>, S>;

/// Wrapping modes: 
struct Wrapping;

/// Returns all stamps fitting within the cuboid,
/// given wrapping modes.
/// Stamps are of a static size. Not sure if that's the best idea.
fn gather_stamps<V, Shape, StampShape, P>(
    cuboid: &FP<V, P, Shape>,
    wrapping: Wrapping,
) -> HashMap<ViewStamp<StampShape, FP<V, P, Shape>>, usize>
where
    V: Copy + Default + Hash,
    Shape: ConstShape,
    StampShape: ConstShape,
    P: Palette<V> + Copy,
{
    let end_stamp_corner: Index =
        cuboid.get_beyond_opposite_corner()
        - VoxelUnits(usize_to_i32_arr(<StampShape as ConstShape>::ARRAY));
    let mut counts: HashMap<ViewStamp<StampShape, FP<V, P, Shape>>, usize>
        = HashMap::new();
    cuboid.visit_indices(|idx| {
        if idx.x() < end_stamp_corner.x()
            && idx.y() < end_stamp_corner.y()
            && idx.z() < end_stamp_corner.z()
        {
            let stamp = ViewStamp::<StampShape, FP<V, P, Shape>>::new(cuboid, idx);
            *counts.entry(stamp).or_insert(0) += 1;
        }
    });
    counts
}

// A bitmap is used because the set of items inside
// is close to the entire space of items.
// Forget Bloom filters.
// u128 likely slow on 64-bit systems,
// so skip that until an actual need emerges.
/// Can only distinguish 64 items.
/// Distinguishes integers strictly.
// Storage is a bit mask
// where a set bit marks a missing value.
#[derive(Clone, Copy)]
struct Superposition(u64);

impl Superposition {
    fn new_full() -> Self {
        Self(0)
    }
    fn contains(&self, v: VoxelId) -> bool {
        (self.0 & (1 << (v as u64))) == 0
    }
    fn is_empty(&self) -> bool {
        self.0 == u64::MAX
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use baustein::re::ConstPow2Shape;

    #[derive(Copy, Clone)]
    struct DumbPalette;

    impl Palette<u16> for DumbPalette {
        fn get(id: VoxelId) -> u16 { id as u16 }
        fn default() -> VoxelId { 0 }
    }
    #[test]
    fn foo() {
        let voxel = PaletteVoxel::<u16, DumbPalette>::default();
        let _v: u16 = voxel.get();
    }

    #[test]
    fn stamps() {
        type Voxel = PaletteVoxel::<u16, DumbPalette>;
        type Shape = ConstPow2Shape<3, 3, 3>;
        type StampShape = ConstPow2Shape<1, 1, 1>;
        let world = FlatPaddedGridCuboid::<Voxel, Shape>::new([0, 0, 0].into());
        let stamps = gather_stamps::<_, _, StampShape, _>(&world, Wrapping);
        assert_eq!(stamps.len(), 1);
        assert_eq!(stamps.into_values().collect::<Vec<_>>(), vec![6*6*6]);
    }
}
