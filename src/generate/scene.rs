/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
use baustein::re::ConstAnyShape;
use baustein::world::FlatPaddedGridCuboid;
use wfc_3d as wfc;


use baustein::traits::Space;
use wfc_3d::palette::Palette as _;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Voxel {
    Empty,
    Grass,
    Concrete,
    Wood,
    Glass,
}

impl Default for Voxel {
    fn default() -> Self {
        Voxel::Empty
    }
}

/// 5 dimensions - 5 distinct voxel possibilities.
type Superposition = wfc::palette::Superposition<Voxel, Palette, 5>;

/// A wrapper over a mundane chunk, for the purpose of becoming the Bevy resource.
#[derive(Clone)]
pub struct World(FlatPaddedGridCuboid<Superposition, ConstAnyShape<64, 20, 64>>);

/// Create a seed world with some collapse involved
pub fn floor() -> World {
    let extent = FlatPaddedGridCuboid::<(), ConstAnyShape<64, 20, 64>>::new([-32, -8, -32].into());
    use Voxel::*;
    let world: FlatPaddedGridCuboid<Superposition, ConstAnyShape<64, 20, 64>>
        = extent.map_index(|i, _| {
            if i == [0,1,0].into() { [Empty].as_slice().into() }
            else if i == [0,0,0].into() { [Grass].as_slice().into() }
            else { Superposition::free() }
        })
        .into();
    World(world)
}

/// Converts between wfc representation and the one for rendering.
#[derive(Clone, Copy)]
struct Palette {}

impl wfc::palette::Palette<Voxel> for Palette {
    fn get(id: wfc::VoxelId) -> Voxel {
        use Voxel::*;
        match id {
            0 => Empty,
            1 => Grass,
            2 => Concrete,
            3 => Wood,
            4 => Glass,
            _ => panic!(),
        }
    }
    fn to_ref(v: Voxel) -> wfc::VoxelId {
        use Voxel::*;
        match v {
            Empty => 0,
            Grass => 1,
            Concrete => 2,
            Wood => 3,
            Glass => 4,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sanity() {
        for id in 0..5 {
            let v = Palette::get(id);
            let r = Palette::to_ref(v);
            assert_eq!(r, id);
        }
    }
}
