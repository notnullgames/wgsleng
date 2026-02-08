# WGSL Game Engine

A minimal game engine that runs games written entirely in WGSL (WebGPU Shading Language). Games are distributed as a single `.wgsl` file or a `.zip` containing `main.wgsl` and assets.

## Features

- **Single-file games**: Write your entire game in WGSL
- **Web runtime**: Run games in any WebGPU-capable browser
- **Native runtime**: Standalone Rust-based player for desktop
- **Simple input**: Supports arrow keys/WASD for movement, Z/X (or K/L) for A/B buttons
- **Asset support**: Include images and audio in zip format

## Quick Start

### Web

```bash
npm start
```

#### Examples

These all work local, and on the web:

- [bob](https://notnullgames.github.io/wgsl-engine/): Demo with sprites and audio
- [logo](https://notnullgames.github.io/wgsl-engine/#examples/logo/main.wgsl): Shows how to draw things without images
- [snake](https://notnullgames.github.io/wgsl-engine/#examples/snake/main.wgsl): (INCOMPLETE) Classic snake game

### Native

You can find the CLI for your platform at [releases](https://github.com/notnullgames/wgsl-engine/releases).

Or if you want to build it yourself (requires rust):

```sh
# build the native CLI for your platform
npm run native

# load logo example, can also be a zip-file
./native/target/release/wgsleng examples/logo/main.wgsl
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

let uv = (dist + 32.0) / 64.0;
let sprite = textureSampleLevel(@texture("player.png"), @engine.sampler, uv, 0.0);
```
