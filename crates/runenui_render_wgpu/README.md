# `runenui_render_wgpu`

> **Category: Library reference**

`runenui_render_wgpu` is RunenUI's reusable concrete wgpu renderer edge over ordinary public paint publications. It owns disposable GPU realization and target state; it does not own widget behavior, semantic identity, mounted/runtime authority, text shaping, logical layout, or a native event loop.

The public `Renderer` consumes the accepted `runenui_core`, `runenui_runtime`, and exact retained `runenui_text` shaped-resource contracts required by paint realization. Native hosts such as `reference_winit` and Counter keep window/event-loop policy outside this crate while using the same renderer for real native presentation.

## Ownership

The renderer owns:

- wgpu instance, adapter, device, queue, offscreen targets, and retained native surface state;
- renderer-local target generations and successful-publication lineage;
- validation and realization of the currently supported paint-scene subset;
- caller-provided external image realization/cache state keyed by complete opaque `ResourceRef` values;
- disposable per-glyph SDF/MSDF generation, quality selection, atlas pages, textures, pipelines, and shaders for exact retained shaped-text resources;
- native surface configuration/acquisition/render/present mechanics and offscreen GPU readback;
- immutable renderer observations covering publication/update, target, resource, render, readback, and present stages.

The renderer does **not** own native event loops, application lifecycle policy, AccessKit, semantic trees/actions, mounted/layout storage, style resolution, font discovery, shaping, line breaking, logical text identity, or application resource identity.

## Construction and native presentation

`Renderer::request` constructs a headless renderer. `Renderer::request_with_display_handle` supplies an owned display connection without creating a surface. `Renderer::request_with_surface_target` creates and retains a native surface before selecting a compatible adapter while remaining independent of winit itself.

A native host then explicitly drives the retained target:

1. `configure_surface` establishes a non-zero physical extent and renderer-local target generation;
2. the runtime publishes one ordinary `PaintPublication` for the host's exact logical surface and raster scale;
3. `render_surface_publication` performs validation/resource preflight, acquires the native surface texture, encodes the same accepted mixed scene used by the offscreen path, submits GPU work, invokes the caller-owned pre-present boundary, presents, and only then commits successful surface lineage;
4. timeout, occlusion, outdated/suboptimal configuration, or surface loss are returned as structured host-visible errors so the host can retry, reconfigure, or recreate the renderer without moving UI authority into this crate.

The host remains responsible for window/event-loop ownership, mapping physical size and scale into RunenUI logical/raster facts, redraw/publication acknowledgement, retry policy, and renderer recreation. The renderer remains winit-free.

Surface creation follows wgpu platform requirements, including main-thread creation where required. Native surface formats are selected only from the accepted sRGB formats advertised by the compatible adapter; unsupported or empty format sets fail structurally rather than changing the color contract.

## Supported scene and resource path

The implementation fails closed before target mutation or GPU submission when a publication cannot be represented by the current renderer subset. Supported production realization includes the accepted literal fills, centered strokes, images, retained shaped-text runs, finite affine transforms, conjunctive clips, scene opacity, and ordered source-over composition used by current RunenUI publications. Unknown or unsupported primitives remain explicit failures rather than being reinterpreted by the renderer.

Canonical `SceneRequirements` / `SceneCapabilities` remain renderer-neutral runtime contracts. Narrow renderer implementation checks and detailed rejection reasons stay renderer-local and do not become a second scene vocabulary.

External image payloads are caller-owned non-zero, tightly packed, unpremultiplied RGBA8 sRGB sources. The complete opaque `ResourceRef` is the provider/cache identity. The renderer never decodes PNG data in production; fixture PNG decoding belongs only to test providers.

The caller-owned `ResourceProvider` resolves external resources such as images. Runtime-shaped text does not round-trip through that provider: the exact immutable shaped-resource binding is retained by `PaintPublication` and consumed directly by the renderer.

## Shaped text

