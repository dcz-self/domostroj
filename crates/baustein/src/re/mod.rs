/*! Re-exports.
 * Use those to avoid getting a type confusion between crate versions. */

pub use ndshape;

/// Using any other version of ConstPow2 is a bad idea.
pub type ConstPow2Shape<const X: usize, const Y: usize, const Z: usize>
    = ndshape::ConstPow2Shape3usize<X, Y, Z>;

/// Using any other version of this is a bad idea.
pub type ConstAnyShape<const X: usize, const Y: usize, const Z: usize>
    = ndshape::ConstShape3usize<X, Y, Z>;

/// Trait "reexport". No need for anything other then 3 and usize in baustein.
pub trait ConstShape : ndshape::ConstShape<3, Coord=usize> {
    const ARRAY: [usize; 3] = <Self as ndshape::ConstShape<3>>::ARRAY;
}

impl<const X: usize, const Y: usize, const Z: usize> ConstShape for ConstPow2Shape<X, Y, Z> {}
