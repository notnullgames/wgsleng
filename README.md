# WGSL Game Engine

A minimal game engine that runs games written entirely in WGSL (WebGPU Shading Language). Games are distributed as a single `.wgsl` file or a `.zip` containing `main.wgsl` and assets.

## Features

- **Single-file games**: Write your entire game in WGSL
- **Web runtime**: Run games in any WebGPU-capable browser
- **Native runtime**: Standalone Rust-based player for desktop
- **Simple input**: Keyboard mapped to SNES controller (see Controls below)
- **Asset support**: Include images and audio in zip format

## Quick Start

### Web

```bash
npm start
```

## Controls

The engine maps keyboard keys to a SNES-style controller:

| Button     | Key        | Color/Position |
| ---------- | ---------- | -------------- |
| **D-Pad**  | Arrow keys | Left side      |
| **A**      | X          | Red, right     |
| **B**      | Z          | Yellow, bottom |
| **X**      | S          | Blue, top      |
| **Y**      | A          | Green, left    |
| **L**      | Q          | Left shoulder  |
| **R**      | W          | Right shoulder |
| **SELECT** | Shift      | Bottom center  |
| **START**  | Enter      | Bottom center  |

#### Examples

These all work local, and on the web:

- [bob](https://notnullgames.github.io/wgsl-engine/): Demo with sprites and audio
- [bunny](https://notnullgames.github.io/wgsl-engine/#examples/bunny/main.wgsl): Demo that loads a 3D model
- [cubespin](https://notnullgames.github.io/wgsl-engine/#examples/cubespin/main.wgsl): Simple spinning 3d cube
- [input](https://notnullgames.github.io/wgsl-engine/#examples/input/main.wgsl): Provides some nice 2D drawing functions, and shows you the current state of the virtual controller
- [logo](https://notnullgames.github.io/wgsl-engine/#examples/logo/main.wgsl): Shows how to draw things without images
- [raymarch](https://notnullgames.github.io/wgsl-engine/#examples/raymarch/main.wgsl): Provides some nice 3D drawing functions with ray marching and SDF primitives
- [snake](https://notnullgames.github.io/wgsl-engine/#examples/snake/main.wgsl): Classic snake game
- [tetris](https://notnullgames.github.io/wgsl-engine/#examples/tetris/main.wgsl): Classic tetris game. This also includes text-rendering. It's a bit obtuse, but works.

### Native

You can find the CLI for your platform at [releases](https://github.com/notnullgames/wgsl-engine/releases).

Or if you want to build it yourself (requires rust):

```sh
# build the native CLI for your platform
npm run native

# load logo example, can also be a zip-file
./native/target/release/wgsleng examples/logo/main.wgsl

# enable live camera input (requires system camera access)
cargo build --release --features camera -p wgsleng
```

## Creating Games

Games can be a single WGSL file or a zip archive containing:

- `main.wgsl` (required)
- Asset files (`.png`, `.ogg`, etc.)

See the `examples/` directory for reference implementations.

Build example zips:

```bash
npm run game
```

### Using Tiled Maps

Create tile-based games using [Tiled](https://www.mapeditor.org/):

1. Create your map in Tiled and save as JSON (`.tmj`)
2. Convert to wgsleng format:
   ```bash
   pip install Pillow
   python3 tools/tiled_to_wgsl.py path/to/map.tmj
   ```
3. Import in your game:
   ```wgsl
   @import("map.wgsl")
   ```

See `examples/rpg/` for a complete example with collision detection.

The converter creates efficient texture-based maps (tile IDs encoded as pixels) which are much faster than array-based storage. See `tools/README.md` for details.

### image-only output

You might find it helpful to render WGSL to images (for LLM-comparison and things.) You can do that like this:

```sh
# output image of first-frame
npm run render examples/logo/main.wgsl /tmp/logo.png

# same, but also print the generated shader
DEBUG_SHADER=1 npm run render examples/logo/main.wgsl /tmp/logo.png

open /tmp/logo.png
```

### extensions to WGSL

The engine works by adding some extensions to the language. Assets are referenced by filename. The idea is that some uniforms/shared-buffers are automatically setup for you and bound, so it all works without you having to manage loading assets. You are meant to be able to control the entire game, just from the main.wgsl.

```wgsl
// this is available in @engine
struct GameEngineHost {
    buttons: array<i32, 12>, // the current state of virtual SNES gamepad (BTN_*)
    time: f32, // clock time
    delta_time: f32, // time since last frame
    screen_width: f32, // current screensize
    screen_height: f32, // current screensize
    state: GameState, // user's game state that persists across frames
    audio: array<u32, {SIZE}>, // audio trigger counters
}

// Button constants for input
const BTN_UP: u32 = 0u;
const BTN_DOWN: u32 = 1u;
const BTN_LEFT: u32 = 2u;
const BTN_RIGHT: u32 = 3u;
const BTN_A: u32 = 4u;
const BTN_B: u32 = 5u;
const BTN_X: u32 = 6u;
const BTN_Y: u32 = 7u;
const BTN_L: u32 = 8u;
const BTN_R: u32 = 9u;
const BTN_START: u32 = 10u;
const BTN_SELECT: u32 = 11u;


// Define this struct for holding your data between frames
// You can put whatever you want in here
struct GameState {
  player_pos: vec2f
}

// set the title of the window
@set_title("Bob-Bonker");

// set the size of the window, defaults to 800x600
@set_size(600, 600);

// inline the source of another file (from your zip/dir)
@import("helpers.wgsl");

// ASSETS
@sound("bump.ogg").play();
@sound("bump.ogg").stop();

let pos = @model("bunny.obj").positions[idx];
let normal = @model("bunny.obj").normals[idx];

let uv = (dist + 32.0) / 64.0;
let sprite = textureSampleLevel(@texture("player.png"), @engine.sampler, uv, 0.0);

// VIDEO: play a looping video file (MP4, WebM, GIF, etc.) as a texture
// native: requires system ffmpeg for non-GIF formats
// web: uses <video> element
let frame = textureSample(@video("clip.mp4"), @engine.sampler, uv);

// CAMERA: sample a live camera feed as a texture (index 0 = default camera)
// native: build with --features camera
// web: uses getUserMedia
let cam = textureSample(@camera(0), @engine.sampler, uv);
```

## asset credits

- font is from [this 8x8](https://opengameart.org/content/8x8-ascii-bitmap-font-with-c-source)
- Bob Ross pixel-art is from [ravenist](https://www.pixilart.com/ravenist)
- tiles are from [lo-bit](https://finalbossblues.itch.io/lo-bit-pack)

## License

This project is licensed under the [zlib/libpng License](LICENSE) - a permissive open-source license that allows commercial use, modification, and redistribution with minimal restrictions.
