/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */

mod extent;
mod wave;

use baustein::indices::{ usize_to_i32_arr, Index, VoxelUnits };
use baustein::re::ConstShape;
use baustein::traits::Space;
use baustein::world::FlatPaddedGridCuboid;
use float_ord::FloatOrd;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::hash::{ Hash, Hasher };
use std::marker::PhantomData;


use extent::Stamped;


fn popcount<T: Hash + Eq>(i: impl Iterator<Item=T>) -> HashMap<T, usize> {
    i.fold(HashMap::new(), |mut map, c| {
        *map.entry(c).or_insert(0) += 1;
        map
    })
}

type StampSpace<Shape> = FlatPaddedGridCuboid<VoxelId, Shape>;

/// The collection of Stamps from a single source.
///
/// Currently stamps are just a &[ViewStamp] where each refers to a Space.
/// Because a Space will usually have large dimensions,
/// the each vertical slice of a ViewStamp is in a different cache line.
/// If stamps are few, it might be faster to put their voxels in a single cache line
/// by keeping each in a contiguous buffer.
/// If stamps are small and they are iterated consecutively (they usually are),
/// try keeping them all in a single large buffer.
/// This needs to get benchmarked:
/// `perf stat` or https://stackoverflow.com/questions/49242919/profiling-cache-evicition
/// https://stackoverflow.com/questions/18172353/how-to-catch-the-l3-cache-hits-and-misses-by-perf-tool-in-linux
struct StampCollection<'a, StampShape: ConstShape, SourceShape: ConstShape>(
    Vec<(ST<'a, StampShape, SourceShape>, usize)>,
);

impl<'a, StampShape: ConstShape, SourceShape: ConstShape> StampCollection<'a, StampShape, SourceShape> {
    fn new(stamps: Vec<(ST<'a, StampShape, SourceShape>, usize)>) -> Self {
        Self(stamps)
    }

    fn get_total_occurrences(&self) -> usize {
        self.0.iter().map(|(_s, v)| *v).sum()
    }

    fn get_collapse_outcomes<S, const C: u8>(&'a self, view: &ViewStamp<StampShape, S>)
        -> CollapseOutcomes<'a, StampShape, StampSpace<SourceShape>>
    where
        S: Space<Voxel=Superposition<C>>,
    {
        let matches = self.0.iter()
            .map(|(stamp, _occurrences)| stamp)
            .filter(|stamp| view.allows(stamp));
        let mut outcome = CollapseOutcomes::None;
        for stamp in matches {
            match outcome {
                CollapseOutcomes::None => {
                    outcome = CollapseOutcomes::One(stamp);
                },
                CollapseOutcomes::One(_) => {
                    outcome = CollapseOutcomes::Multiple;
                    break;
                },
                _ => { unreachable!(); },
            }
        }
        outcome
    }
}

enum CollapseOutcomes<'a, Shape: ConstShape, S: Space> {
    One(&'a ViewStamp<'a, Shape, S>),
    None,
    Multiple,
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
        self.visit_indices(|i| out.push(self.get(i)));
        out
    }

    fn visit_indices<F: FnMut(StampIndex)>(&self, mut f: F) {
        for i in 0..Shape::SIZE {
            f(StampIndex(Shape::delinearize(i)));
        }
    }
}

//ViewStamp<'a, StampShape, FlatPaddedGridCuboid<VoxelId, Shape>>;

impl<'a, Shape, S, const C: u8> ViewStamp<'a, Shape, S>
    where
    Shape: ConstShape,
    S: Space<Voxel=Superposition<C>> + 'a,
{
    fn allows<U>(&self, stamp: &ViewStamp<Shape, U>) -> bool
        where U: Space<Voxel=VoxelId>
    {
        for i in 0..Shape::SIZE {
            let index = StampIndex(Shape::delinearize(i));
            if !self.get(index).allows(stamp.get(index)) {
                return false;
            }
        }
        return true;
    }
}

impl<'a, Shape, S> Clone for ViewStamp<'a, Shape, S>
    where
    Shape: ConstShape,
    S: Space + 'a,
{
    fn clone(&self) -> Self {
        Self {
            space: self.space,
            offset: self.offset,
            shape: Default::default(),
        }
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

impl<'a, V, Shape, S> fmt::Debug for ViewStamp<'a, Shape, S>
where
    V: fmt::Debug + Copy,
    Shape: ConstShape,
    S: Space<Voxel=V> + 'a,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        (self.offset, self.get_samples()).fmt(f)
    }
}

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

type FP<S> = FlatPaddedGridCuboid<VoxelId, S>;

