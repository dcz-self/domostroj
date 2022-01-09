/*! Serialization using serde */
use serde;
use serde::{Serialize, Serializer, Deserialize, Deserializer};

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

impl<'de, V: Deserialize<'de>, Shape: ConstShape> Deserialize<'de> for FlatPaddedGridCuboid<V, Shape> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
        D::Error: serde::de::Error,
    {
        let ret = FlatCuboid::deserialize(deserializer)?;
        let expected = ret.dimensions[0] * ret.dimensions[1] * ret.dimensions[2];
        if expected != ret.data.len() {
            return Err(serde::de::Error::invalid_length(ret.data.len(), &"Data length doesn't match dimensions"));
        }
        let offset = ret.offset.into();
        Ok(unsafe { Self::new_from_samples(ret.data, offset) })
    }
}
