# agent-visuals

Offline renderer for generative 3D agent network animations and abstract streaming topologies.

Produces looping PNG frames (and optionally WebP/GIF) suitable for blog thumbnails, Open Graph images, and site heroes.

## Scenes

- **mesh** — 3D agents as soft glowing nodes with dynamic streaming connections and flowing particles between them.
- **field** — Abstract vector field of agent-like traces and residual streams (more to come).

## Build & Run

Requires Rust.

```bash
cargo run --release -- mesh --width 1200 --height 630 --frames 60 ./out
```

Then encode frames to a looping asset with your preferred tool (e.g. `img2webp` or `ffmpeg`).

## Design notes

Deterministic seeds, dark backgrounds, no labels, no UI chrome. Aimed at clean, fantastical technical imagery rather than diagrams.
