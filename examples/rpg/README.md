# RPG Example

A simple RPG demo using Tiled maps with collision detection.

## Features

- Loads Tiled map (level1) with multiple layers
- Tile-based collision detection
- Camera follows player
- Arrow keys to move

## Files

- `main.wgsl` - Main game logic and rendering
- `map/level1.tmj` - Tiled map file (JSON format)
- `map/level1.wgsl` - Exported map data (generated from Tiled)
- `map/*.png` - Tileset graphics
- `map/*.tsj` - Tileset definitions

## Running

### Web
```bash
npm start
```
Then navigate to: http://localhost:8080/#examples/rpg/main.wgsl

### Native
```bash
npm run native
./native/target/release/wgsleng examples/rpg/main.wgsl
```

## Map Structure

The level1 map has three tile layers:
- **ground** - Base terrain
- **stuff** - Objects, trees, decorations
- **collisions** - Invisible collision tiles

The player can't walk through tiles in the collision layer.

## Editing the Map

1. Install [Tiled](https://www.mapeditor.org/)
2. Open `map/level1.tmj`
3. Edit the map
4. File > Save (saves as JSON)
5. Run the converter:
   ```bash
   python3 tools/tiled_to_wgsl.py examples/rpg/map/level1.tmj
   ```

This generates:
- `level1_ground.png` - Ground layer as texture
- `level1_stuff.png` - Objects layer as texture
- `level1_collisions.png` - Collisions layer as texture
- `level1.wgsl` - Helper functions and metadata

## How It Works

**Efficient Texture-Based Format:**

Map layers are stored as PNG textures where each pixel encodes a tile ID:
- For maps with ≤255 tiles: R channel stores tile ID
- For maps with >255 tiles: RG channels store 16-bit tile ID

This approach is **much more efficient** than arrays because:
- GPU texture cache is optimized for 2D spatial access
- Smaller memory footprint
- Can handle very large maps
- Hardware-accelerated texture sampling

The renderer:
1. Converts screen position to world position (with camera offset)
2. Converts world position to tile coordinates
3. Reads tile ID from map texture using `textureLoad()`
4. Determines which tileset to use based on tile ID range
5. Calculates UV coordinates within the tileset
6. Samples the tileset texture

Collision is checked by reading a pixel from the collision layer texture.
