/*! Stress analysis in voxel worlds.
 *
 * Gravity constant is 1.0, so is the unit area.
 *
 *
 * ## Algorithm
 *
 * The algorithm calculates the static load in a system.
 * Static means that the system is in equilibrium
 * and the result applies to a single moment in time.
 *
 * Some assumptions for simplicity:
 * - all forces act in the same direction
 * - no torque
 * - compression and stretching are equivalent
 *
 * This means the direction of gravity doesn't matter,
 * and levers don't work.
 *
 * ### Forces
 *
 * Each voxel has outward forces acting on nieghbors.
 * Those neighbors treat those same forces as inwards relative to them.
 * 
 * ### Bedrock
 *
 * Bedrock is a magical voxel type that limits the scope of our simulation
 * by letting laws of physics (constants) break around it.
 * Namely, its outward forces are always made to be equal to inward ones
 * (it can take on any load).
 * 
 * ### Constants:
 *
 * Constants are preserved at each iteration of the calculation.
 *
 * 1. For each voxel, sum(outward forces) = weight
 *
 * ### Goals:
 *
 * Goals are not satisfied in the beginning, but hopely get closer with each iteration.
 * 
 * 1. For each voxel, sum(outward forces) = sum(inward forces)
 *
 * ## TODO
 *
 * The distribution converges rather slowly in the case of 2 elements and a bedrock.
 * Possible improvements:
 * 1. When neighboring voxel request F force,
 * overshoot in compensation and give a*F. At least once in a while.
 * 2. Move bedrock handling to the neighbors:
 * let them dump *all* of their forces on the first found one.
 * Note that this won't solve propagation far away from bedrock.
 */

use baustein::indices::{Neighbours6, NamedNeighbours6};
use baustein::traits::{ Extent, IterableSpace, Space };
use baustein::world::FlatPaddedCuboid;
use float_ord::FloatOrd;
use genawaiter::{rc::gen, yield_, Generator};
use std::fmt;
use std::ops;


#[derive(Clone, Copy)]
struct Mass(f32);

