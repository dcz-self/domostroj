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

impl<'a, Shape, S, const C: u8> ViewStamp<'a, Shape, S>
    where
    Shape: ConstShape,
    S: Space<Voxel=Superposition<C>> + 'a,
{
    fn is_allowed<U>(&self, stamp: ViewStamp<Shape, U>) -> bool
        where U: Space<Voxel=VoxelId>
    {
        for i in 0..Shape::SIZE {
            let index = StampIndex(Shape::delinearize(i));
            if !self.get(index).is_allowed(stamp.get(index)) {
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
/// Tracks which items have been excluded.
/// Can only distinguish up to 64 items.
/// Distinguishes integers strictly.
/// The dimension count is needed to be able to distinguish the case
/// where only one option remains.
// Storage is a bit mask
// where a set bit marks a missing value.
#[derive(Clone, Copy)]
struct Superposition<const DIMENSIONS: u8>(u64);

impl<const D: u8> Superposition<D> {
    /// Nothing excluded
    const FREE: Self = Self(0);
    /// Everything excluded; use as a mask.
    fn impossible() -> Self {
        Self(((D as u64) << 2) - 1)
    }
    fn is_allowed(&self, v: VoxelId) -> bool {
        (self.0 & (1 << (v as u64))) == 0
    }
    fn count_allowed(&self) -> u8 {
        D - self.0.count_ones() as u8
    }
}

/// Calculates Shannon entropy of weighted options multiplied by total weight.
///
/// Shannon entropy (in natural units):
///
/// P_i: probability of choosing an item
///
/// H = -Σ{i} P_i · log(P_i)
///
/// In this case, the probability is defined by the occurrence count w_i
/// and total number of occurrences across all items W:
///
/// P_i = w_i / W
///
/// Transforming to achieve more operations that can be done on integers efficiently:
///
/// H = -Σ{i} P_i · log(P_i)
/// = -Σ{i} w_i / W · log(w_i / W)
/// = -Σ{i} w_i / W · (log(w_i) - log(W))
/// = Σ{i} w_i / W · (log(W) - log(w_i))
/// = 1/W · Σ{i} w_i · (log(W) - log(w_i))
///
/// In our case, this is executing on the CPU, so integer efficiency matters.
/// log(x) is approximated with the use of usize::leading_zeros().
/// Meanwhile, division is more lossy.
///
/// The result is fast in integers, at the cost of not being so accurate at low entropies
/// (which is quite useless actually), so it does the final division in float (I give up).
///
/// Problems:
/// - the number of occurrences can't be more than usize::MAX / 2, or log() will flop.
/// - total = 0, or any item results in a panic.
fn get_entropy(weights: impl Iterator<Item=usize>, total: usize) -> f32 {
    let log_total = log2(total);
    weights
        .map(|w| w * (log_total - log2(w)))
        .sum::<usize>() as f32
        / (total as f32)
}

fn log2(v: usize) -> usize {
    (usize::BITS - v.leading_zeros() - 1) as usize
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_float_eq::*;
    use baustein::re::ConstPow2Shape;
    use more_asserts::*;

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

    #[test]
    fn log() {
        for i in 0..usize::BITS {
            assert_eq!(log2(1 << i as usize), i as usize);
        }
    }

    #[test]
    #[should_panic]
    fn log0() {
        log2(0); // -1, not representable, no one cares here.
    }

    #[test]
    fn entropy() {
        let items = [30, 1];
        let total = items.iter().sum();
        assert_ne!(get_entropy(items.iter().map(|v| *v), total), 0.0);
        // How extreme can we get?
        let items = [30000, 1];
        let total = items.iter().sum();
        assert_ne!(get_entropy(items.iter().map(|v| *v), total), 0.0);
    }

    #[test]
    fn entropy_1() {
        // One bit
        let items = [1, 1];
        let total = items.iter().sum();
        assert_float_absolute_eq!(get_entropy(items.iter().map(|v| *v), total), 1.0);
    }

    #[test]
    fn entropy_extreme() {
        // When does it stop being meaningful?
        let items = [30000, 1];
        let total = items.iter().sum();
        let items2 = [30, 1];
        let total2 = items2.iter().sum();
        assert_gt!(
            get_entropy(items2.iter().map(|v| *v), total2),
            get_entropy(items.iter().map(|v| *v), total),
        );
    }
    
    #[test]
    fn entropy_precision() {
        // How precise can we go?
        // Compare same total, to eliminate the influence of the final `/ total`.
        let items = [30001, 1];
        let total = items.iter().sum();
        let items2 = [30000, 2];
        let total2 = items2.iter().sum();
        assert_gt!(
            get_entropy(items2.iter().map(|v| *v), total2),
            get_entropy(items.iter().map(|v| *v), total),
        );
    }

    #[test]
    fn entropy_precision2() {
        // How precise can we go?
        // Compare counts so similar that they make little difference.
        let items = [30001, 1001];
        let total = items.iter().sum();
        let items2 = [30000, 1002];
        let total2 = items2.iter().sum();
        assert_gt!(
            get_entropy(items2.iter().map(|v| *v), total2),
            get_entropy(items.iter().map(|v| *v), total),
        );
        // HOW DOES THIS EVEN PASS? This method is so naive!
        // Scoop:
        // [crates/wfc_3d/src/lib.rs:264] weights.map(|w| w * (log_total - log2(w))).sum::<usize>() = 5010
        // [crates/wfc_3d/src/lib.rs:264] weights.map(|w| w * (log_total - log2(w))).sum::<usize>() = 5005
    }

}
