---
name: "bevy-procedural-surface-patterns"
description: "Decouple dense checkerboards or surface grids from mesh tessellation in Bevy PBR."
version: 1
created: "2026-09-30"
updated: "2026-09-30"
---
## When to Use
Use when a regular surface pattern creates excessive tile vertices or grid-line geometry as logical resolution increases.

## Procedure
1. Establish a geometry-budget regression using the actual mesh builder and startup systems at coarse and dense logical resolutions; record vertex/triangle counts and construction timing before changes.
2. Use a fixed tessellation with shared vertices, and a version-matched ExtendedMaterial<StandardMaterial, Extension> fragment shader. Pass logical resolution and grid visibility in aligned uniforms.
3. Invert the surface projection from interpolated position in the fragment shader; match the placement coordinate bases, signs, and seam conventions.
4. Filter checkerboard parity with a pixel-footprint integral using fwidth; compute grid-line distance in pixel units and fade lines when cells become subpixel.
5. Embed the shader with embedded_asset! and construct its embedded URI using env!("CARGO_CRATE_NAME") so #[path] example crates resolve their own asset namespaces.
6. Toggle a shader uniform rather than hiding the terrain entity. Test both material state and retained objects.
7. Run the real game's windowless screenshot verifier at ordinary and dense resolutions; inspect on/off/on images and shader logs.

## Pitfalls
- Fixed surface triangles approximate the ideal sphere: object-ground meshes following the mathematical sphere no longer share every terrain vertex. Document this or add conforming geometry if exact contact is required.
- Rust compilation cannot validate runtime WGSL imports, bindings, or pipeline specialization. Require a real renderer run.
- A checkerboard per tile with constant subdivisions has quadratic geometry growth; reducing subdivisions delays rather than removes that scaling.
- Report geometry reduction separately from measured FPS improvements.

## Verification
1. Terrain and grid geometry counts stay fixed across increased logical resolutions, and placement footprints/occupancy tests pass.
2. Formatter, compiler/linter, and tests pass.
3. Fresh rendered captures show terrain, objects, and working grid toggles without shader errors.