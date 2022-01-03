/*! Stress analysis in voxel worlds.
 *
 * Gravity constant is 1.0, so is the unit area.
 */

use baustein::indices::Neighbours6;
use baustein::traits::{ Extent, IterableSpace, Space };
use baustein::world::FlatPaddedCuboid;
use float_ord::FloatOrd;
use std::ops;


#[derive(Clone, Copy)]
struct Mass(f32);

impl Mass {
    fn get_gravity_force(self) -> Force {
        Force(self.0)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Force(f32);

impl ops::Add<Force> for Force {
    type Output = Force;
    fn add(self, other: Force) -> Force {
        Force(self.0 + other.0)
    }
}

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

/// Forces in the down direction
type SixForces = Neighbours6<Force>;

/// Returns the total load acting on the voxel
/// This is only a magnitude. This should be used to determine destruction. 
fn get_load(sf: SixForces) -> Force {
    // Maximum force. It's rather naive but fast
    Force(
        sf.0.iter()
            .map(|f| f.0.abs())
            .map(|f| FloatOrd(f))
            .max()
            .map(|f| f.0)
            .unwrap_or(0.0)
    )
}

/// Same as get_load, but maybe a bit more accurate. Slower?
fn get_load_sum(sf: SixForces) -> Force {
    Force(
        sf.0.iter()
            .map(|f| f.0)
            .filter(|f| *f > 0.0)
            .sum()
    )
}

/// Stores force imbalance between neigboring voxels.
/// Because imbalance is the same (but negative) in each direction,
/// only 3 neighbors are stored, each in positive axis direction.
#[derive(Clone, Copy)]
pub struct ThreeForces([Force; 3]);

impl ThreeForces {
    /// The total unbalanced downwards force acting on the voxel.
    fn imbalance(&self) -> Force {
        self.0[0] + self.0[1] + self.0[2]
    }
}

/// Creates the initial internal force distribution.
/// Forces are the initial unbalanced forces acting on each voxel in separation,
/// like gravity.
/// Both spaces must cover the same area.
fn get_initial_forces<FS, VS>(forces: &FS, voxels: &VS) -> FlatPaddedCuboid<SixForces>
    where
    FS: Space<Voxel=Force> + Extent + IterableSpace,
    VS: Space<Voxel=StressVoxel> + Extent + IterableSpace,
{
    let sf = voxels.map(|_sv| SixForces::default());
    voxels
        .zip(forces)
        .zip(&sf)
        .map_index(|i, v| voxel::distribute_forces(voxels, i, v))
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
    
fn distribute<FS, BS, VS>(space: &VS, balance: &BS, forces: &FS)
    -> FlatPaddedCuboid<SixForces>
where
    FS: Space<Voxel=SixForces> + Extent + IterableSpace,
    VS: Space<Voxel=StressVoxel> + Extent + IterableSpace,
    BS: Space<Voxel=ThreeForces> + Extent + IterableSpace,
{
    let b = balance.map(|tf| tf.imbalance());
    space
        .zip(&b)
        .zip(forces)
        .map_index(|i, v| voxel::distribute_forces(space, i, v))
        .into()
}

/*
fn solve<SF, SV, SO>(external_forces: &SF, space: &SV, threshold: f32)
    -> impl Space<Voxel=Force>
where
    SF: Space<Voxel=Force>,
    SV: Space<Voxel=StressVoxel>,
{
    let mut sixforces = get_initial_forces(external_forces, space);
    loop {
        let balance = process_newton_discrepancy(sixforces);
        // Yield sixforces, balance here for stepped execution.
        if get_newton_global_loss(balance) < threshold {
            return sixforces.map(|sf| get_load_sum(sf))
        }
        sixforces = distribute(space, balance, sixforces);
    }
}
*/
/// That which applies voxel-wise.
mod voxel {
    use baustein::indices::{Index, Neighbours6};
    use baustein::traits::Space;
    use super::StressVoxel;
    use super::{Force, SixForces, ThreeForces};

    /// Distribute unbalances forces.
    /// Spreads discrepancies across neighbors.
    /// Hopefully, this converges.
    pub fn distribute_forces<S: Space<Voxel=StressVoxel>>(
        space: S,
        index: Index,
        v: ((StressVoxel, Force), SixForces),
    ) -> SixForces {
        let ((sv, force), sf) = v;
        use StressVoxel::*;

        match sv {
            Empty | Bedrock => Default::default(),
            Bound => {
                let mut forces_ordered = sf.0;
                let mut nbs = index.neighbours6().0.iter()
                    .map(|i| space.get(*i))
                    // line up with slots for forces
                    .zip(forces_ordered.iter_mut())
                    // consider only neighbors that can share forces on all sides for simplicity
                    .filter(|(v, _force)| match v {
                        Bound | Bedrock => true,
                        Empty => false,
                        _ => todo!(),
                    })
                    // actual content of the neighbor not needed any more
                    .map(|(_v, force)| force)
                    .collect::<Vec<_>>();
                // nbs now contains force slots from forces_ordered,
                // one for each side neighboring a voxel that can take forces
                let force_share = force.0 / nbs.len() as f32;
                for neighbor in nbs {
                    *neighbor = Force(force_share);
                }
                // all forces have been filled in, turn them into the sixforces array
                Neighbours6(forces_ordered)
            },
            //Loose {mass, ..} | Bedrock{mass} => {},
            _ => todo!(),
        }
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
