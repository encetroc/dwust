---
name: "bevy-grid-relocation-previews"
description: "Implement mouse-driven relocation previews for reserved grid objects in Bevy."
version: 3
created: "2026-09-30"
updated: "2026-09-30"
---
## When to Use
Use for click-to-grab placement objects with occupancy validation, transparent previews, and cancel/drop behavior.

## Procedure
1. Keep the original footprint reserved throughout grabbing. Validate candidate occupancy excluding only the original footprint; remove old cells and reserve new ones only after successful validation.
2. Separate cursor-ray sampling, hit testing, and interaction systems. Run ray sampling after camera transforms and visibility propagation; preserve a windowless ray-input seam for tests.
3. Pick the nearest eligible mesh, but account for terrain occlusion. Project dragging rays onto the terrain independently of object tops; reject intersections behind the ray origin.
4. Clone the held object's material and use AlphaMode::Blend; retain original mesh/material handles for cancellation. Use private preview/final meshes rather than mutating potentially shared originals.
5. Generate the contact patch from exactly the snapped integer footprint; radial depth-fighting lift must preserve cell-space boundaries. Use HUD validity feedback when opaque blockers hide the patch.
6. Reuse preview mesh assets as the pointer moves and refresh their picking AABBs when vertices move. Restore handles/bounds and remove temporary assets on drop or cancel.
7. Test the live contact mesh by inverse-projecting vertices into cell coordinates and asserting min/max equal origin and origin plus footprint. Cover initial grab, movement across faces and seams, and blocked previews.
8. Verify real input consumers using a render-headless camera journey; inspect screenshots and assert visible contact colors only where not occluded.
## Pitfalls
- ButtonInput::clear clears edge flags but keeps held buttons pressed. Reset or release before simulating another click.
- MinimalPlugins with VisibilityPlugin needs mesh/bounds resources; use AssetPlugin and MeshPlugin for actual raycast tests.
- Correct material color and ECS state do not prove an invalid contact patch is visible through an overlapping object.
- Document the windowless ray-input coverage boundary: it verifies camera conversion and gameplay, not OS cursor delivery.

## Verification
1. Focused tests cover exact contact bounds, own-cell overlap, partial overlap relocation, rejected-drop atomicity, cancellation, off-terrain rays, and picking after movement.
2. Formatter, compiler/linter, and the complete test suite pass.
3. Fresh captures show transparency, exact green free-placement contacts, blocked-placement HUD feedback (red patches may be occluded), opaque successful drops, and original-position cancellation.