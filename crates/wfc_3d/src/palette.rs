/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
/*! Some helpers for conventing to and back from `VoxelId`s
 */

use super::VoxelId;
use std::hash::{ Hash, Hasher };
use std::marker::PhantomData;

/// Not instantiable. A mapping between u64 and an actual type of voxel.
/// V::Default() must correspond to VoxelId(0);
pub trait Palette<V> {
    fn get(id: VoxelId) -> V;
    fn to_ref(v: V) -> VoxelId;
    fn default_id() -> VoxelId {
        0
    }
}

/// Superposition with a palette statically attached.
/// This is to ease the storage of each component.
/// It can stay as a bitmap and get resolved on demand.
/// The DIMENSIONS constant should ideally
/// be dependent on the palette,
/// but I haven't found a way to make it work.
pub struct Superposition<V, P: Palette<V>, const DIMENSIONS: u8> {
    voxel: crate::Superposition<DIMENSIONS>,
    palette: PhantomData<P>,
    v: PhantomData<V>,
}

impl<V, P: Palette<V>, const D: u8> Superposition<V, P, D> {
    pub fn iter_allowed<'a>(&'a self) -> impl Iterator<Item=V> + 'a {
        (0..D)
            .filter(|id| self.voxel.allows(*id))
            .map(P::get)
    }
}

impl<V, P: Palette<V>, const D: u8> From<crate::Superposition<D>>
    for Superposition<V, P, D>
{
    fn from(voxel: crate::Superposition<D>) -> Self
    {
        Self {
            voxel,
            palette: Default::default(),
            v: Default::default(),
        }
    }
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
            id: P::default_id(),
            palette: Default::default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[derive(Copy, Clone)]
    struct DumbPalette;

    impl Palette<u16> for DumbPalette {
        fn get(id: VoxelId) -> u16 { id as u16 }
        fn to_ref(v: u16) -> VoxelId { v as u8 }
    }
    #[test]
    fn foo() {
        let voxel = PaletteVoxel::<u16, DumbPalette>::default();
        let _v: u16 = voxel.get();
    }
}
