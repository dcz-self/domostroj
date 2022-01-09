/*! Serialization using serde */

use serde::{Serialize, Serializer, Deserialize};

use crate::re::ConstShape;
use crate::world::{FlatPaddedCuboid, FlatPaddedGridCuboid};

/// Helper for serialization without relying on random types.
#[derive(Serialize, Deserialize)]
struct FlatCuboid<V> {
    data: Vec<V>,
    offset: [i64; 3],
    dimensions: [usize; 3],
}

impl<V: Serialize + Clone, Shape: ConstShape> Serialize for FlatPaddedGridCuboid<V, Shape> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        FlatCuboid {
            data: self.data.clone(),
            offset: self.offset.into(),
            dimensions: <Shape as ConstShape>::ARRAY,
        }.serialize(serializer)
    }
}
