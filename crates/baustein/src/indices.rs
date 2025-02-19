/*! Indexing facilities */
#[cfg(feature="prefab_bevy")]
use bevy::prelude::Vec3;
use feldspar_core::glam::IVec3;
use feldspar_map;
use ndshape;
use std::ops;

// Traits
use ndshape::ConstShape;

pub fn to_i32_arr(a: [u32; 3]) -> [i32; 3] {
    [a[0] as i32, a[1] as i32, a[2] as i32]
}

pub fn to_i64_arr(a: [i32; 3]) -> [i64; 3] {
    [a[0] as i64, a[1] as i64, a[2] as i64]
}

pub fn usize_to_i32_arr(a: [usize; 3]) -> [i32; 3] {
    [a[0] as i32, a[1] as i32, a[2] as i32]
}

pub(crate) fn usize_to_u32_arr(a: [usize; 3]) -> [u32; 3] {
    [a[0] as u32, a[1] as u32, a[2] as u32]
}

pub(crate) fn to_u32_arr(a: [i32; 3]) -> [u32; 3] {
    [a[0] as u32, a[1] as u32, a[2] as u32]
}

pub fn to_usize_arr(a: [i32; 3]) -> [usize; 3] {
    [a[0] as usize, a[1] as usize, a[2] as usize]
}

pub fn i64_to_i32_arr(a: [i64; 3]) -> [i32; 3] {
    [a[0] as i32, a[1] as i32, a[2] as i32]
}

/// Ideally for relative indexing.
pub struct VoxelUnits(pub [i32; 3]);

impl Into<IVec3> for VoxelUnits {
    fn into(self) -> IVec3 {
        self.0.into()
    }
}

impl From<[i32; 3]> for VoxelUnits {
    fn from(v: [i32; 3]) -> Self {
        Self(v)
    }
}

/// Delberately not public inside,
/// to be able to replace it in the future with a chunk+voxel combo
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorldIndex(pub IVec3);

impl WorldIndex {
    pub fn new(offset: VoxelUnits) -> Self {
        Self(offset.0.into())
    }

    /// Returns the 6 neighbouring indices sharing a face.
    pub fn neighbours6(&self) -> Neighbours6<Self> {
        Neighbours6([
            *self + VoxelUnits([1, 0, 0].into()),
            *self + VoxelUnits([-1, 0, 0].into()),
            *self + VoxelUnits([0, 1, 0].into()),
            *self + VoxelUnits([0, -1, 0].into()),
            *self + VoxelUnits([0, 0, 1].into()),
            *self + VoxelUnits([0, 0, -1].into()),
        ])
    }

    pub fn iter_neighbours6(&self) -> impl Iterator<Item=Self> {
        self.neighbours6().0.into_iter()
    }

    pub fn x(&self) -> i32 {
        self.0[0]
    }

    pub fn y(&self) -> i32 {
        self.0[1]
    }

    pub fn z(&self) -> i32 {
        self.0[2]
    }
}

impl ops::Index<usize> for WorldIndex {
    type Output = i32;
    fn index(&self, offset: usize) -> &i32 {
        &self.0[offset]
    }
}

impl From<[i32; 3]> for WorldIndex {
    fn from(coords: [i32; 3]) -> Self {
        Self(coords.into())
    }
}

impl From<[i64; 3]> for WorldIndex {
    fn from(coords: [i64; 3]) -> Self {
        Self(i64_to_i32_arr(coords).into())
    }
}

impl From<IVec3> for WorldIndex {
    fn from(coords: IVec3) -> Self {
        Self(coords.into())
    }
}

#[cfg(feature="prefab_bevy")]
impl From<Vec3> for WorldIndex {
    fn from(coords: Vec3) -> Self {
        [coords.x.round() as i32, coords.y.round() as i32, coords.z.round() as i32].into()
    }
}

impl Into<[i32; 3]> for WorldIndex {
    fn into(self) -> [i32; 3] {
        self.0.into()
    }
}

impl Into<[i64; 3]> for WorldIndex {
    fn into(self) -> [i64; 3] {
        to_i64_arr(self.0.into())
    }
}

impl Into<IVec3> for WorldIndex {
    fn into(self) -> IVec3 {
        self.0.into()
    }
}

