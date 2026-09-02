# Bevy camera `RenderLayers` + shadows leak repro

Minimal standalone reproduction for an apparent memory leak / eventual crash when an existing 3D camera's `RenderLayers` value is repeatedly changed while shadow maps are enabled.

## Run

The repro uses a single camera and can be run with either Bevy's direct or indirect drawing path.

Direct drawing with shadows enabled:

```sh
cargo run -- --direct --shadows
```

Indirect drawing with shadows enabled:

```sh
cargo run -- --indirect --shadows
```

Controls with all shadow maps disabled:

```sh
cargo run -- --direct --no-shadows
cargo run -- --indirect --no-shadows
```

If neither drawing mode is specified, the repro defaults to `--indirect`. Shadows are enabled by default.

The test runs indefinitely. The layer-1 mesh owners stay fixed while the existing `RenderLayers` component on the single camera alternates between layer 0 and layer 1 every 0.5 seconds.

- `--direct` inserts `NoIndirectDrawing` on the camera.
- `--indirect` uses Bevy's normal indirect drawing path.
- `--shadows` enables shadow maps.
- `--no-shadows` disables shadow maps as a control.

Observed behavior in the originating test:

- With shadows enabled, memory grows continually, shadows become corrupted, and the process eventually crashes.
- With shadows disabled, the same repeated camera `RenderLayers` mutation does not appear to leak or corrupt shadows.

No environment variables or non-Bevy dependencies are used.

## Bevy features

The project uses `default-features = false` and enables only the public Bevy features directly required by the repro: assets, color, window/winit, image/mesh/camera/light/material, renderer, core 3D pipeline, and PBR. `std` / `async_executor` provide the normal native task runtime. `x11` is included solely so the project runs out of the box on Linux/X11; it is unrelated to the bug.

Broad feature collections such as `3d`, `3d_bevy_render`, UI, audio, glTF, scenes, picking, gizmos, animation, post-processing, and Bevy's meshlet features are intentionally not enabled.
