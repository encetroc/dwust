# Bevy 0.20 improvements for Dwust

Dwust already depends on Bevy 0.20.0. This document records opportunities, not a commitment to implement them all.

Sources:
- [Official Bevy 0.20 release notes](https://bevy.org/news/bevy-0-20/)
- [Bevy scene documentation](https://docs.rs/bevy_scene/0.20.0/bevy_scene/)
- [WESL documentation](https://wesl-lang.dev/)

## Recommended priorities

1. Gradually adopt BSN for repeated HUD structures.
2. Use headless widgets when adding tabs or other controls.
3. Use WESL and custom materials for clearer combat feedback.
4. Consider font-relative sizing if adjustable UI scale becomes a requirement.

Keep changes incremental. No wholesale rewrite is necessary.

## BSN: reusable HUD construction

BSN first landed in Bevy 0.19; 0.20 improves its syntax and adds the scene `Ready` event.

Benefits for Dwust:
- Declarative entity trees make nested HUD hierarchies easier to read.
- Reusable scene functions can define ability icons, inventory slots, timeline entries, and panel chrome.
- Field-level patching allows variants to override individual style fields without replacing whole components.
- Named entity references reduce manual entity-ID wiring.
- Asset templates and the `Ready` event help initialize scenes once dependencies and entity hierarchies are available. `Ready` does not guarantee rendering pipelines or UI layout measurements are ready.

Recommended boundary:
- **BSN:** construction, hierarchy, initial styling, reusable scene composition.
- **Existing systems:** combat updates, dragging, input blocking, saved positions and visibility, content measurement, and 16px snapping.
- Keep `HudGrid` and reuse its layout definitions within BSN.

Start with ability icons or timeline entries in `src/combat.rs`, then migrate panels individually. Check appearance and interactions after each migration. Simple readable spawns can remain unchanged.

`src/bevy_020_lab.rs` already demonstrates BSN and tabs.

Limitations:
- BSN does not automatically provide reactive state updates or improve FPS.
- Scene resolution adds work; do not adopt it for high-volume procedural spawning expecting a speedup.
- Official external `.bsn` assets are not shipped in the installed 0.20 scene documentation. Do not assume live editing of `.bsn` files is available.
- Keep map/spell JSON and gameplay systems independent of HUD authoring.

## WESL: maintainable custom shaders

Bevy 0.20 adopts WESL for its shaders and removes the old custom Bevy WGSL dialect. Plain WGSL without preprocessing directives remains supported.

Benefits:
- Modules/imports allow shared ring, outline, grid, noise, and color helpers.
- Conditional compilation supports optional effects and cheaper shader variants.
- Imports expose Bevy rendering functions and types.
- Compatible shader tooling can provide navigation and diagnostics.

Potential uses:
- Spell-area previews and animated ground markers.
- Selected-unit highlights.
- Procedural terrain patterns.
- Sprite damage flashes, outlines, or dissolves through custom materials.

`assets/shaders/bevy_020_spell_ring.wesl` already implements an animated ring.

WESL improves code organization, not fundamentally available graphics techniques. Imports and conditionals existed in Bevy's previous shader dialect too. It does not automatically improve FPS, visual quality, draw-call counts, or ordinary UI behavior. Build shared shader modules only when effects genuinely share code.

## Other relevant improvements

| Feature | Opportunity for Dwust | Caveat |
| --- | --- | --- |
| Headless tab widgets | Inventory, equipment, and settings tabs with keyboard navigation, focus, and accessibility integration | Bring your own visuals; the lab already uses these |
| `em` / `rem` UI sizing | Adjustable text and spacing for accessibility and different screen sizes | Preserve final 16px panel snapping; font-relative sizing is not automatic whole-HUD scaling |
| Custom sprite materials | Shader effects directly on world sprites | Not automatically applicable to UI `ImageNode` elements |
| Extended 2D materials | Extend an existing 2D material for markers and spell previews | Does not require replacing ordinary UI with shaders |
| More Feathers widgets | Dropdowns, selectable scrolling lists, color inputs, and scrubbable number inputs for future authoring tools | Feathers is editor-oriented, not a ready-made Dwust HUD theme |
| Contextual Feathers theming | Different theme surfaces for dialogs, panels, and popups in authoring tools | Applies to Feathers; does not automatically theme the existing HUD |
| Pan-orbit camera | Map inspection and authoring/debug navigation | Keep the existing orthographic gameplay camera |
| Weak system ordering | More parallelism when strict ordering unnecessarily serializes systems | Audit dependencies and profile first; untracked ordering dependencies require strict ordering |
| Unified sprite rendering backend | Improved sprite performance in many cases | Already received by using 0.20; no guaranteed project-specific FPS increase |

## Low priority for the current game

- **Solari / DLSS:** substantial ray-tracing improvements, but hardware requirements and complexity are unlikely to justify adoption for the current stylized tactical game.
- **Mesh shaders:** advanced GPU geometry support with platform constraints and no web support; not an immediate need for Dwust.

## Verification when adopting changes

- Preserve existing HUD appearance, interactions, saved layouts, and input blocking.
- Check content-derived panel dimensions and 16px placement alignment.
- Test keyboard/focus behavior when introducing widgets.
- Use the project's headless capture/testing workflows for HUD and visual changes.
- Measure performance before claiming a rendering or scheduling improvement.