RunenUI logical text authority remains outside the renderer. Runtime publication retains the exact scale-independent `ResourceRef -> ShapedTextResource` binding produced by the accepted text/layout path. The renderer consumes that already-shaped resource, extracts each required outline with Skrifa, generates per-glyph MSDF fields with `bymsdfgen-core`, packs deterministic renderer-local atlas pages, and reconstructs coverage in the GPU shader.

Raster scale and renderer quality affect only disposable realization. They do not change text content, line breaking, glyph selection, logical metrics, `ResourceRef` identity, or runtime layout. Resource/atlas cache loss can therefore be reconstructed from the retained logical publication without a runtime republish, provider lookup, reshaping, re-line-breaking, or resource remint.

Supported outline glyphs never silently fall back to alpha-raster text. COLR, SVG, bitmap/intrinsic-color glyphs, faux-bold requirements, invalid font data, and invalid outlines produce explicit structured diagnostics unless a future separately accepted resource/paint contract represents them truthfully.

## Geometry, color, and clipping

Accepted scene transforms remain finite affine transforms. Non-invertible geometry contributes no paint coverage rather than falling back to source rectangles. Renderer-internal raster construction may widen finite scene components for robust clipping/math, but the public logical geometry remains unchanged.

The exact continuous raster canvas is `logical_size * RasterScale`; integer texture extents are ceil-rounded storage/readback extents only. Fractional-scale padding does not become logical paint coverage.

Literal and text foreground colors are unpremultiplied sRGB8 inputs. RGB is decoded to straight linear color before blending; alpha remains linear and includes scene opacity. wgpu's non-premultiplied source-over blend produces the accumulated target, and the sRGB target applies storage transfer. Offscreen targets initialize to transparent black as target policy, not authored paint.

Clipped fills, strokes, images, and shaped runs use renderer-owned stencil/clip realization while preserving the same runtime-authored transforms, scene order, opacity, and target authority.

## Target lineage, retry, and cache loss

One retained offscreen target owns its texture, extent, format, renderer-local generation, and successful publication lineage. Target loss/recreation or extent/format changes reset that lineage and require full resynchronization. Pre-submission validation/resource failures do not mutate the retained target. Post-submission failures conservatively invalidate target state where correctness requires it.

Native surface reconfiguration similarly creates a new renderer-local target generation and resets successful surface-publication lineage while allowing disposable resource caches to remain reusable when valid.

`discard_offscreen_target` explicitly drops offscreen target realization. `discard_resource_cache` drops renderer-owned image and shaped-text realizations and invalidates successful target lineage so the next complete publication reconstructs resources through the ordinary production path.

Renderer observations are evidence of renderer-local work only; they do not replace runtime trace/publication authority.

## Evidence

Repository tests exercise the real wgpu path rather than a software expected renderer. Current evidence includes:

- real offscreen readback and checked-in PNG/golden coverage for accepted scene/resource behavior;
- transform, clipping, opacity/source-over, fractional raster-scale, and target-lineage/retry regressions;
- shaped-text SDF/MSDF realization, raster-scale changes, retained-publication retry, resource-cache re-realization, and explicit intrinsic-format diagnostics;
- the M8D responsive multiscript corpus, which regenerates a compact human-inspectable contact sheet when a real wgpu adapter is available while keeping automated logical/resource assertions authoritative.

Adapter-independent tests may exercise the same production geometry helpers for deterministic edge cases; they supplement rather than replace real-wgpu evidence.

## Boundaries and current limitations

The package must not become UI behavior authority. It must not depend on concrete widgets, semantic-tree behavior, mounted/layout storage, private runtime mutation seams, winit, or AccessKit. Renderer caches and device resources remain disposable derived state.

Current supported text realization is outline SDF/MSDF; intrinsic COLR/SVG/bitmap rendering remains unsupported with explicit diagnostics. Broader device-loss/platform breadth belongs to later platform work, not to an alternate text/layout/semantic path inside this renderer.

Exact public signatures and error variants are authoritative in source/Rustdoc. Conceptual cross-crate ownership is summarized in [`docs/architecture/public-api.md`](../../docs/architecture/public-api.md), and current accepted maturity is owned by [`docs/status.md`](../../docs/status.md).
