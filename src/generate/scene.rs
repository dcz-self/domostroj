/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
use baustein::re::ConstAnyShape;
use baustein::world::FlatPaddedGridCuboid;
use block_mesh;
use block_mesh::MergeVoxel;
use rand::rngs::StdRng;
use wfc_3d as wfc;
use wfc::wave;


use baustein::traits::Space;
use rand::SeedableRng;
// this is actually used. Rustc is just complaining.
use wfc_3d::palette::Palette as _;


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Voxel {
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

impl MergeVoxel for Voxel {
    type MergeValue = Self;
    fn merge_value(&self) -> Self {
        *self
    }
}

impl block_mesh::Voxel for Voxel {
    fn is_empty(&self) -> bool {
        match self {
            Voxel::Empty => true,
            _ => false,
        }
    }
    fn is_opaque(&self) -> bool {
        match self {
            Voxel::Empty => true,
            _ => false,
        }
    }
}

/// 5 dimensions - 5 distinct voxel possibilities.
pub type Superposition = wfc::palette::Superposition<Voxel, Palette, 5>;

pub type SceneShape = ConstAnyShape<10, 10, 10>;

/// A wrapper over a mundane chunk, for the purpose of becoming the Bevy resource.
pub struct World{
    pub wave: wave::Naive<SceneShape, 5>,
    pub rng: StdRng,
}

/// Create a seed world with some collapse involved
pub fn seed() -> World {
    let extent = FlatPaddedGridCuboid::<(), SceneShape>::new([-5, -5, -5].into());
    use Voxel::*;
    let world: FlatPaddedGridCuboid<wfc::Superposition<5>, SceneShape>
        = extent.map_index(|i, _| {
            if i == [0,1,0].into() { [Empty].as_slice().into() }
            else if i == [0,0,0].into() { [Grass].as_slice().into() }
            else { Superposition::free() }
        })
        .map(|v: Superposition| v.into())
        .into();
    World {
        wave: wave::Naive::new(world),
        rng: StdRng::seed_from_u64(0),
    }
}

/// Converts between wfc representation and the one for rendering.
#[derive(Clone, Copy, Debug)]
pub struct Palette {}

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

    use crate::edit;
    use crate::generate::collapse;

    #[test]
    fn sanity() {
        for id in 0..5 {
            let v = Palette::get(id);
            let r = Palette::to_ref(v);
            assert_eq!(r, id);
        }
    }

    pub type SceneShape = ConstAnyShape<5, 5, 5>;

    fn seed_test() -> FlatPaddedGridCuboid<wfc::Superposition<5>, SceneShape> {
        let extent = FlatPaddedGridCuboid::<(), SceneShape>::new([-4, -4, -4].into());
        use Voxel::*;
        extent.map_index(|i, _| {
            if i == [0,1,0].into() { [Empty].as_slice().into() }
            else if i == [0,0,0].into() { [Grass].as_slice().into() }
            else { Superposition::free() }
        })
        .map(|v: Superposition| v.into())
        .into()
    }
    use wfc_3d::stamp::ViewStamp;

    #[test]
    fn seed_entropy() {
        let converted_source
            = edit::floor().0
            .map(|v| v.0 as wfc::VoxelId)
            .into();
        let stamps = collapse::Stamps::from_source(converted_source);
        let world = seed_test();
        collapse::Stamps::rent(
            &stamps,
            |stamps| {
                let total = stamps.get_total_occurrences();
                let stamps = stamps.get_distribution();

                let sup = ViewStamp::new(&world, [-2, -2, -2].into());
                dbg!(&sup);
                dbg!(
                    wfc::get_distribution(&sup, stamps).count()//collect::<Vec<_>>()
                );
                

                let p = |offset: [i32; 3]| {
                    let template = ViewStamp::new(&world, offset.into());
                    dbg!(wfc::get_distribution(&template, stamps).count());
                    wfc::get_superposition_pseudo_entropy(&template, stamps, total)
                };
                dbg!(p([-1, -1, -1]));
                dbg!(p([-2, -2, -2]));
                dbg!(p([-4, -4, -4]));
                //panic!("{:?}", wfc::find_lowest_pseudo_entropy(&world, stamps, total));
            }
        );
    }
}
