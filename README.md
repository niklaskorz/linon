# Interactive Exploration of Nonlinear Ray Casting

The _linon_ project is the result of an advanced software practical in the [Visual Computing Group](https://vcg.iwr.uni-heidelberg.de/) at the Institute of Computer Science, Heidelberg University.
It is based on the [Rust](https://www.rust-lang.org/) programming language and the upcoming [WebGPU](https://web.dev/gpu/) graphics standard through the [wgpu](https://github.com/gfx-rs/wgpu) library.
At its core is the continuous evaluation of ray paths in a nonlinear field through [4th order Runge-Kutta integration](https://en.wikipedia.org/wiki/Runge%E2%80%93Kutta_methods).

Prebuilt binaries for Windows, macOS and Linux can be found in the [releases](https://github.com/niklaskorz/linon/releases) of the Github repository.
If you are running a nightly version of Firefox or Chrome and have the WebGPU flag enabled, you can also run [linon on the web](https://niklaskorz.github.io/linon/).
See https://web.dev/gpu/#enabling-via-about:flags for information on how to enable WebGPU in Chrome Canary.

## Usage

The application provides a sandbox for defining and exploring field functions that are evaluated inside a compute shader and rendered through ray casting.

![Screenshot](screenshot.png)

On the right side, the main view gives the possibility to explore the nonlinear scene using arcball camera controls.
The left side contains a reference view in which the path of rays is visualized linearly in a rasterized scene.
By clicking on a fragment of the main view, the rays around this fragment are visualized in the reference view.
The user can select from a list of predefined field functions on the top left of the application, and then edit the function inside the text editor below.
Through the "Overlay" dropdown, a [Lyapunov exponents](https://en.wikipedia.org/wiki/Lyapunov_exponent) overlay can be enabled to emphasize areas in image space for which rays diverge in behavior.
The "Outline" button next to it renders a path mesh for the rays on the outline of these divering areas.
If the image appears fragmented or inaccurate, the "Enhance" button can be used to rerender the current frame once using a smaller step size for Runge-Kutta integration.

## Build instructions

Compilation requires at least [Rust](https://www.rust-lang.org/) version 1.54 to be installed.
The preferred way of installing Rust is through [rustup](https://rustup.rs/).
Furthermore, an up to date graphics driver with support for Vulkan or DirectX 12 is assumed.
If you are on macOS, Apple's Metal graphics API will be used automatically by WebGPU.
Then, use cargo (included in Rust) for execution or compilation in the root of the repository.
The dependencies (listed in `Cargo.toml`) will be downloaded and built automatically by cargo.

```sh
# Build and run release build
cargo run --release
# Build and run debug build
cargo run
# Build release build (see target/ directory)
cargo build --release
# Build debug build (see target/ directory)
cargo build
```

To build the web version of linon, execute the following commands:

```sh
rustup target add wasm32-unknown-unknown
cargo install -f wasm-bindgen-cli --version 0.2.77
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --no-default-features --target wasm32-unknown-unknown --release
wasm-bindgen --out-dir public --web target/wasm32-unknown-unknown/release/linon.wasm
```

The resulting web application in the `public/` directory can then be served by an HTTP server of your choice.