/// Wrapping modes: 
struct Wrapping;

/// Returns all stamps fitting within the cuboid,
/// given wrapping modes.
/// Stamps are of a static size. Not sure if that's the best idea.
fn gather_stamps<Shape, StampShape>(
    cuboid: &FP<Shape>,
    wrapping: Wrapping,
) -> HashMap<ViewStamp<StampShape, FP<Shape>>, usize>
where
    Shape: ConstShape,
    StampShape: ConstShape,
{
    let views = cuboid
        .get_stamps_extent::<StampShape>()
        .iter()
        .map(|idx| ViewStamp::<StampShape, FP<Shape>>::new(cuboid, idx));
    popcount(views)
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
// where a set bit marks a disallowed value.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct Superposition<const DIMENSIONS: u8>(u64);

impl<const D: u8> Superposition<D> {
    /// Nothing excluded
    const FREE: Self = Self(0);
    /// Everything excluded; use as a mask.
    fn impossible() -> Self {
        Self(((D as u64) << 2) - 1)
    }
    fn only(v: VoxelId) -> Self {
        Self(Self::impossible().0 & !(1 << (v as u64)))
    }
    fn allows(&self, v: VoxelId) -> bool {
        (self.0 & (1 << (v as u64))) == 0
    }
    fn count_allowed(&self) -> u8 {
        D - self.0.count_ones() as u8
    }
}

/// Calculates Shannon entropy-ish
/// of weighted options multiplied by total weight.
///
/// Probabilities don't have to add up to 1, so this is not strict entropy calculation.
/// See following comments.
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
fn get_pseudo_entropy(weights: impl Iterator<Item=usize>, total: usize) -> f32 {
    let log_total = log2(total);
    weights
        .map(|w| w * (log_total - log2(w)))
        .sum::<usize>() as f32
        / (total as f32)
}

fn log2(v: usize) -> usize {
    (usize::BITS - v.leading_zeros() - 1) as usize
}

/// Superposition type
type FPC<S, const C: u8> = FlatPaddedGridCuboid<Superposition<C>, S>;
/// Stamp type
type ST<'a, StampShape, Shape> = ViewStamp<'a, StampShape, StampSpace<Shape>>;
/// Superposition view template
type SV<'a, StampShape, Shape, const C: u8> = ViewStamp<'a, StampShape, FlatPaddedGridCuboid<Superposition<C>, Shape>>;



fn get_distribution<'a, 's, 't: 'a, SShape, TShape, StampShape, const C: u8> (
    superposition: &'a SV<'s, StampShape, SShape, C>,
    stamps: &'t [(ST<'t, StampShape, TShape>, usize)],
) -> impl Iterator<Item=(&'t ST<'t, StampShape, TShape>, usize)> + 'a
    where
    StampShape: ConstShape,
    SShape: ConstShape,
    TShape: ConstShape,
{
    stamps.iter()
        .filter(|(stamp, _occurrences)| superposition.allows(stamp))
        .map(|(stamp, occurrences)| (stamp, *occurrences))
}

#[derive(Debug, Clone, Copy)]
enum PseudoEntropy {
    /// No possible choices
    Impossible,
    /// Exactly one possible choice
    Collapsed,
    /// Multiple possible choices, meaningful value
    Open(f32)
}

