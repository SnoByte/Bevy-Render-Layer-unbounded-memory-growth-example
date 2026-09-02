# Bevy camera `RenderLayers` + shadows leak repro

Minimal standalone reproduction for an apparent memory leak / eventual crash when existing 3D cameras' `RenderLayers` values are repeatedly changed while shadow maps are enabled.

## Run

Shadowed repro (default):

```sh
cargo run -- --shadows
```

Control with all shadow maps disabled:

```sh
cargo run -- --no-shadows
```

The test runs indefinitely. The layer-1 mesh owners stay fixed while the existing `RenderLayers` components on both cameras alternate between layer 0 and layer 1 every 0.5 seconds.

Observed behavior in the originating test:

- `--shadows`: memory grows continually, shadows become corrupted, and the process eventually crashes.
- `--no-shadows`: the leak/corruption does not reproduce.

No environment variables or non-Bevy dependencies are used.

## Bevy features

The project uses `default-features = false` and enables only the public Bevy features directly required by the repro: assets, color, window/winit, image/mesh/camera/light/material, renderer, core 3D pipeline, and PBR. `std` / `async_executor` provide the normal native task runtime. `x11` is included solely so the project runs out of the box on Linux/X11; it is unrelated to the bug.

Broad feature collections such as `3d`, `3d_bevy_render`, UI, audio, glTF, scenes, picking, gizmos, animation, post-processing, and Bevy's meshlet features are intentionally not enabled.