#[cfg(feature="prefab_bevy")]
impl Into<Vec3> for WorldIndex {
    fn into(self) -> Vec3 {
        Vec3::new(self.0.x as f32, self.0.y as f32, self.0.z as f32)
    }
}

impl ops::Sub<VoxelUnits> for WorldIndex {
    type Output = WorldIndex;
    fn sub(self, s: VoxelUnits) -> Self::Output {
        WorldIndex(self.0 - IVec3::from(s.0))
    }
}

impl ops::Sub<WorldIndex> for WorldIndex {
    type Output = VoxelUnits;
    fn sub(self, s: WorldIndex) -> Self::Output {
        VoxelUnits((self.0 - IVec3::from(s.0)).into())
    }
}

impl ops::Add<VoxelUnits> for WorldIndex {
    type Output = WorldIndex;
    fn add(self, s: VoxelUnits) -> Self::Output {
        WorldIndex(self.0 + IVec3::from(s.0))
    }
}

impl ops::Sub<feldspar_map::units::VoxelUnits<IVec3>> for WorldIndex {
    type Output = WorldIndex;
    fn sub(self, s: feldspar_map::units::VoxelUnits<IVec3>) -> Self::Output {
        WorldIndex(self.0 - IVec3::from(s.0))
    }
}

impl ops::Add<feldspar_map::units::VoxelUnits<IVec3>> for WorldIndex {
    type Output = WorldIndex;
    fn add(self, s: feldspar_map::units::VoxelUnits<IVec3>) -> Self::Output {
        WorldIndex(self.0 + IVec3::from(s.0))
    }
}

pub type Index = WorldIndex;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct ChunkIndex(IVec3);

pub type ChunkShape = ndshape::ConstShape3u32<16, 16, 16>;

/// This is really slow, we already know chunk coords are pow2.
/// Thankfully the threshold is known, so this will likely optimize out.
fn trunc(v: i32, thr: u32) -> i32 {
    let r = v.rem_euclid(thr as i32);
    v - r
}

impl ChunkIndex {
    fn center() -> Self {
        Self(IVec3::new(0, 0, 0))
    }

    /// Returns the index of the chunk, to which the world index belongs.
    pub fn new_encompassing(index: WorldIndex) -> Self {
        Self(IVec3::new(
            trunc(index.0[0], ChunkShape::ARRAY[0]),
            trunc(index.0[1], ChunkShape::ARRAY[1]),
            trunc(index.0[2], ChunkShape::ARRAY[2]),
        ))
    }

    /// Returns the offset (minimum in all dimensions),
    /// at which this chunk begins.
    pub fn get_world_offset(&self) -> WorldIndex {
        self.0.into()
    }

    /// Offset relative to the beginning of the chunk.
    pub fn get_internal_offset(&self, index: WorldIndex) -> VoxelUnits {
        VoxelUnits((index.0 - Self::new_encompassing(index).0).into())
    }
}


/// Ordered collection of properties of neighbours.
/// x+ x- y+ y- z+ z-
#[derive(Default, Clone, Copy, Debug)]
pub struct Neighbours6<T: Copy>(pub [T; 6]);

impl<T: Copy> Neighbours6<T> {
    pub fn xp(&self) -> T {
        self.0[0]
    }
    pub fn xm(&self) -> T {
        self.0[1]
    }
    pub fn yp(&self) -> T {
        self.0[2]
    }
    pub fn ym(&self) -> T {
        self.0[3]
    }
    pub fn zp(&self) -> T {
        self.0[4]
    }
    pub fn zm(&self) -> T {
        self.0[5]
    }
}

/// Another name for the same thing.
/// Maybe better than the original.
#[derive(Default, Clone, Copy)]
pub struct NamedNeighbours6<T> {
    pub xp: T,
    pub xm: T,
    pub yp: T,
    pub ym: T,
    pub zp: T,
    pub zm: T,
}

impl<T: Copy> From<NamedNeighbours6<T>> for Neighbours6<T> {
    fn from(nn: NamedNeighbours6<T>) -> Self {
        Self([nn.xp, nn.xm, nn.yp, nn.ym, nn.zp, nn.zm])
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn neg_index() {
        assert_eq!(
            ChunkIndex::new_encompassing([-1, -1, -1].into()),
            ChunkIndex([-16, -16, -16].into()),
        );
    }
}