/// This calculates the "entropy" of a certain stamp in the wave (`superposition`).
/// Shannon entropy is calculated based on possible outcomes,
/// so in case of a world with all stamps coming with the same probability,
/// all positions in the wave would always have the same entropy,
/// because they weigh allowed options equally.
/// Entropy comes out the same for 2 or 3 equal options:
///
/// E(1/3, 1/3, 1/3) == E(1/2, 1/2).
///
/// This is not necessarily realistic for an actual template in use,
/// but it indicates that a better heuristic than entropy can be achieved.
///
/// This heuristic implements a similar measure to entropy,
/// except the probabilities don't need to add up to 1.
/// Instead of normalizing probabilities, we naively erase them.
/// As a result, in the case of 3 equal stamps,
/// the wave position that can accommodate 2 of them
/// is lower entropy than the one which can accommodate all 3.
///
/// PE(1/3, 1/3, 1/3) > PE(1/3, 1/3).
fn get_superposition_pseudo_entropy<'s, 't, SShape, TShape, StampShape, const C: u8> (
    superposition: &SV<'s, StampShape, SShape, C>,
    stamps: &[(ST<'t, StampShape, TShape>, usize)],
    total: usize,
) -> PseudoEntropy
    where
    StampShape: ConstShape,
    SShape: ConstShape,
    TShape: ConstShape,
{
    let possibilities_count = get_distribution(superposition, stamps).count();
    if possibilities_count == 0 {
        PseudoEntropy::Impossible
    } else if possibilities_count == 1 {
        PseudoEntropy::Collapsed
    } else {
        PseudoEntropy::Open(get_pseudo_entropy(
            get_distribution(superposition, stamps)
                .map(|(_stamp, occurrences)| occurrences),
            total,
        ))
    }
}

/// Returns the index of the template that has the lowest entropy
/// in relation to possible stamp choices,
/// or None if all are either undefined or 0.
fn find_lowest_pseudo_entropy<'a, Shape, StampShape, const C: u8>(
    wave: &FPC<Shape, C>,
    stamps: &[(ST<'a, StampShape, Shape>, usize)],
    total: usize,
) -> Option<Index>
    where
    Shape: ConstShape,
    StampShape: ConstShape,
{
    wave
        .get_stamps_extent::<StampShape>()
        .iter()
        .map(|i| SV::<StampShape, Shape, C>::new(wave, i))
        .map(|template| (
            template.offset,
            get_superposition_pseudo_entropy(&template, stamps, total)
        ))
        .filter_map(|(index, entropy)| match entropy {
            PseudoEntropy::Open(value) => Some((index, value)),
            _ => None,
        })
        .min_by_key(|(_index, entropy)| FloatOrd(*entropy))
        .map(|(index, _entropy)| index)
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_float_eq::*;
    use baustein::re::ConstAnyShape;
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
        type Shape = ConstAnyShape<8, 8, 8>;
        type StampShape = ConstAnyShape<2, 2, 2>;
        let world = FlatPaddedGridCuboid::<VoxelId, Shape>::new([0, 0, 0].into());
        let stamps = gather_stamps::<_, StampShape>(&world, Wrapping);
        assert_eq!(stamps.len(), 1);
        assert_eq!(stamps.into_values().collect::<Vec<_>>(), vec![7*7*7]);
    }

    #[test]
    fn stamps2() {
        type Shape = ConstAnyShape<4, 4, 4>;
        type StampShape = ConstAnyShape<2, 2, 2>;
        
        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Split into 2 areas
        let world = extent.map_index(|i, _| {
            if i.y() < 2 { 1 }
            else { 0 }
        });
        let world: FlatPaddedGridCuboid<u8, Shape> = world.into();
        
        let stamps = gather_stamps::<_, StampShape>(&world, Wrapping);
        assert_eq!(dbg!(&stamps).len(), 3);
        // all 1
        assert_eq!(stamps.get(&ViewStamp::new(&world, [0, 0, 0].into())).map(|x| *x), Some(3*3));
        // bottom 1, top 0
        assert_eq!(stamps.get(&ViewStamp::new(&world, [0, 1, 0].into())).map(|x| *x), Some(3*3));
        // all 0
        assert_eq!(stamps.get(&ViewStamp::new(&world, [0, 2, 0].into())).map(|x| *x), Some(3*3));
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
        assert_ne!(get_pseudo_entropy(items.iter().map(|v| *v), total), 0.0);
        // How extreme can we get?
        let items = [30000, 1];
        let total = items.iter().sum();
        assert_ne!(get_pseudo_entropy(items.iter().map(|v| *v), total), 0.0);
    }

    #[test]
    fn entropy_0() {
        // Nothing.
        let items = [5];
        let total = items.iter().sum();
        assert_eq!(get_pseudo_entropy(items.iter().map(|v| *v), total), 0.0);
    }

    #[test]
    fn entropy_1() {
        // One bit
        let items = [1, 1];
        let total = items.iter().sum();
        assert_float_absolute_eq!(get_pseudo_entropy(items.iter().map(|v| *v), total), 1.0);
    }

    #[test]
    fn entropy_extreme() {
        // When does it stop being meaningful?
        let items = [30000, 1];
        let total = items.iter().sum();
        let items2 = [30, 1];
        let total2 = items2.iter().sum();
        assert_gt!(
            get_pseudo_entropy(items2.iter().map(|v| *v), total2),
            get_pseudo_entropy(items.iter().map(|v| *v), total),
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
            get_pseudo_entropy(items2.iter().map(|v| *v), total2),
            get_pseudo_entropy(items.iter().map(|v| *v), total),
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
            get_pseudo_entropy(items2.iter().map(|v| *v), total2),
            get_pseudo_entropy(items.iter().map(|v| *v), total),
        );
        // HOW DOES THIS EVEN PASS? This method is so naive!
        // Scoop:
        // [crates/wfc_3d/src/lib.rs:264] weights.map(|w| w * (log_total - log2(w))).sum::<usize>() = 5010
        // [crates/wfc_3d/src/lib.rs:264] weights.map(|w| w * (log_total - log2(w))).sum::<usize>() = 5005
    }

    #[test]
    fn pseudo_entropy_equal() {
        let items = [1, 1];
        // fudging to suss out the difference between 2 and 3 allowed states remaining
        let total = 3;
        let items2 = [1, 1, 1];
        let total2 = items2.iter().sum();
        assert_gt!(
            get_pseudo_entropy(items2.iter().map(|v| *v), total2),
            get_pseudo_entropy(items.iter().map(|v| *v), total),
        );
    }

    /// This test fails on real entropy due to equal distribution of all stamps.
    #[test]
    fn superposition_lowest_entropy() {
        type Shape = ConstAnyShape<4, 4, 4>;
        type StampShape = ConstAnyShape<2, 2, 2>;

        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Split into 2 areas
        let world = extent.map_index(|i, _| {
            if i.y() < 2 { 1 }
            else { 0 }
        });
        let world: FlatPaddedGridCuboid<u8, Shape> = world.into();
        let stamps: Vec<_>
            = gather_stamps::<_, StampShape>(&world, Wrapping)
            .into_iter()
            .collect();
    
        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Corner will be the only one constrained in any way
        let world = extent.map_index(|i, _| {
            if i == [0,0,0].into() { Superposition::only(1) }
            else { Superposition::FREE }
        });
        let world: FlatPaddedGridCuboid<Superposition<2>, Shape> = world.into();

        let s = |i: [i32; 3]| {
            ViewStamp::<StampShape, _>::new(&world, i.into())
        };
        let gd = |i: [i32; 3]| {
            get_distribution(&s(i), &stamps)
                .collect::<Vec<_>>()
        };

        dbg!(gd([0, 0, 0]));
        dbg!(gd([1, 0, 0]));
        dbg!(gd([1, 1, 1]));

        let total: usize = stamps.iter().map(|(_s, v)| *v).sum();

        dbg!(get_superposition_pseudo_entropy(&s([0, 0, 0]), &stamps, total));
        dbg!(get_superposition_pseudo_entropy(&s([1, 0, 0]), &stamps, total));
        dbg!(get_superposition_pseudo_entropy(&s([1, 1, 1]), &stamps, total));
        let lowest = find_lowest_pseudo_entropy(&world, &stamps, total);
        assert_eq!(lowest, Some([0, 0, 0].into()));
    }

    #[test]
    fn superposition_collapsed_entropy() {
        type Shape = ConstAnyShape<4, 4, 4>;
        type StampShape = ConstAnyShape<1, 2, 1>;

        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Split into 2 areas
        let world = extent.map_index(|i, _| {
            if i.y() < 2 { 1 }
            else { 0 }
        });
        let world: FlatPaddedGridCuboid<u8, Shape> = world.into();
        let stamps: Vec<_>
            = gather_stamps::<_, StampShape>(&world, Wrapping)
            .into_iter()
            .collect();
    
        let extent = FlatPaddedGridCuboid::<(), Shape>::new([0, 0, 0].into());
        // Corner is completely collapsed
        let world = extent.map_index(|i, _| {
            if i == [0,0,0].into() { Superposition::only(1) }
            else if i == [0,1,0].into() { Superposition::only(1) }
            else { Superposition::FREE }
        });
        let world: FlatPaddedGridCuboid<Superposition<2>, Shape> = world.into();

        let s = |i: [i32; 3]| {
            ViewStamp::<StampShape, _>::new(&world, i.into())
        };
        let gd = |i: [i32; 3]| {
            get_distribution(&s(i), &stamps)
                .collect::<Vec<_>>()
        };

        dbg!(gd([0, 0, 0]));
        dbg!(gd([0, 1, 0]));
        dbg!(gd([1, 1, 1]));

        let total: usize = stamps.iter().map(|(_s, v)| *v).sum();

        dbg!(get_superposition_pseudo_entropy(&s([0, 0, 0]), &stamps, total));
        dbg!(get_superposition_pseudo_entropy(&s([0, 1, 0]), &stamps, total));
        dbg!(get_superposition_pseudo_entropy(&s([1, 1, 1]), &stamps, total));
        let lowest = find_lowest_pseudo_entropy(&world, &stamps, total);
        assert_eq!(lowest, Some([0, 1   , 0].into()));
    }
}
