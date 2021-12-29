# Domostroj

Voxel house editor and generator using the [feldspar](https://github.com/bonsairobo/feldspar) voxel plugin for
[bevy](https://github.com/bevyengine/bevy).

Forked off feldspar-editor.

## Warning

This is very much a work in progress and very experimental. But we hope that eventually this will actually be
useful for making games.

## Controls

### Camera

Unreal Engine style mouse camera.

- Left drag: Locomotion
- Right drag: Change viewing angle
- Left and Right drag: Translate up/down/left/right

Orbital Camera
- CTRL + mouse: Orbit camera
- Middle click + mouse: Pan camera
- Mouse wheel: Zoom

### Editing Tools

- `T`: Enter terraforming mode
  - `Z`: create terrain
  - `X`: remove terrain
  - `1..4`: Select voxel type
  - `UP`/`DOWN`: Increase/decrease brush radius
- `D`: Enter face dragging mode
  - Click two face corners, then drag the highlighted region
- `U`: Undo last edit
- `R`: Redo last undone edit
- `Q`: Slicing mode: move mouse cursor to select voxel
  - `Z`: Add voxel
  - `X`: Clear voxel
  - `1..4`: Select voxel type
  - `M`: Raise selection level
  - `N`: Lower selection level

### Roadmap

- Generate world based on the template
- Save template
- Save world
- Select voxel types using a mouse
- Place voxels using the mouse
- Camera that doesn't take over the mouse so badly
- Stepwise generation
- Inspect generator probabilities
- Alter generated world while paused