impl Mass {
    fn get_gravity_force(self) -> Force {
        Force(self.0)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Force(pub f32);

impl fmt::Debug for Force {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl ops::Neg for Force {
    type Output = Force;
    fn neg(self) -> Force {
        Force(-self.0)
    }
}

impl ops::Add<Force> for Force {
    type Output = Force;
    fn add(self, other: Force) -> Force {
        Force(self.0 + other.0)
    }
}

impl ops::Sub<Force> for Force {
    type Output = Force;
    fn sub(self, other: Force) -> Force {
        Force(self.0 - other.0)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Stress(pub f32);


/// This kind of analysis requires bedrock to exist:
/// voxels which can take on any stress.
#[derive(Clone, Copy)]
pub enum StressVoxel {
    /// Forces acting on this are always 0
    Empty,
    /// Can take shear forces from other voxels of the same class,
    /// can take press/pull forces from any voxels,
    /// when forces exceed strength, it turns into a loose voxel
    // TODO: which loose voxel?
    Bound,
    /// Like Bound with class 0, infinite strength.
    /// Exists to make analysis stop.
    Bedrock,
}

impl Default for StressVoxel {
    fn default() -> Self {
        StressVoxel::Empty
    }
}

/// Forces in the down direction
type SixForces = Neighbours6<Force>;

/// Returns the total load acting on the voxel
/// This is only a magnitude. This should be used to determine destruction.
fn get_stress(sf: SixForces) -> Stress {
    // Maximum force. It's rather naive but fast
    Stress(
        sf.0.iter()
            .map(|f| f.0.abs())
            .map(|f| FloatOrd(f))
            .max()
            .map(|f| f.0)
            .unwrap_or(0.0)
    )
}

/// Same as get_load, but maybe a bit more accurate. Slower?
/// Caution: doesn't work on objects of negative weight.
pub fn get_stress_sum(sf: SixForces) -> Stress {
    Stress(
        sf.0.iter()
            .map(|f| f.0)
            .filter(|f| *f > 0.0)
            .sum()
    )
}

/// Stores force imbalance between neigboring voxels.
/// Because imbalance is the same (but negative) in each direction,
/// only 3 neighbors are stored, each in positive axis direction.
#[derive(Clone, Copy, Default)]
pub struct ThreeForces([Force; 3]);

impl ThreeForces {
    /// The total unbalanced downwards force acting on the voxel.
    pub fn imbalance(&self) -> Force {
        self.0[0] + self.0[1] + self.0[2]
    }
}

pub fn process_newton_discrepancy<S>(space: &S)
    -> FlatPaddedCuboid<ThreeForces>
where S: Space<Voxel=SixForces> + Extent + IterableSpace
{
    // Actually, no padding is needed.
    // While ThreeForces stores only in 3 directions,
    // and 3 negative directions out of 6 forces will get lost
    // (because map_index is free not to iterate over empty voxels),
    // those are not supposed to be part of the simulation extent and will be clamped to 0.
    // This presumes that voxels outside of iteration extent are empty.
    // let space = space.pad([[-1, 0, 0], [0, -1, 0], [0, 0, -1]]);
    space
        .map_index(|i, v| voxel::get_newton_discrepancy(space, i, v))
        .into()
}

/// How far away from perfect match we are (squares).
/// Maybe it's better to find maximum?
/// Returns a loss value in the squared, not linear, space.
fn get_newton_global_loss<S>(space: &S) -> f32
    where S: Space<Voxel=ThreeForces> + IterableSpace
{
    let mut sum = 0.0;
    space
        .visit_indices(|i| {
            sum += space.get(i)
                .0
                .iter()
                .map(|f| f.0 * f.0)
                .sum::<f32>()
        });
    sum
}

/// Initializes the outwardly forces array.
/// Just sets it to 0, bounded by the voxel storage.
/// This function exists for convenience only.
pub fn get_initial_forces<S>(voxels: &S) -> FlatPaddedCuboid<SixForces>
    where
    S: Space<Voxel=StressVoxel> + Extent + IterableSpace,
{
    voxels.map(|_sv| SixForces::default()).into()
}

/// The main step of the solver.
/// `weights` is the space with unchanging weight forces acting on voxels.
/// `forces` contains the current estimate of forces acting between voxels.
/// The return value is the next estimate of forces between voxels.
pub fn distribute<FS, WS, VS>(space: &VS, weights: &WS, forces: &FS)
    -> FlatPaddedCuboid<SixForces>
where
    FS: Space<Voxel=SixForces> + Extent + IterableSpace,
    VS: Space<Voxel=StressVoxel> + Extent + IterableSpace,
    WS: Space<Voxel=Force> + Extent + IterableSpace,
{
    let outward = space.zip(forces);
    space
        .zip(weights)
        .map_index(|i, v| voxel::distribute_forces(&outward, i, v))
        .into()
}

/// Convenience function for calculating how far from reaching the goal we are.
fn calculate_loss<FS>(forces: &FS) -> f32
    where FS: Space<Voxel=SixForces> + Extent + IterableSpace
{
    let balance = process_newton_discrepancy(&forces);
    get_newton_global_loss(&balance)
}

/// An example application of the solver.
/// Probably not the best idea to actually use it,
/// because it doesn't ofer a way out if there's no convergence.
///
/// Returns the magnitude of internal stresses for each voxel.
/// This must act on a continuous block,
/// and that block must be attached to a Bedrock.
/// Detached pieces will carry nonsense results:
/// 1x1x1 contributes 0 loss and experiences 0 strain,
/// while bigger ones contributes to loss but carries no strain.
pub fn solve<'a, SF, SV>(weights: &'a SF, space: &'a SV, threshold: f32)
    -> FlatPaddedCuboid<Stress>
where
    SF: Space<Voxel=Force> + Extent + IterableSpace,
    SV: Space<Voxel=StressVoxel> + Extent + IterableSpace,
{
    let mut forces = get_initial_forces(space);
    // forces are zeroed at this stage.
    loop {
        // Insert some values into forces
        forces = distribute(space, &weights, &forces);
        // Optional check for quality of the result.
        // Does not need to be done on each loop,
        // but it's needed to stop.
        {
            // Overall divergence from Newton's laws. Closer to 0 is better.
            let overall = calculate_loss(&forces);
            if overall < threshold {
                return forces.map(|sf| get_stress_sum(sf)).into()
            }
        }
    }
}

/// That which applies voxel-wise.
mod voxel {
    use baustein::indices::{Index, NamedNeighbours6, Neighbours6};
    use baustein::traits::Space;
    use std::cmp;
    use super::StressVoxel;
    use super::{Force, SixForces, ThreeForces};

    /// Breaks constant 1. to reach goal 1. immediately.
    /// This is a middle-of-the-way calculation
    fn counteract_inwards_forces<S: Space<Voxel=SixForces>>(
        s: S,
        index: Index,
    ) -> NamedNeighbours6<Force> {
        let nbs = index.neighbours6();
        
        NamedNeighbours6{
            xp: -s.get(nbs.xp()).xm(),
            xm: -s.get(nbs.xm()).xp(),
            yp: -s.get(nbs.yp()).ym(),
            ym: -s.get(nbs.ym()).yp(),
            zp: -s.get(nbs.zp()).zm(),
            zm: -s.get(nbs.zm()).zp(),
        }
    }

    /// Preserves outwardly forces balance (constant 1.)
    /// by adjusting forces to each neighbour by the same magnitude (not fraction).
    fn preserve_internal_forces<S: Space<Voxel=(StressVoxel, SixForces)>>(
        space: S,
        index: Index,
        v: ((StressVoxel, Force), NamedNeighbours6<Force>),
    ) -> SixForces {
        let ((sv, weight), outward) = v;
        use StressVoxel::*;

        match sv {
            // No material, no forces
            Empty => Default::default(),
            // Magical unphysical material, already matches goal 1.
            // (from counteraction that just happened),
            // its outwardly forces are defined by forces acting on it,
            // so nothing to be done to preserve balance.
            Bedrock => outward.into(),
            // Make the total of outwardly forces to be equal to weight.
            Bound => {
                let outward: Neighbours6<_> = outward.into();
                let outward_sum = Force(
                    outward.0.iter()
                        .map(|f| f.0)
                        .sum()
                );
                // How far from =weight are we? Redistribute that across neighbours.
                let to_distribute = weight - outward_sum;

                // This contains the result
                let mut forces_ordered = outward.0;
                let nbs = index.neighbours6().0.iter()
                    .map(|i| space.get(*i).0)
                    // line up with slots for forces
                    .zip(forces_ordered.iter_mut())
                    // Consider neighbors that can receive forces.
                    .filter(|(v, _force)| match v {
                        Bound | Bedrock => true,
                        Empty => false,
                        _ => todo!(),
                    })
                    // actual content of the neighbor not needed any more
                    .map(|(_v, force)| force)
                    .collect::<Vec<_>>();
                // nbs now contains force slots from forces_ordered,
                // one for each side neighboring a voxel that can take forces.

                // Avoid division by 0, which would happen for lonely blocks.
                // There's no reason to let those interfere, even if themselves can't be handled.
                let neighbor_count = cmp::max(nbs.len(), 1);
                let force_share = Force(to_distribute.0 / neighbor_count as f32);
                // The neighbors contain forces from the previous step:
                // balanced against inward forces.
                // References forces_ordered array.
                for neighbor in nbs {
                    *neighbor = *neighbor + force_share;
                }
                // Now outward forces are not guaranteed
                // to be balanced against inward ones,
                // but guaranteed to sum up to weight.
                Neighbours6(forces_ordered)
            },
            //Loose {mass, ..} | Bedrock{mass} => {},
            _ => todo!(),
        }
    }
    
    /// Distribute outwardly forces.
    /// 1. Breaks constant 1. to reach goal 1. immediately
    /// 2. Updates forces to restore constant 1.
    /// Spreads discrepancies across neighbors.
    /// Hopefully, this converges.
    pub fn distribute_forces<S: Space<Voxel=(StressVoxel, SixForces)>>(
        space: S,
        index: Index,
        v: (StressVoxel, Force),
    ) -> SixForces {
        let (sv, force) = v;
        use StressVoxel::*;

        let outwards = counteract_inwards_forces(space.map(|(_sv, sf)| sf), index);

        preserve_internal_forces(space, index, ((sv, force), outwards))
    }
 
    /// Sums up forces acting in the direction of gravity on each interface,
    /// showing how far from satisfying Newton's law we are (action = -reaction).
    pub fn get_newton_discrepancy<S: Space<Voxel=SixForces>>(
        space: &S,
        idx: Index,
        voxel: SixForces,
    ) -> ThreeForces {
        let nb = idx.neighbours6();
        ThreeForces([
            // Summing, because SixForces stores forces not in the direction of "outwards"
            // (which would have been opposite depending on who you ask),
            // but "downwards", which is always aligned.
            // The neighbor must *take* some force for it to be balanced.
            voxel.xp() + space.get(nb.xp()).xm(),
            voxel.yp() + space.get(nb.yp()).ym(),
            voxel.zp() + space.get(nb.zp()).zm(),
        ])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use baustein::world::FlatPaddedGridCuboid;
    use baustein::re::ConstPow2Shape;

/*
    struct Solver(FlatPaddedCuboid<SixForces>);

    impl Solver {
        fn new(space: &S) -> Self {
            Self(get_initial_forces(space))
        }
        fn step(space: &S, weights: &S) -> FlatPaddedCuboid<SixForces> {
        }
    }*/
    
    /// Checks a single bedrock voxel
    #[test]
    fn bedrock() {
        // 2x2x2
        type Shape = ConstPow2Shape<1, 1, 1>;
        let mut world = FlatPaddedGridCuboid::<StressVoxel, Shape>::new([0, 0, 0].into());
        world.set([0, 0, 0].into(), StressVoxel::Bedrock).unwrap();

        // For this algorithm, empty is ignored, and bedrock forces should too.
        let weights = world.map(|_v| Force(1.0));
        let outforces = get_initial_forces(&world);

        let outforces = distribute(&world, &weights, &outforces);
        
        let balance = process_newton_discrepancy(&outforces);

        // Bedrock may never be imbalanced
        assert_eq!(balance.get([0, 0, 0].into()).imbalance().0, 0.0);
    }

    /// Checks a single bound voxel.
    /// Actually, this will never work. the forces have nowhere to spread.
    #[test]
    fn bound1() {
        // 4x4x4
        type Shape = ConstPow2Shape<2, 2, 2>;
        let mut world = FlatPaddedGridCuboid::<StressVoxel, Shape>::new([0, 0, 0].into());
        world.set([1, 1, 1].into(), StressVoxel::Bound).unwrap();

        // For this algorithm, empty is ignored, and bedrock forces should too.
        let weights = world.map(|_v| Force(1.0));
        let outforces = get_initial_forces(&world);

        let outforces = distribute(&world, &weights, &outforces);
        
        let balance = process_newton_discrepancy(&outforces);

        assert_eq!(balance.get([1, 1, 1].into()).imbalance().0, 0.0);
    }

    /// Checks two voxels: one bound to bedrock.
    #[test]
    fn boundb() {
        // 4x4x4
        type Shape = ConstPow2Shape<2, 2, 2>;
        let mut world = FlatPaddedGridCuboid::<StressVoxel, Shape>::new([0, 0, 0].into());
        world.set([1, 1, 1].into(), StressVoxel::Bound).unwrap();
        world.set([1, 1, 2].into(), StressVoxel::Bedrock).unwrap();

        // For this algorithm, empty is ignored, and bedrock forces should too.
        let weights = world.map(|_v| Force(1.0));
        let mut outforces = get_initial_forces(&world);

        for i in 0..4 {
            outforces = distribute(&world, &weights, &outforces);
            let balance = process_newton_discrepancy(&outforces);
            println!("i {} bound {}", i, balance.get([1, 1, 1].into()).imbalance().0);
            println!("bedrock {}", balance.get([1, 1, 2].into()).imbalance().0);
            println!("{:?}", outforces.get([1, 1, 1].into()));
            println!("{:?}", outforces.get([1, 1, 2].into()));
        }
        
        let balance = process_newton_discrepancy(&outforces);

        let stresses = outforces.map(|sf| get_stress_sum(sf));
        assert_float_absolute_eq!(stresses.get([1, 1, 1].into()).0, 1.0);
        // get_load_sum is unable to calculate stress on bedrock,
        // because sum of forces will be negative
        
        // This should end up well balanced
        assert_float_absolute_eq!(balance.get([1, 1, 1].into()).imbalance().0, 0.0);
        assert_float_absolute_eq!(balance.get([1, 1, 2].into()).imbalance().0, 0.0);
    }

    /// Checks 3 voxels: one bound to bedrock.
    #[test]
    fn bound2b() {
        // 4x4x4
        type Shape = ConstPow2Shape<2, 2, 2>;
        let mut world = FlatPaddedGridCuboid::<StressVoxel, Shape>::new([0, 0, 0].into());
        world.set([1, 1, 1].into(), StressVoxel::Bedrock).unwrap();
        world.set([1, 1, 2].into(), StressVoxel::Bound).unwrap();
        world.set([1, 1, 3].into(), StressVoxel::Bound).unwrap();

        // For this algorithm, empty is ignored, and bedrock forces should too.
        let weights = world.map(|_v| Force(1.0));
        let mut outforces = get_initial_forces(&world);

        for i in 0..10 {
            outforces = distribute(&world, &weights, &outforces);
            let balance = process_newton_discrepancy(&outforces);
            println!("i {} bedrock {}", i, balance.get([1, 1, 1].into()).imbalance().0);
            println!("bound {}", balance.get([1, 1, 2].into()).imbalance().0);
            println!("bound {}", balance.get([1, 1, 3].into()).imbalance().0);
            
            println!("{:?}", outforces.get([1, 1, 1].into()));
            println!("{:?}", outforces.get([1, 1, 2].into()));
            println!("{:?}", outforces.get([1, 1, 3].into()));
        }
        
        let balance = process_newton_discrepancy(&outforces);

        let stresses = outforces.map(|sf| get_stress_sum(sf));
        assert_float_absolute_eq!(stresses.get([1, 1, 2].into()).0, 2.0, 0.1);
        assert_float_absolute_eq!(stresses.get([1, 1, 3].into()).0, 1.0, 0.1);
        // get_load_sum is unable to calculate stress on bedrock,
        // because sum of forces will be negative
        
        // This should end up well balanced
        assert_float_absolute_eq!(balance.get([1, 1, 1].into()).imbalance().0, 0.0, 0.1);
        assert_float_absolute_eq!(balance.get([1, 1, 2].into()).imbalance().0, 0.0, 0.1);
        assert_float_absolute_eq!(balance.get([1, 1, 3].into()).imbalance().0, 0.0, 0.1);
    }
}
