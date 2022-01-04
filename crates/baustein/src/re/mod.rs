/*! Re-exports.
 * Use those to avoid getting a type confusion between crate versions. */

pub use ndshape;

/// Using any other version of ConstPow2 is a bad idea.
pub type ConstPow2Shape<const X: usize, const Y: usize, const Z: usize>
    = ndshape::ConstPow2Shape3usize<X, Y, Z>;
