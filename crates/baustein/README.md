Baustein
=====

A set of types and abstractions for working with voxel data sets:
- easy indexing
- easy voxel transformation
- easy iteration over voxels
- copy-on-write world modification

It's inspired by feldspar, but chooses a set of different opinions.
feldspar's rendering pipeline is supported.

Priorities
-----------

In order of importance:
1. Easy to use
2. Powerful
3. Fast

Example
-----------

This illustrates how to fill a chunk with voxels based on their placement.

```
type World = FlatPaddedGridCuboid<PaletteVoxel, ConstPow2Shape<5, 5, 5>>;

/// Create a World with a grassy, diggable floor below level 0.
pub fn floor() -> World {
    let extent = FlatPaddedGridCuboid::<(), ConstPow2Shape<5, 5, 5>>::new([0, -8, 0].into());
    // Transform all voxels within the Cuboid based on index
    let world = extent.map_index(|i, _| {
        if i.y() < 0 {
            PaletteVoxel(1) // hopefully grass
        } else {
            PaletteVoxel::EMPTY
        }
    });
    // Iterate over all the mapped voxels and package them up.
    world.into()
}
```

Usage
--------

In your `Cargo.toml`, add this:

```
[dependencies.baustein]
git = "https://github.com/dcz-self/domostroj"
rev = "c26aaba2b"
```

*Baustein* is still badly experimental, so you might want to update the revision occasionally.

License
---------

### Summary for users

*Baustein* contains pieces licensed under the "Apache-2.0 or MIT" scheme, as well as pieces under the LGPL 3.0 license.

That means: if you distribute modified versions of *Baustein*, you must make its sources available.

**If you use *Baustein* together with open source libraries only, you're most likely already covered.**

If you use *Baustein* in binary form together with closed libraries, then you must also make it possible for the user to replace *Baustein* with their own recompiled version.

*(This is just a quick summary, not guaranteed to be 100% accurate.)*

### Summary for cntributors

By contributing to this project, you agree to license your work under the LGPL 3.0 license or any later version.
