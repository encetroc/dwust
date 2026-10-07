---
name: "cube-sphere-projection-consistency"
description: "Change cube-sphere projections while keeping shader patterns, voxel placement and picking aligned."
version: 1
created: "2026-10-01"
updated: "2026-10-01"
---
## When to Use
When reducing cube-sphere grid distortion or changing its coordinate mapping.

## Procedure
1. Measure distortion through actual grid directions: sum spherical triangle areas for each tile and compare largest/smallest; retain a regression bound.
2. Verify forward/inverse formulas against executable reference code. Equal-angle mapping on [-1,1] uses tan(pi*u/4) forward, atan(w)*4/pi inverse after dividing direction by face-normal component.
3. Centralize CPU forward/inverse transforms and route terrain vertices, voxel faces, snapping, support picking and anchor selection through them; mirror the inverse in the fragment shader.
4. Recompute nominal cell sizes and radial height units using pi*radius/2 per face-center arc when switching from linear to equal-angle; update fixtures that assume old physical heights or inverse formulas.
5. Run unit checks for round trips at multiple radii, seam agreement and distortion, then real windowless render and interaction journeys; inspect captures.

## Pitfalls
- Red Blob's appendix prose has inconsistent formulas; the page's demo shader implements atan(w)/(pi/4).
- Changing only the shader misaligns rendering with placement and contacts.
- Equal-angle reduces area variation but does not preserve equal area, aspect or right angles everywhere.

## Verification
1. At 64 cells per face, equal-angle largest/smallest spherical tile area is approximately 1.4 versus approximately 5.0 for normalized linear mapping.
2. Forward/inverse round trips, seam continuity and placement tests pass; geometry budget remains independent of logical resolution.
3. Rendering and grab/drop captures show aligned footprints and correct validity feedback.