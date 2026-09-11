# Actual egui → Rust/WASM shape lab

This proof deliberately has **no DOM UI**. The page contains only a `<canvas>` and a small WebGL2 bridge.

Rust owns the UI state, hit-testing, dragging, sliders, pie controls, outer-tab rotation, Boolean merge, adaptive convex/concave fillets, and all drawing commands. `egui 0.36.2` turns the Rust paint commands into meshes; JavaScript only forwards browser pointer events and uploads egui's vertex/index buffers to WebGL2.

Pipeline:

```text
browser pointer event → Rust Event → egui::Context::run_ui
                                   → Rust UI + geometry
                                   → egui tessellation
                                   → mesh buffers
                                   → tiny WebGL2 presenter
```

Build:

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
```

The final single-file demo is made by base64-embedding the resulting `.wasm` into `web/index.template.html` at `__WASM_BASE64__`.
