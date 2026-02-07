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

Open `http://localhost:8080/#examples/logo/main.wgsl` in a WebGPU-compatible browser.

### Native

You can find the CLI for your platform at [releases](https://github.com/notnullgames/wgsl-engine/releases).

Or if you want to build it yourself (requires rust):

```bash
npm run native
./native/target/release/wgsleng examples/logo/main.wgsl
```

## Creating Games

Games can be a single WGSL file or a zip archive containing:

- `main.wgsl` (required)
- Asset files (`.png`, `.ogg`, etc.)

See the `examples/` directory for reference implementations.

## Examples

- **bob**: Demo with sprites and audio
- **logo**: Shows how to draw things.
- **snake**: (INCOMPLETE) Classic snake game

Build example zips:

```bash
npm run game
```
