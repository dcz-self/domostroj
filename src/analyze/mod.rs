/*! Stress analysis integration.
 * Includes world derivation, UI, and rendering.
 */
pub mod render;

use baustein::re::ConstPow2Shape;
use baustein::world::FlatPaddedGridCuboid;
use bevy::ecs::system::Commands;
use bevy::transform::components::Transform;

use render::{ Analyzed, StressChunk };
use crate::stress::{ get_initial_forces, distribute, get_stress_sum, process_newton_discrepancy, Force, StressVoxel };

// Used traits
use baustein::traits::Space;


fn test_bound2b() -> Analyzed {
    // 4x4x4
    type Shape = ConstPow2Shape<4, 4, 4>;
    let mut world = FlatPaddedGridCuboid::<StressVoxel, Shape>::new([0, 0, 0].into());
    world.set([1, 1, 1].into(), StressVoxel::Bedrock).unwrap();
    world.set([1, 1, 2].into(), StressVoxel::Bound).unwrap();
    world.set([1, 1, 3].into(), StressVoxel::Bound).unwrap();
    world.set([1, 1, 4].into(), StressVoxel::Bound).unwrap();
    world.set([1, 1, 5].into(), StressVoxel::Bound).unwrap();

    // For this algorithm, empty is ignored, and bedrock forces should too.
    let weights = world.map(|v| Force(1.0));
    let mut outforces = get_initial_forces(&world);

    for i in 0..30 {
        outforces = distribute(&world, &weights, &outforces);
        let balance = process_newton_discrepancy(&outforces);

    }
    
    let balance = process_newton_discrepancy(&outforces);

        println!("bedrock {}", balance.get([1, 1, 1].into()).imbalance().0);
        println!("bound {}", balance.get([1, 1, 2].into()).imbalance().0);
        println!("bound {}", balance.get([1, 1, 3].into()).imbalance().0);
        
        println!("{:?}", outforces.get([1, 1, 1].into()));
        println!("{:?}", outforces.get([1, 1, 2].into()));
        println!("{:?}", outforces.get([1, 1, 3].into()));

    let stresses = outforces.map(|sf| get_stress_sum(sf));

    let stresses = world
        .zip(&stresses)
        .map(|(v, s)| match v {
            StressVoxel::Empty => render::Voxel::Empty,
            _ => render::Voxel::Stressed(s.0 * 50.0),
        })
        .into();

    Analyzed(stresses)
}


pub fn spawn_test_chunk_2b(
    mut commands: Commands,
) {
    commands.spawn()
        .insert(test_bound2b())
        .insert(Transform::from_xyz(5.0, 5.0, 5.0));
}
