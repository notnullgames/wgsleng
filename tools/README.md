# Tools

Utilities for working with wgsleng.

## tiled_to_wgsl.py

Converts Tiled JSON maps to efficient texture-based format for wgsleng.

### Installation

```bash
pip install Pillow
```

### Usage

```bash
python3 tools/tiled_to_wgsl.py path/to/map.tmj [output_dir]
```

If `output_dir` is not specified, files are created in the same directory as the input file.

### Example

```bash
# Convert level1 map
python3 tools/tiled_to_wgsl.py examples/rpg/map/level1.tmj

# Output to specific directory
python3 tools/tiled_to_wgsl.py examples/rpg/map/level1.tmj output/
```

### Output

For a map named `level1.tmj` with layers "ground", "stuff", "collisions":

**Generated files:**
- `level1_ground.png` - Ground layer as texture
- `level1_stuff.png` - Stuff layer as texture
- `level1_collisions.png` - Collisions layer as texture
- `level1.wgsl` - WGSL helper file with:
  - Map constants (width, height, tile size)
  - Tileset information
  - Helper functions (`get_tile`, `world_to_tile`, etc.)
  - Example usage code

### Texture Format

Map textures encode tile IDs in pixel values:

**8-bit (0-255 tiles):**
- R channel = tile ID
- Example: Tile ID 42 → RGB(42, 0, 0)

**16-bit (256-65535 tiles):**
- R channel = low byte
- G channel = high byte
- Example: Tile ID 610 → RGB(98, 2, 0) → 98 + (2 << 8) = 610

### Using in Your Game

```wgsl
@import("level1.wgsl")

@fragment
fn fs_render(@builtin(position) coord: vec4f) -> @location(0) vec4f {
    // Get tile ID from map texture
    let tile_coord = LEVEL1_world_to_tile(coord.xy);
    let tile_id = LEVEL1_get_tile_16bit(
        @texture("level1_ground.png"),
        tile_coord.x,
        tile_coord.y
    );

    // Get tile UV from tileset
    let tile_offset = fract(coord.xy / vec2f(LEVEL1_TILE_WIDTH, LEVEL1_TILE_HEIGHT));
    let uv = LEVEL1_get_tile_uv(LEVEL1_TILESET_GROUND, tile_id, tile_offset);

    // Sample tileset texture
    return textureSampleLevel(
        @texture("tileset.png"),
        @engine.sampler,
        uv,
        0.0
    );
}
```

### Benefits

**Texture-based maps are much more efficient than array-based:**

- ✓ GPU texture cache optimized for 2D access
- ✓ Smaller memory footprint
- ✓ Supports very large maps
- ✓ Hardware texture filtering
- ✓ Doesn't use storage buffer space

**When to use arrays vs textures:**

- **Textures** (recommended): Any map, especially >20×20 tiles
- **Arrays**: Only for tiny maps (<100 tiles total) or if you need >65535 unique tiles per layer
