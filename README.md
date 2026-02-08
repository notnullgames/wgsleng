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

By default it will show bob-example, but you can switch to others:

## Examples

These all work local, and on the web:

- [bob](https://notnullgames.github.io/wgsl-engine/): Demo with sprites and audio
- [logo](https://notnullgames.github.io/wgsl-engine/#examples/logo/main.wgsl): Shows how to draw things.
- [snake](https://notnullgames.github.io/wgsl-engine/#examples/snake/main.wgsl): (INCOMPLETE) Classic snake game

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

Build example zips:

```bash
npm run game
```
