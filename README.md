# Domostroj

Voxel house editor and generator using [bevy](https://github.com/bevyengine/bevy).

It's built on a 3D version of the [wave function collapse](https://github.com/mxgmn/WaveFunctionCollapse) algorithm.

Forked off feldspar-editor by bonsairobo.

## Status

Works as a sandbox for the wave function collapse algorithm. Houses are not amazing yet.

The stamp size is 3×3×3, and there are 4 materials (and empty) to choose from.

## Controls

Only the editor window is interactive at first.

Choose desired level with the slider, a material. and draw on the terrain with your mouse.

Then, click "update stamps" in the generator, and push the generation using the "step one" button until the area is completely collapsed.

### Camera

Hold the right mouse button to slide, hold the middle mouse button to look around.

## Roadmap

The big features:

- More material types (metal, dirt, water, color marker, custom marker...).
- Selectable stamp size
- Furniture meshes and markers
- Multiple configurable passes (e.g. in pass 2, use 4×4×3 stamp and treat all voxels marked "wall" the same)
- ...
- Stress analysis on drawn/generated structues

## TODO

The paper cuts:

- Use Bevy 0.6
- Use a renderer pipeline that can draw more than 4 materials
- Stop inefficiencies in rendering
- Generate in one step
- Show generation progress
- Editing voxels in generator
- Movable camera in generator
- Select voxel in generator window
- Voxel info in generator
- Changing voxels in generator
- Nicely visualised terrain borders
- Saving generator output
- Highlight low entropy tiles

## Licensing

There's a lot of MIT, Apache 2.0, and AGPL-3.0 code in *Domostroj*.
