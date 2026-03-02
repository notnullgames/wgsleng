// WGSL Game Engine Runtime for web
// Loads and runs expanded WGSL games with WebGPU

import { Unzip, AsyncUnzipInflate } from "fflate";

const decoder = new TextDecoder();

// Parse OBJ file format
function parseOBJ(objText) {
  const positions = [];
  const normals = [];
  const indices = [];

  const lines = objText.split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    const parts = trimmed.split(/\s+/);
    if (parts.length === 0) continue;

    if (parts[0] === "v" && parts.length >= 4) {
      // Vertex position
      positions.push([
        parseFloat(parts[1]),
        parseFloat(parts[2]),
        parseFloat(parts[3]),
      ]);
    } else if (parts[0] === "vn" && parts.length >= 4) {
      // Vertex normal
      normals.push([
        parseFloat(parts[1]),
        parseFloat(parts[2]),
        parseFloat(parts[3]),
      ]);
    } else if (parts[0] === "f" && parts.length >= 4) {
      // Face (triangle)
      for (let i = 1; i <= 3; i++) {
        const vertexData = parts[i].split("/");
        const posIndex = parseInt(vertexData[0]) - 1; // OBJ is 1-indexed
        indices.push(posIndex);
      }
    }
  }

  // If no normals in file, calculate them
  if (normals.length === 0) {
    const tempNormals = new Array(positions.length)
      .fill(null)
      .map(() => [0, 0, 0]);

    // Accumulate face normals for each vertex
    for (let i = 0; i < indices.length; i += 3) {
      const i0 = indices[i];
      const i1 = indices[i + 1];
      const i2 = indices[i + 2];

      const v0 = positions[i0];
      const v1 = positions[i1];
      const v2 = positions[i2];

      // Calculate edges
      const edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
      const edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

      // Cross product
      const normal = [
        edge1[1] * edge2[2] - edge1[2] * edge2[1],
        edge1[2] * edge2[0] - edge1[0] * edge2[2],
        edge1[0] * edge2[1] - edge1[1] * edge2[0],
      ];

      // Accumulate to each vertex
      tempNormals[i0][0] += normal[0];
      tempNormals[i0][1] += normal[1];
      tempNormals[i0][2] += normal[2];
      tempNormals[i1][0] += normal[0];
      tempNormals[i1][1] += normal[1];
      tempNormals[i1][2] += normal[2];
      tempNormals[i2][0] += normal[0];
      tempNormals[i2][1] += normal[1];
      tempNormals[i2][2] += normal[2];
    }

    // Normalize
    for (const n of tempNormals) {
      const len = Math.sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
      if (len > 0) {
        n[0] /= len;
        n[1] /= len;
        n[2] /= len;
      }
      normals.push(n);
    }
  }

  // If no normals in file, calculate smooth normals
  if (normals.length === 0) {
    const tempNormals = new Array(positions.length)
      .fill(null)
      .map(() => [0, 0, 0]);

    // Accumulate face normals for each vertex
    for (let i = 0; i < indices.length; i += 3) {
      const i0 = indices[i];
      const i1 = indices[i + 1];
      const i2 = indices[i + 2];

      const v0 = positions[i0];
      const v1 = positions[i1];
      const v2 = positions[i2];

      // Calculate edges
      const edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
      const edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];

      // Cross product
      const normal = [
        edge1[1] * edge2[2] - edge1[2] * edge2[1],
        edge1[2] * edge2[0] - edge1[0] * edge2[2],
        edge1[0] * edge2[1] - edge1[1] * edge2[0],
      ];

      // Accumulate to each vertex
      tempNormals[i0][0] += normal[0];
      tempNormals[i0][1] += normal[1];
      tempNormals[i0][2] += normal[2];
      tempNormals[i1][0] += normal[0];
      tempNormals[i1][1] += normal[1];
      tempNormals[i1][2] += normal[2];
      tempNormals[i2][0] += normal[0];
      tempNormals[i2][1] += normal[1];
      tempNormals[i2][2] += normal[2];
    }

    // Normalize
    for (const n of tempNormals) {
      const len = Math.sqrt(n[0] * n[0] + n[1] * n[1] + n[2] * n[2]);
      if (len > 0) {
        n[0] /= len;
        n[1] /= len;
        n[2] /= len;
      }
      normals.push(n);
    }
  }

  // Expand vertices based on indices (convert indexed mesh to vertex list)
  const expandedPositions = [];
  const expandedNormals = [];

  for (const idx of indices) {
    expandedPositions.push(positions[idx]);
    expandedNormals.push(normals[idx]);
  }

  return {
    positions: expandedPositions,
    normals: expandedNormals,
    vertexCount: expandedPositions.length,
  };
}

// get a file from a zip
async function extractSingleFile(zipData, targetFilename) {
  return new Promise((resolve, reject) => {
    let fileDataChunks = [];
    let found = false;

    const unzipper = new Unzip((file) => {
      if (file.name === targetFilename) {
        found = true;
        file.ondata = (err, chunk, final) => {
          if (err) {
            reject(err);
            return;
          }
          fileDataChunks.push(chunk);
          if (final) {
            const fullData = new Uint8Array(
              fileDataChunks.reduce((acc, curr) => acc + curr.length, 0),
            );
            let offset = 0;
            for (const chunk of fileDataChunks) {
              fullData.set(chunk, offset);
              offset += chunk.length;
            }
            resolve(fullData);
          }
        };
        file.start();
      } else {
        file.terminate();
      }
    });

    unzipper.register(AsyncUnzipInflate);
    unzipper.push(zipData, true);
    unzipper.ondata = (err, data) => {
      if (err && !found) {
        reject(
          new Error(`File ${targetFilename} not found or an error occurred.`),
        );
      }
    };
  });
}

const BTN_UP = 0,
  BTN_DOWN = 1,
  BTN_LEFT = 2,
  BTN_RIGHT = 3;
const BTN_A = 4,
  BTN_B = 5,
  BTN_X = 6,
  BTN_Y = 7;
const BTN_L = 8,
  BTN_R = 9,
  BTN_START = 10,
  BTN_SELECT = 11;

// Keyboard mapping to SNES controller
const KEY_MAP = {
  ArrowUp: BTN_UP,
  ArrowDown: BTN_DOWN,
  ArrowLeft: BTN_LEFT,
  ArrowRight: BTN_RIGHT,
  KeyX: BTN_A,
  KeyZ: BTN_B,
  KeyS: BTN_X,
  KeyA: BTN_Y,
  KeyQ: BTN_L,
  KeyW: BTN_R,
  Enter: BTN_START,
  ShiftRight: BTN_SELECT,
  ShiftLeft: BTN_SELECT,
};

// Map e.code strings to KEY_* indices (winit KeyCode enum order, shared with native)
const KEY_CODE_INDEX = {
  Backquote:0, Backslash:1, BracketLeft:2, BracketRight:3, Comma:4,
  Digit0:5, Digit1:6, Digit2:7, Digit3:8, Digit4:9,
  Digit5:10, Digit6:11, Digit7:12, Digit8:13, Digit9:14,
  Equal:15, IntlBackslash:16, IntlRo:17, IntlYen:18,
  KeyA:19, KeyB:20, KeyC:21, KeyD:22, KeyE:23, KeyF:24, KeyG:25,
  KeyH:26, KeyI:27, KeyJ:28, KeyK:29, KeyL:30, KeyM:31, KeyN:32,
  KeyO:33, KeyP:34, KeyQ:35, KeyR:36, KeyS:37, KeyT:38, KeyU:39,
  KeyV:40, KeyW:41, KeyX:42, KeyY:43, KeyZ:44,
  Minus:45, Period:46, Quote:47, Semicolon:48, Slash:49,
  AltLeft:50, AltRight:51, Backspace:52, CapsLock:53, ContextMenu:54,
  ControlLeft:55, ControlRight:56, Enter:57, SuperLeft:58, SuperRight:59,
  ShiftLeft:60, ShiftRight:61, Space:62, Tab:63,
  Convert:64, KanaMode:65, Lang1:66, Lang2:67, Lang3:68, Lang4:69, Lang5:70,
  NonConvert:71, Delete:72, End:73, Help:74, Home:75, Insert:76,
  PageDown:77, PageUp:78, ArrowDown:79, ArrowLeft:80, ArrowRight:81, ArrowUp:82,
  NumLock:83, Numpad0:84, Numpad1:85, Numpad2:86, Numpad3:87, Numpad4:88,
  Numpad5:89, Numpad6:90, Numpad7:91, Numpad8:92, Numpad9:93,
  NumpadAdd:94, NumpadBackspace:95, NumpadClear:96, NumpadClearEntry:97,
  NumpadComma:98, NumpadDecimal:99, NumpadDivide:100, NumpadEnter:101,
  NumpadEqual:102, NumpadHash:103, NumpadMemoryAdd:104, NumpadMemoryClear:105,
  NumpadMemoryRecall:106, NumpadMemoryStore:107, NumpadMemorySubtract:108,
  NumpadMultiply:109, NumpadParenLeft:110, NumpadParenRight:111,
  NumpadStar:112, NumpadSubtract:113,
  Escape:114, Fn:115, FnLock:116, PrintScreen:117, ScrollLock:118, Pause:119,
  BrowserBack:120, BrowserFavorites:121, BrowserForward:122, BrowserHome:123,
  BrowserRefresh:124, BrowserSearch:125, BrowserStop:126, Eject:127,
  LaunchApp1:128, LaunchApp2:129, LaunchMail:130, MediaPlayPause:131,
  MediaSelect:132, MediaStop:133, MediaTrackNext:134, MediaTrackPrevious:135,
  Power:136, Sleep:137, AudioVolumeDown:138, AudioVolumeMute:139, AudioVolumeUp:140,
  WakeUp:141, Meta:142, Hyper:143, Turbo:144, Abort:145, Resume:146, Suspend:147,
  Again:148, Copy:149, Cut:150, Find:151, Open:152, Paste:153, Props:154,
  Select:155, Undo:156, Hiragana:157, Katakana:158,
  F1:159, F2:160, F3:161, F4:162, F5:163, F6:164, F7:165, F8:166, F9:167,
  F10:168, F11:169, F12:170, F13:171, F14:172, F15:173, F16:174, F17:175,
  F18:176, F19:177, F20:178, F21:179, F22:180, F23:181, F24:182, F25:183,
  F26:184, F27:185, F28:186, F29:187, F30:188, F31:189, F32:190, F33:191,
  F34:192, F35:193,
};

class WGSLGameEngine {
  constructor(canvas, handleError = console.error) {
    this.canvas = canvas;
    this.handleError = handleError;

    this.buttons = new Int32Array(12);
    this.time = 0;
    this.lastTime = 0;
    this.deltaTime = 0;

    // Mouse state (iMouse-style): xy=pos, zw=last click (neg if not pressed)
    this.mouseX = 0;
    this.mouseY = 0;
    this.mouseClickX = 0;
    this.mouseClickY = 0;

    // Raw key state indexed by KEY_* constants (winit KeyCode enum order)
    this.keys = new Uint32Array(194);

    this.sounds = [];
    this.audioContext = null;
  }

  async preprocessShader(source, importedFiles = new Set(), isTopLevel = true) {
    // Process @import directives first (recursive, like C #include)
    const importMatches = [...source.matchAll(/@import\("([^"]+)"\)/g)];
    for (const match of importMatches) {
      const filename = match[1];

      // Skip if already imported (each file is only included once)
      if (importedFiles.has(filename)) {
        source = source.replace(match[0], `// Already imported: ${filename}`);
        continue;
      }

      // Mark as imported
      importedFiles.add(filename);

      // Read and process the imported file (same importedFiles set, not top level)
      const importedCode = await this.readFileText(filename);
      const processedImport = await this.preprocessShader(
        importedCode,
        importedFiles,
        false, // Not top level - don't add header
      );

      // Replace @import with the processed code (just the code, not metadata)
      source = source.replace(
        match[0],
        `// Imported from ${filename}\n${processedImport.code}\n`,
      );
    }

    // Extract metadata
    const metadata = {
      title: "WGSL Game",
      width: 800,
      height: 600,
      sounds: [],
      textures: [],
      videos: [],
      cameras: [],
      oscParams: [],
    };

    // Extract @set_title
    const titleMatch = source.match(/@set_title\("([^"]+)"\)/);
    if (titleMatch) metadata.title = titleMatch[1];

    // Extract @set_size
    const sizeMatch = source.match(/@set_size\((\d+),\s*(\d+)\)/);
    if (sizeMatch) {
      metadata.width = parseInt(sizeMatch[1]);
      metadata.height = parseInt(sizeMatch[2]);
    }

    // Find all @sound() references (both with and without .play()/.stop())
    const soundMatches = source.matchAll(
      /@sound\("([^"]+)"\)(?:\.(?:play|stop)\(\))?/g,
    );
    for (const match of soundMatches) {
      if (!metadata.sounds.includes(match[1])) {
        metadata.sounds.push(match[1]);
      }
    }

    // Find all @texture() references
    const textureMatches = source.matchAll(/@texture\("([^"]+)"\)/g);
    for (const match of textureMatches) {
      if (!metadata.textures.includes(match[1])) {
        metadata.textures.push(match[1]);
      }
    }

    // Find all @texture_index() references (also need to load these textures)
    const textureIndexMatches = source.matchAll(/@texture_index\("([^"]+)"\)/g);
    for (const match of textureIndexMatches) {
      if (!metadata.textures.includes(match[1])) {
        metadata.textures.push(match[1]);
      }
    }

    // Find all @video() references
    const videoMatches = source.matchAll(/@video\("([^"]+)"\)/g);
    for (const match of videoMatches) {
      if (!metadata.videos.includes(match[1])) {
        metadata.videos.push(match[1]);
      }
    }

    // Find all @camera() references
    const cameraMatches = source.matchAll(/@camera\((\d+)\)/g);
    for (const match of cameraMatches) {
      const idx = parseInt(match[1]);
      if (!metadata.cameras.includes(idx)) {
        metadata.cameras.push(idx);
      }
    }
    metadata.cameras.sort((a, b) => a - b);

    // Find all @osc() references
    const oscMatches = source.matchAll(/@osc\("([^"]+)"\)/g);
    for (const match of oscMatches) {
      if (!metadata.oscParams.includes(match[1])) {
        metadata.oscParams.push(match[1]);
      }
    }

    // Find all @model() references
    const modelMatches = source.matchAll(/@model\("([^"]+)"\)/g);
    metadata.models = [];
    for (const match of modelMatches) {
      if (!metadata.models.includes(match[1])) {
        metadata.models.push(match[1]);
      }
    }

    // Remove @set_* directives
    source = source.replace(/@set_title\([^)]+\)[^\n]*/g, "");
    source = source.replace(/@set_size\([^)]+\)[^\n]*/g, "");

    // Find GameState struct to inject before GameEngineHost
    const gameStateMatch = source.match(/struct GameState\s*{[^}]+}/s);
    const gameStateStruct = gameStateMatch ? gameStateMatch[0] : "";

    // Calculate GameState size for buffer allocation
    let stateSize = 0;
    let stateAlignment = 4; // Default alignment for scalars
    if (gameStateStruct) {
      // Match field types including arrays with angle brackets
      const fields =
        gameStateStruct.match(/:\s*(?:array<[^>]+>|[^,;\n]+)/g) || [];
      const arrayRegex = /array<([^,>]+),\s*(\d+)>/;

      for (const field of fields) {
        // Check if it's an array
        const arrayMatch = field.match(arrayRegex);
        if (arrayMatch) {
          const elementType = arrayMatch[1];
          const count = parseInt(arrayMatch[2]);

          let elementSize, elementAlign;
          if (elementType.includes("vec4f")) {
            elementSize = 16;
            elementAlign = 16;
          } else if (elementType.includes("vec3f")) {
            elementSize = 16; // vec3 aligns to 16 in arrays
            elementAlign = 16;
          } else if (elementType.includes("vec2f")) {
            elementSize = 8;
            elementAlign = 8;
          } else {
            // u32, i32, f32
            elementSize = 4;
            elementAlign = 4;
          }

          stateAlignment = Math.max(stateAlignment, elementAlign);
          stateSize += elementSize * count;
        } else {
          // Regular field
          if (field.includes("vec4f")) {
            stateSize += 16;
            stateAlignment = Math.max(stateAlignment, 16);
          } else if (field.includes("vec3f")) {
            stateSize += 12;
            stateAlignment = Math.max(stateAlignment, 16);
          } else if (field.includes("vec2f")) {
            stateSize += 8;
            stateAlignment = Math.max(stateAlignment, 8);
          } else if (
            field.includes("u32") ||
            field.includes("i32") ||
            field.includes("f32")
          ) {
            stateSize += 4;
            stateAlignment = Math.max(stateAlignment, 4);
          }
        }
      }
      // Align to struct's alignment (largest member)
      stateSize = Math.max(
        stateAlignment,
        Math.ceil(stateSize / stateAlignment) * stateAlignment,
      );
    } else {
      stateSize = 16; // Minimum size
    }
    metadata.stateSize = stateSize;

    // Build the header with structs and constants (only for top-level file)
    let header = "";
    if (isTopLevel) {
      header = `// Preprocessed WGSL - generated from macros\n\n`;

      // Add GameState first (if found)
      if (gameStateStruct) {
        header += `${gameStateStruct}\n\n`;
      }

      // Add GameEngineHost struct
      header += `// Engine host struct that contains all engine state\n`;
      header += `struct GameEngineHost {\n`;
      header += `    buttons: array<i32, 12>, // the current state of virtual SNES gamepad (BTN_*)\n`;
      header += `    time: f32, // clock time\n`;
      header += `    delta_time: f32, // time since last frame\n`;
      header += `    screen_width: f32, // current screensize\n`;
      header += `    screen_height: f32, // current screensize\n`;
      header += `    mouse: vec4f, // mouse state (iMouse): xy=pos, z=click_x (neg if not pressed), w=click_y\n`;
      if (gameStateStruct) {
        header += `    state: GameState, // user's game state that persists across frames\n`;
      }
      if (metadata.sounds.length > 0) {
        header += `    audio: array<u32, ${metadata.sounds.length}>, // audio trigger counters\n`;
      }
      header += `    osc: array<f32, 64>, // OSC float uniforms: /u/name or /u/N\n`;
      header += `    keys: array<u32, 194>, // raw key state: 1=down, 0=up, indexed by KEY_* constants\n`;
      header += `}\n\n`;

      // Add button constants
      header += `// Button constants for input\n`;
      header += `const BTN_UP: u32 = 0u;\n`;
      header += `const BTN_DOWN: u32 = 1u;\n`;
      header += `const BTN_LEFT: u32 = 2u;\n`;
      header += `const BTN_RIGHT: u32 = 3u;\n`;
      header += `const BTN_A: u32 = 4u;\n`;
      header += `const BTN_B: u32 = 5u;\n`;
      header += `const BTN_X: u32 = 6u;\n`;
      header += `const BTN_Y: u32 = 7u;\n`;
      header += `const BTN_L: u32 = 8u;\n`;
      header += `const BTN_R: u32 = 9u;\n`;
      header += `const BTN_START: u32 = 10u;\n`;
      header += `const BTN_SELECT: u32 = 11u;\n\n`;

      // Key constants — indices match winit KeyCode enum order / e.code strings
      header += `// Key constants for @engine.keys[] — same on native and web\n`;
      header += `const KEY_BACKQUOTE: u32 = 0u;\n`;
      header += `const KEY_BACKSLASH: u32 = 1u;\n`;
      header += `const KEY_BRACKET_LEFT: u32 = 2u;\n`;
      header += `const KEY_BRACKET_RIGHT: u32 = 3u;\n`;
      header += `const KEY_COMMA: u32 = 4u;\n`;
      header += `const KEY_0: u32 = 5u;\n`;
      header += `const KEY_1: u32 = 6u;\n`;
      header += `const KEY_2: u32 = 7u;\n`;
      header += `const KEY_3: u32 = 8u;\n`;
      header += `const KEY_4: u32 = 9u;\n`;
      header += `const KEY_5: u32 = 10u;\n`;
      header += `const KEY_6: u32 = 11u;\n`;
      header += `const KEY_7: u32 = 12u;\n`;
      header += `const KEY_8: u32 = 13u;\n`;
      header += `const KEY_9: u32 = 14u;\n`;
      header += `const KEY_EQUAL: u32 = 15u;\n`;
      header += `const KEY_INTL_BACKSLASH: u32 = 16u;\n`;
      header += `const KEY_INTL_RO: u32 = 17u;\n`;
      header += `const KEY_INTL_YEN: u32 = 18u;\n`;
      header += `const KEY_A: u32 = 19u;\n`;
      header += `const KEY_B: u32 = 20u;\n`;
      header += `const KEY_C: u32 = 21u;\n`;
      header += `const KEY_D: u32 = 22u;\n`;
      header += `const KEY_E: u32 = 23u;\n`;
      header += `const KEY_F: u32 = 24u;\n`;
      header += `const KEY_G: u32 = 25u;\n`;
      header += `const KEY_H: u32 = 26u;\n`;
      header += `const KEY_I: u32 = 27u;\n`;
      header += `const KEY_J: u32 = 28u;\n`;
      header += `const KEY_K: u32 = 29u;\n`;
      header += `const KEY_L: u32 = 30u;\n`;
      header += `const KEY_M: u32 = 31u;\n`;
      header += `const KEY_N: u32 = 32u;\n`;
      header += `const KEY_O: u32 = 33u;\n`;
      header += `const KEY_P: u32 = 34u;\n`;
      header += `const KEY_Q: u32 = 35u;\n`;
      header += `const KEY_R: u32 = 36u;\n`;
      header += `const KEY_S: u32 = 37u;\n`;
      header += `const KEY_T: u32 = 38u;\n`;
      header += `const KEY_U: u32 = 39u;\n`;
      header += `const KEY_V: u32 = 40u;\n`;
      header += `const KEY_W: u32 = 41u;\n`;
      header += `const KEY_X: u32 = 42u;\n`;
      header += `const KEY_Y: u32 = 43u;\n`;
      header += `const KEY_Z: u32 = 44u;\n`;
      header += `const KEY_MINUS: u32 = 45u;\n`;
      header += `const KEY_PERIOD: u32 = 46u;\n`;
      header += `const KEY_QUOTE: u32 = 47u;\n`;
      header += `const KEY_SEMICOLON: u32 = 48u;\n`;
      header += `const KEY_SLASH: u32 = 49u;\n`;
      header += `const KEY_ALT_LEFT: u32 = 50u;\n`;
      header += `const KEY_ALT_RIGHT: u32 = 51u;\n`;
      header += `const KEY_BACKSPACE: u32 = 52u;\n`;
      header += `const KEY_CAPS_LOCK: u32 = 53u;\n`;
      header += `const KEY_CONTEXT_MENU: u32 = 54u;\n`;
      header += `const KEY_CTRL_LEFT: u32 = 55u;\n`;
      header += `const KEY_CTRL_RIGHT: u32 = 56u;\n`;
      header += `const KEY_ENTER: u32 = 57u;\n`;
      header += `const KEY_SUPER_LEFT: u32 = 58u;\n`;
      header += `const KEY_SUPER_RIGHT: u32 = 59u;\n`;
      header += `const KEY_SHIFT_LEFT: u32 = 60u;\n`;
      header += `const KEY_SHIFT_RIGHT: u32 = 61u;\n`;
      header += `const KEY_SPACE: u32 = 62u;\n`;
      header += `const KEY_TAB: u32 = 63u;\n`;
      header += `const KEY_DELETE: u32 = 72u;\n`;
      header += `const KEY_END: u32 = 73u;\n`;
      header += `const KEY_HOME: u32 = 75u;\n`;
      header += `const KEY_INSERT: u32 = 76u;\n`;
      header += `const KEY_PAGE_DOWN: u32 = 77u;\n`;
      header += `const KEY_PAGE_UP: u32 = 78u;\n`;
      header += `const KEY_DOWN: u32 = 79u;\n`;
      header += `const KEY_LEFT: u32 = 80u;\n`;
      header += `const KEY_RIGHT: u32 = 81u;\n`;
      header += `const KEY_UP: u32 = 82u;\n`;
      header += `const KEY_ESCAPE: u32 = 114u;\n`;
      header += `const KEY_F1: u32 = 159u;\n`;
      header += `const KEY_F2: u32 = 160u;\n`;
      header += `const KEY_F3: u32 = 161u;\n`;
      header += `const KEY_F4: u32 = 162u;\n`;
      header += `const KEY_F5: u32 = 163u;\n`;
      header += `const KEY_F6: u32 = 164u;\n`;
      header += `const KEY_F7: u32 = 165u;\n`;
      header += `const KEY_F8: u32 = 166u;\n`;
      header += `const KEY_F9: u32 = 167u;\n`;
      header += `const KEY_F10: u32 = 168u;\n`;
      header += `const KEY_F11: u32 = 169u;\n`;
      header += `const KEY_F12: u32 = 170u;\n`;
      header += `\n`;

      // Add bindings
      header += `// Bindings: group 0 = textures, group 1 = engine state\n\n`;

      // Add sampler
      header += `@group(0) @binding(0) var _engine_sampler: sampler;\n`;

      // Add texture bindings
      metadata.textures.forEach((texName, i) => {
        header += `@group(0) @binding(${i + 1}) var _texture_${i}: texture_2d<f32>; // ${texName}\n`;
      });

      // Add video texture bindings
      const videoBase = metadata.textures.length + 1;
      metadata.videos.forEach((vidName, i) => {
        header += `@group(0) @binding(${videoBase + i}) var _video_${i}: texture_2d<f32>; // ${vidName}\n`;
      });

      // Add camera texture bindings
      const cameraBase = metadata.textures.length + metadata.videos.length + 1;
      metadata.cameras.forEach((camIdx, i) => {
        header += `@group(0) @binding(${cameraBase + i}) var _camera_${i}: texture_2d<f32>; // camera ${camIdx}\n`;
      });

      // Add engine buffer
      header += `\n@group(1) @binding(0) var<storage, read_write> _engine: GameEngineHost;\n`;

      // Add model buffers
      if (metadata.models && metadata.models.length > 0) {
        header += `\n// Model data buffers\n`;
        metadata.models.forEach((modelName, i) => {
          const bindingBase = 1 + i * 2;
          header += `struct Model${i}Positions { data: array<vec3f> }\n`;
          header += `@group(2) @binding(${bindingBase}) var<storage, read> _model_${i}_positions: Model${i}Positions; // ${modelName}\n`;
          header += `struct Model${i}Normals { data: array<vec3f> }\n`;
          header += `@group(2) @binding(${bindingBase + 1}) var<storage, read> _model_${i}_normals: Model${i}Normals;\n`;
        });
      }

      header += `\n`;

      // Remove GameState struct from source since we added it to header
      if (gameStateStruct) {
        source = source.replace(/struct GameState\s*{[^}]+}\s*/s, "");
      }
    }

    // Replace macros in source
    // Replace @engine.* with _engine.*
    source = source.replace(/@engine\.buttons/g, "_engine.buttons");
    source = source.replace(/@engine\.time/g, "_engine.time");
    source = source.replace(/@engine\.delta_time/g, "_engine.delta_time");
    source = source.replace(/@engine\.screen_width/g, "_engine.screen_width");
    source = source.replace(/@engine\.screen_height/g, "_engine.screen_height");
    source = source.replace(/@engine\.mouse/g, "_engine.mouse");
    source = source.replace(/@engine\.keys/g, "_engine.keys");
    source = source.replace(/@engine\.sampler/g, "_engine_sampler");
    source = source.replace(/@engine\.state/g, "_engine.state");

    // Replace @sound().play() and @sound().stop() with audio trigger operations
    metadata.sounds.forEach((soundName, i) => {
      const escapedName = soundName.replace(/\./g, "\\.");
      // Replace @sound("file").play() with _engine.audio[index]++ (triggers playback)
      const playRegex = new RegExp(
        `@sound\\("${escapedName}"\\)\\.play\\(\\)`,
        "g",
      );
      source = source.replace(playRegex, `_engine.audio[${i}]++`);

      // Replace @sound("file").stop() with no-op for now (could implement later)
      const stopRegex = new RegExp(
        `@sound\\("${escapedName}"\\)\\.stop\\(\\)`,
        "g",
      );
      source = source.replace(
        stopRegex,
        `/* stop sound ${i} - not implemented */`,
      );

      // Also support legacy @sound("file")++ syntax
      const legacyRegex = new RegExp(`@sound\\("${escapedName}"\\)`, "g");
      source = source.replace(legacyRegex, `_engine.audio[${i}]`);
    });

    // Replace @texture() with _texture_index
    metadata.textures.forEach((texName, i) => {
      const regex = new RegExp(
        `@texture\\("${texName.replace(".", "\\.")}"\\)`,
        "g",
      );
      source = source.replace(regex, `_texture_${i}`);
    });

    // Replace @texture_index() with the texture binding number (as u32)
    metadata.textures.forEach((texName, i) => {
      const regex = new RegExp(
        `@texture_index\\("${texName.replace(".", "\\.")}"\\)`,
        "g",
      );
      source = source.replace(regex, `${i}u`);
    });

    // Replace @video()
    metadata.videos.forEach((vidName, i) => {
      const escaped = vidName.replace(/\./g, "\\.");
      const regex = new RegExp(`@video\\("${escaped}"\\)`, "g");
      source = source.replace(regex, `_video_${i}`);
    });

    // Replace @camera()
    metadata.cameras.forEach((camIdx, i) => {
      const regex = new RegExp(`@camera\\(${camIdx}\\)`, "g");
      source = source.replace(regex, `_camera_${i}`);
    });

    // Replace @osc()
    metadata.oscParams.forEach((oscName, i) => {
      const escaped = oscName.replace(/\./g, "\\.");
      const regex = new RegExp(`@osc\\("${escaped}"\\)`, "g");
      source = source.replace(regex, `_engine.osc[${i}]`);
    });

    // Replace @model() references
    if (metadata.models) {
      metadata.models.forEach((modelName, i) => {
        const escapedName = modelName.replace(/\./g, "\\.");
        // Replace .positions with buffer access
        const posRegex = new RegExp(
          `@model\\("${escapedName}"\\)\\.positions`,
          "g",
        );
        source = source.replace(posRegex, `_model_${i}_positions.data`);
        // Replace .normals with buffer access
        const normRegex = new RegExp(
          `@model\\("${escapedName}"\\)\\.normals`,
          "g",
        );
        source = source.replace(normRegex, `_model_${i}_normals.data`);
        // Replace any remaining @model references with comment
        const modelRegex = new RegExp(`@model\\("${escapedName}"\\)`, "g");
        source = source.replace(
          modelRegex,
          `/* @model("${modelName}") - use .positions or .normals */`,
        );
      });
    }

    // Replace @str() with fixed-size array of character codes (padded with zeros)
    // We use a fixed size of 128 to handle most strings
    const MAX_STRING_LENGTH = 128;
    // Match @str("...") including escaped characters
    const strMatches = [...source.matchAll(/@str\("((?:[^"\\]|\\.)*)"\)/g)];
    for (const match of strMatches) {
      // Unescape the string properly
      const str = match[1]
        .replace(/\\n/g, '\n')
        .replace(/\\r/g, '\r')
        .replace(/\\t/g, '\t')
        .replace(/\\"/g, '"')
        .replace(/\\\\/g, '\\');

      const charCodes = Array.from(str).map(c => c.charCodeAt(0));
      // Pad with zeros up to MAX_STRING_LENGTH
      while (charCodes.length < MAX_STRING_LENGTH) {
        charCodes.push(0);
      }
      const replacement = `array<u32, ${MAX_STRING_LENGTH}>(${charCodes.map(c => `${c}u`).join(', ')})`;
      source = source.replace(match[0], replacement);
    }

    return {
      code: header + source,
      metadata,
    };
  }

  showError(msg) {
    console.error(msg);
    this.handleError(msg);
  }

  async init(gamePath) {
    try {
      // Initialize WebGPU
      if (!navigator.gpu) {
        throw new Error("WebGPU not supported in this browser");
      }

      this.adapter = await navigator.gpu.requestAdapter();
      if (!this.adapter) {
        throw new Error("Failed to get GPU adapter");
      }

      this.device = await this.adapter.requestDevice();
      this.device.lost.then((info) => {
        this.showError(`WebGPU device lost: ${info.message}`);
      });

      this.context = this.canvas.getContext("webgpu");
      this.presentationFormat = navigator.gpu.getPreferredCanvasFormat();

      this.context.configure({
        device: this.device,
        format: this.presentationFormat,
        alphaMode: "opaque",
      });

      // Load game files
      await this.loadGame(gamePath);

      // Setup input
      this.setupInput();

      // Initialize audio context on first user interaction
      const initAudio = () => this.initAudio();
      window.addEventListener("click", initAudio, { once: true });
      window.addEventListener("keydown", initAudio, { once: true });

      // Start game loop
      this.running = true;
      this.lastTime = performance.now();
      this.gameLoop();

      // Return game metadata
      return {
        title: this.gameTitle,
        width: this.canvas.width,
        height: this.canvas.height,
        sounds: this.soundFiles,
        textures: this.textureFiles,
      };
    } catch (err) {
      this.showError(err.stack || err.message);
      throw err;
    }
  }

  async loadGame(gamePath) {
    // Load main.wgsl with macros
    const shaderResponse = await fetch(gamePath);

    if (!shaderResponse.ok) {
      throw new Error(`Failed to load main.wgsl: ${shaderResponse.statusText}`);
    }

    const entryBytes = new Uint8Array(await shaderResponse.arrayBuffer());

    const doZip = entryBytes[0] === 0x50 && entryBytes[1] === 0x4b;

    // use this to read a file from the current "filesystem" (URL or zip)
    if (doZip) {
      this.readFile = async (name) => extractSingleFile(entryBytes, name);
    } else {
      const urlBase = gamePath.replace(/\/?main\.wgsl$/, "") || ".";
      this.readFile = async (name) =>
        new Uint8Array(
          await fetch(`${urlBase}/${name}`).then((r) => r.arrayBuffer()),
        );
    }
    this.readFileText = async (name) =>
      decoder.decode(await this.readFile(name));

    // reuse entryBytes, if it's the entrypoint wgsl
    const shaderCode = doZip
      ? await this.readFileText("main.wgsl")
      : decoder.decode(entryBytes);

    // Preprocess macros
    const result = await this.preprocessShader(shaderCode);
    const processedCode = result.code;
    const metadata = result.metadata;

    // output processed shader
    console.log(result.code);

    // Set metadata
    this.gameTitle = metadata.title;
    this.canvas.width = metadata.width;
    this.canvas.height = metadata.height;

    // Create depth texture for 3D rendering
    this.depthTexture = this.device.createTexture({
      size: [this.canvas.width, this.canvas.height],
      format: "depth24plus",
      usage: GPUTextureUsage.RENDER_ATTACHMENT,
    });
    this.depthView = this.depthTexture.createView();

    this.soundFiles = metadata.sounds;
    this.textureFiles = metadata.textures;
    this.audioCount = metadata.sounds.length;
    this.textureCount = metadata.textures.length;
    this.stateSize = metadata.stateSize;

    // Store model files
    this.modelFiles = metadata.models || [];

    // Store video/camera metadata
    this.videoFiles = metadata.videos || [];
    this.cameraIndices = metadata.cameras || [];
    this.oscParams = metadata.oscParams || [];
    this.oscValues = new Float32Array(64); // 64 OSC float slots

    // Load textures
    await this.loadTextures();

    // Load videos
    await this.loadVideos();

    // Load cameras
    await this.loadCameras();

    // Load models
    await this.loadModels();

    // Load sounds
    await this.loadSounds();

    // Create shader module
    this.shaderModule = this.device.createShaderModule({
      code: processedCode,
      label: "game-shader",
    });

    // Check for compilation errors
    const compilationInfo = await this.shaderModule.getCompilationInfo();
    if (compilationInfo.messages.length > 0) {
      const errors = compilationInfo.messages
        .filter((m) => m.type === "error")
        .map((m) => `${m.lineNum}:${m.linePos} - ${m.message}`)
        .join("\n");
      if (errors) {
        throw new Error(`Shader compilation errors:\n${errors}`);
      }
    }

    // Create buffers first (before pipelines so we can create explicit layouts)
    this.setupBuffers();

    // Create explicit bind group layouts
    // Group 0: sampler (always) and textures (if present)
    const renderGroup0Entries = [
      // Sampler (always present since preprocessor always adds it)
      {
        binding: 0,
        visibility: GPUShaderStage.FRAGMENT,
        sampler: { type: "filtering" },
      },
    ];

    // Add texture bindings if present
    for (let i = 0; i < this.textureFiles.length; i++) {
      renderGroup0Entries.push({
        binding: i + 1,
        visibility: GPUShaderStage.FRAGMENT,
        texture: { sampleType: "float", viewDimension: "2d" },
      });
    }

    // Add video texture bindings
    const jsVideoBase = this.textureFiles.length + 1;
    for (let i = 0; i < this.videoFiles.length; i++) {
      renderGroup0Entries.push({
        binding: jsVideoBase + i,
        visibility: GPUShaderStage.FRAGMENT,
        texture: { sampleType: "float", viewDimension: "2d" },
      });
    }

    // Add camera texture bindings
    const jsCameraBase = this.textureFiles.length + this.videoFiles.length + 1;
    for (let i = 0; i < this.cameraIndices.length; i++) {
      renderGroup0Entries.push({
        binding: jsCameraBase + i,
        visibility: GPUShaderStage.FRAGMENT,
        texture: { sampleType: "float", viewDimension: "2d" },
      });
    }

    this.renderBindGroupLayout0 = this.device.createBindGroupLayout({
      label: "Render Bind Group Layout 0",
      entries: renderGroup0Entries,
    });

    // Group 1: engine buffer (storage for fragment only)
    this.renderBindGroupLayout1 = this.device.createBindGroupLayout({
      label: "Render Bind Group Layout 1",
      entries: [
        {
          binding: 0,
          visibility: GPUShaderStage.FRAGMENT,
          buffer: { type: "storage" },
        },
      ],
    });

    // Group 2: model buffers (if models exist)
    const modelGroup2Entries = [];
    if (this.models && this.models.length > 0) {
      this.models.forEach((model, i) => {
        const bindingBase = 1 + i * 2;
        // Positions buffer
        modelGroup2Entries.push({
          binding: bindingBase,
          visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
          buffer: { type: "read-only-storage" },
        });
        // Normals buffer
        modelGroup2Entries.push({
          binding: bindingBase + 1,
          visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
          buffer: { type: "read-only-storage" },
        });
      });
    }

    this.renderBindGroupLayout2 =
      modelGroup2Entries.length > 0
        ? this.device.createBindGroupLayout({
            label: "Render Bind Group Layout 2",
            entries: modelGroup2Entries,
          })
        : null;

    // Compute bind group layout for engine buffer (read-write)
    this.computeBindGroupLayout1 = this.device.createBindGroupLayout({
      label: "Compute Bind Group Layout 1",
      entries: [
        {
          binding: 0,
          visibility: GPUShaderStage.COMPUTE,
          buffer: { type: "storage" },
        },
      ],
    });

    // Create pipeline layouts
    const renderBindGroupLayouts = [
      this.renderBindGroupLayout0,
      this.renderBindGroupLayout1,
    ];
    if (this.renderBindGroupLayout2) {
      renderBindGroupLayouts.push(this.renderBindGroupLayout2);
    }

    const renderPipelineLayout = this.device.createPipelineLayout({
      label: "Render Pipeline Layout",
      bindGroupLayouts: renderBindGroupLayouts,
    });

    const computePipelineLayout = this.device.createPipelineLayout({
      label: "Compute Pipeline Layout",
      bindGroupLayouts: [
        this.renderBindGroupLayout0,
        this.computeBindGroupLayout1,
      ],
    });

    // Setup render pipeline with explicit layout
    this.renderPipeline = this.device.createRenderPipeline({
      layout: renderPipelineLayout,
      vertex: {
        module: this.shaderModule,
        entryPoint: "vs_main",
      },
      fragment: {
        module: this.shaderModule,
        entryPoint: "fs_render",
        targets: [
          {
            format: this.presentationFormat,
          },
        ],
      },
      primitive: {
        topology: "triangle-list",
      },
      depthStencil: {
        format: "depth24plus",
        depthWriteEnabled: true,
        depthCompare: "less",
      },
    });

    // Setup compute pipeline with explicit layout
    this.updatePipeline = this.device.createComputePipeline({
      layout: computePipelineLayout,
      compute: {
        module: this.shaderModule,
        entryPoint: "update",
      },
    });

    // Create bind groups using layouts from the pipelines
    this.setupBindGroups();
  }

  async loadTextures() {
    this.textures = [];

    for (const filename of this.textureFiles) {
      // Use readFile to support both zip and directory
      const imageData = await this.readFile(filename);

      // Create a blob and object URL from the image data
      const blob = new Blob([imageData], { type: "image/png" });
      const url = URL.createObjectURL(blob);

      const img = new Image();
      img.src = url;
      await img.decode();

      const imageBitmap = await createImageBitmap(img);

      // Clean up the object URL
      URL.revokeObjectURL(url);

      const texture = this.device.createTexture({
        size: [imageBitmap.width, imageBitmap.height, 1],
        format: "rgba8unorm",
        usage:
          GPUTextureUsage.TEXTURE_BINDING |
          GPUTextureUsage.COPY_DST |
          GPUTextureUsage.RENDER_ATTACHMENT,
      });

      this.device.queue.copyExternalImageToTexture(
        { source: imageBitmap },
        { texture },
        [imageBitmap.width, imageBitmap.height],
      );

      this.textures.push(texture);
    }

    // Create sampler
    this.sampler = this.device.createSampler({
      magFilter: "nearest",
      minFilter: "nearest",
      mipmapFilter: "nearest",
      addressModeU: "clamp-to-edge",
      addressModeV: "clamp-to-edge",
    });
  }

  async loadVideos() {
    this.videoElements = [];
    this.videoTextures = [];

    for (const filename of this.videoFiles) {
      const imageData = await this.readFile(filename);
      const ext = filename.split(".").pop().toLowerCase();
      const isGif = ext === "gif";

      const mimeType = isGif ? "image/gif" : `video/${ext}`;
      const blob = new Blob([imageData], { type: mimeType });
      const url = URL.createObjectURL(blob);

      let width = 1, height = 1, element;

      if (isGif) {
        element = new Image();
        element.src = url;
        await new Promise((resolve) => {
          element.onload = resolve;
          element.onerror = resolve;
        });
        width = element.naturalWidth || 1;
        height = element.naturalHeight || 1;
      } else {
        element = document.createElement("video");
        element.autoplay = true;
        element.loop = true;
        element.muted = true;
        element.playsInline = true;
        element.src = url;
        await new Promise((resolve) => {
          element.onloadedmetadata = resolve;
          element.onerror = resolve;
        });
        width = element.videoWidth || 1;
        height = element.videoHeight || 1;
        element.play().catch(() => {});
      }

      // Offscreen canvas for GIF (needed for copyExternalImageToTexture)
      let canvas = null, ctx = null;
      if (isGif) {
        canvas = new OffscreenCanvas(width, height);
        ctx = canvas.getContext("2d");
      }

      const texture = this.device.createTexture({
        size: [width, height, 1],
        format: "rgba8unorm",
        usage:
          GPUTextureUsage.TEXTURE_BINDING |
          GPUTextureUsage.COPY_DST |
          GPUTextureUsage.RENDER_ATTACHMENT,
      });

      this.videoElements.push({ element, canvas, ctx, texture, width, height, isGif });
      this.videoTextures.push(texture);
    }
  }

  async loadCameras() {
    this.cameraVideoElements = [];
    this.cameraTextures = [];

    for (const camIdx of this.cameraIndices) {
      try {
        const devices = await navigator.mediaDevices.enumerateDevices();
        const videoDevices = devices.filter((d) => d.kind === "videoinput");
        const deviceId = videoDevices[camIdx]?.deviceId;
        if (!deviceId) throw new Error(`Camera index ${camIdx} not found`);

        const stream = await navigator.mediaDevices.getUserMedia({
          video: { deviceId: { exact: deviceId } },
        });

        const video = document.createElement("video");
        video.srcObject = stream;
        video.playsInline = true;
        video.muted = true;
        await new Promise((resolve) => {
          video.onloadedmetadata = resolve;
        });
        video.play();

        const width = video.videoWidth || 640;
        const height = video.videoHeight || 480;

        const texture = this.device.createTexture({
          size: [width, height, 1],
          format: "rgba8unorm",
          usage:
            GPUTextureUsage.TEXTURE_BINDING |
            GPUTextureUsage.COPY_DST |
            GPUTextureUsage.RENDER_ATTACHMENT,
        });

        this.cameraVideoElements.push({ element: video, texture, width, height });
        this.cameraTextures.push(texture);
      } catch (err) {
        console.warn(`[camera] Failed to open camera ${camIdx}:`, err);
        // Fallback: 1x1 black texture
        const texture = this.device.createTexture({
          size: [1, 1, 1],
          format: "rgba8unorm",
          usage:
            GPUTextureUsage.TEXTURE_BINDING |
            GPUTextureUsage.COPY_DST |
            GPUTextureUsage.RENDER_ATTACHMENT,
        });
        this.cameraVideoElements.push({ element: null, texture, width: 1, height: 1 });
        this.cameraTextures.push(texture);
      }
    }
  }

  async loadModels() {
    this.models = [];
    this.modelVertexCount = 0;

    for (const filename of this.modelFiles) {
      // Load OBJ file
      const objText = await this.readFileText(filename);
      const model = parseOBJ(objText);

      console.log(
        `Loaded model: ${filename} (${model.vertexCount} vertices, ${model.vertexCount / 3} triangles)`,
      );

      this.modelVertexCount = model.vertexCount;

      // Create positions buffer
      // IMPORTANT: array<vec3f> in WGSL storage buffers has 16-byte alignment (like vec4)
      // So we need to pad each vec3 to 4 floats
      const positionsData = new Float32Array(
        model.positions.flatMap((p) => [p[0], p[1], p[2], 0.0]),
      );
      const positionsBuffer = this.device.createBuffer({
        size: positionsData.byteLength,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        mappedAtCreation: true,
      });
      new Float32Array(positionsBuffer.getMappedRange()).set(positionsData);
      positionsBuffer.unmap();

      // Create normals buffer
      // Same padding required for normals
      const normalsData = new Float32Array(
        model.normals.flatMap((n) => [n[0], n[1], n[2], 0.0]),
      );
      const normalsBuffer = this.device.createBuffer({
        size: normalsData.byteLength,
        usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
        mappedAtCreation: true,
      });
      new Float32Array(normalsBuffer.getMappedRange()).set(normalsData);
      normalsBuffer.unmap();

      this.models.push({ positionsBuffer, normalsBuffer });
    }
  }

  async loadSounds() {
    this.soundBuffers = [];

    for (const filename of this.soundFiles) {
      try {
        // Use readFile to support both zip and directory
        const soundData = await this.readFile(filename);
        // Convert Uint8Array to ArrayBuffer
        this.soundBuffers.push(soundData.buffer);
      } catch (err) {
        console.warn(`Failed to load sound ${filename}:`, err);
        this.soundBuffers.push(null);
      }
    }
  }

  async initAudio() {
    if (this.audioContext) return;
    this.audioContext = new AudioContext();

    // Decode all sound buffers
    const decodePromises = this.soundBuffers.map(async (buffer, i) => {
      if (!buffer) return null;
      try {
        return await this.audioContext.decodeAudioData(buffer.slice(0));
      } catch (err) {
        console.warn(`Failed to decode sound ${i}:`, err);
        return null;
      }
    });

    this.sounds = await Promise.all(decodePromises);
  }

  playSound(index) {
    if (!this.audioContext || !this.sounds[index]) {
      return;
    }

    const source = this.audioContext.createBufferSource();
    source.buffer = this.sounds[index];
    source.connect(this.audioContext.destination);
    source.start();
  }

  setupBuffers() {
    // Calculate buffer sizes matching WGSL struct layout rules for storage buffers
    // GameEngineHost layout:
    //   buttons: array<i32, 12> at offset 0 (48 bytes)
    //   time: f32 at offset 48 (4 bytes)
    //   delta_time: f32 at offset 52 (4 bytes)
    //   screen_width: f32 at offset 56 (4 bytes)
    //   screen_height: f32 at offset 60 (4 bytes)
    //   mouse: vec4f at offset 64 (16 bytes, 16-byte aligned)
    //   state: GameState at offset 80 (aligned to 8 bytes for vec2f)
    //   audio: array<u32, N> at offset 80 + stateSize

    const buttonSize = 12 * 4; // 48 bytes
    const floatDataSize = 8 * 4; // 32 bytes (time, delta, width, height + mouse xyzw)

    // stateSize is already aligned to its struct's alignment by the preprocessor
    const alignedStateSize = this.stateSize;

    const audioSize = this.audioCount * 4;
    const oscSize = 64 * 4; // 256 bytes for osc array (64 f32s)
    const keysSize = 194 * 4; // 776 bytes for keys array (194 u32s)

    // Total size must be multiple of 16 for storage buffer
    const totalSizeUnaligned =
      buttonSize + floatDataSize + alignedStateSize + audioSize + oscSize + keysSize;
    const totalSize = Math.ceil(totalSizeUnaligned / 16) * 16;

    // Create storage buffer for engine state (writable from compute shader)
    this.engineBuffer = this.device.createBuffer({
      size: totalSize,
      usage:
        GPUBufferUsage.STORAGE |
        GPUBufferUsage.COPY_DST |
        GPUBufferUsage.COPY_SRC,
    });

    // Create staging buffer for CPU reads
    this.stagingBuffer = this.device.createBuffer({
      size: totalSize,
      usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
    });

    const oscOffset = buttonSize + floatDataSize + alignedStateSize + audioSize;
    this.bufferOffsets = {
      buttons: 0,
      floats: buttonSize, // 48
      state: buttonSize + floatDataSize, // 80 (8-byte aligned)
      audio: buttonSize + floatDataSize + alignedStateSize,
      osc: oscOffset,
      keys: oscOffset + 64 * 4, // after osc (256 bytes for 64 f32s)
    };

    // Initialize game state to zero
    const initData = new ArrayBuffer(totalSize);
    const initView = new DataView(initData);

    // Set initial screen size
    initView.setFloat32(this.bufferOffsets.floats + 8, this.canvas.width, true);
    initView.setFloat32(
      this.bufferOffsets.floats + 12,
      this.canvas.height,
      true,
    );

    // Set initial player position (center of screen)
    initView.setFloat32(
      this.bufferOffsets.state + 0,
      this.canvas.width / 2,
      true,
    ); // player_pos.x
    initView.setFloat32(
      this.bufferOffsets.state + 4,
      this.canvas.height / 2,
      true,
    ); // player_pos.y

    this.device.queue.writeBuffer(this.engineBuffer, 0, initData);
  }

  setupBindGroups() {
    // Group 0: Sampler (always) and textures (if present)
    const group0Entries = [
      // Sampler (always present since preprocessor always adds it)
      {
        binding: 0,
        resource: this.sampler,
      },
    ];

    // Add texture bindings if present
    this.textures.forEach((texture, i) => {
      group0Entries.push({
        binding: i + 1,
        resource: texture.createView(),
      });
    });

    // Add video texture views
    const bgVideoBase = this.textureFiles.length + 1;
    (this.videoTextures || []).forEach((texture, i) => {
      group0Entries.push({
        binding: bgVideoBase + i,
        resource: texture.createView(),
      });
    });

    // Add camera texture views
    const bgCameraBase = this.textureFiles.length + (this.videoFiles || []).length + 1;
    (this.cameraTextures || []).forEach((texture, i) => {
      group0Entries.push({
        binding: bgCameraBase + i,
        resource: texture.createView(),
      });
    });

    // Create bind groups for render pipeline using explicit layouts
    this.renderBindGroup0 = this.device.createBindGroup({
      layout: this.renderBindGroupLayout0,
      entries: group0Entries,
    });

    this.renderBindGroup1 = this.device.createBindGroup({
      layout: this.renderBindGroupLayout1,
      entries: [
        {
          binding: 0,
          resource: {
            buffer: this.engineBuffer,
          },
        },
      ],
    });

    // Create model bind group if models exist
    if (this.renderBindGroupLayout2 && this.models && this.models.length > 0) {
      const modelEntries = [];
      this.models.forEach((model, i) => {
        const bindingBase = 1 + i * 2;
        modelEntries.push({
          binding: bindingBase,
          resource: { buffer: model.positionsBuffer },
        });
        modelEntries.push({
          binding: bindingBase + 1,
          resource: { buffer: model.normalsBuffer },
        });
      });

      this.renderBindGroup2 = this.device.createBindGroup({
        layout: this.renderBindGroupLayout2,
        entries: modelEntries,
      });
    }

    // Create bind group for compute pipeline using explicit layout
    this.computeBindGroup1 = this.device.createBindGroup({
      layout: this.computeBindGroupLayout1,
      entries: [
        {
          binding: 0,
          resource: {
            buffer: this.engineBuffer,
          },
        },
      ],
    });
  }

  setupInput() {
    // Make canvas focusable and auto-focus it
    this.canvas.tabIndex = 1000;
    this.canvas.focus();

    // Listen on window for broader capture
    window.addEventListener("keydown", (e) => {
      const btn = KEY_MAP[e.code];
      if (btn !== undefined) {
        this.buttons[btn] = 1;
        e.preventDefault();
      }
      // Track raw key state via canonical index
      const ki = KEY_CODE_INDEX[e.code];
      if (ki !== undefined) this.keys[ki] = 1;
    });

    window.addEventListener("keyup", (e) => {
      const btn = KEY_MAP[e.code];
      if (btn !== undefined) {
        this.buttons[btn] = 0;
        e.preventDefault();
      }
      // Track raw key state via canonical index
      const ki = KEY_CODE_INDEX[e.code];
      if (ki !== undefined) this.keys[ki] = 0;
    });

    // Refocus canvas on click
    this.canvas.addEventListener("click", () => this.canvas.focus());

    // Mouse tracking (iMouse-style: xy=current pos, zw=click pos)
    this.canvas.addEventListener("mousemove", (e) => {
      const rect = this.canvas.getBoundingClientRect();
      const scaleX = this.canvas.width / rect.width;
      const scaleY = this.canvas.height / rect.height;
      this.mouseX = (e.clientX - rect.left) * scaleX;
      this.mouseY = (e.clientY - rect.top) * scaleY;
    });

    this.canvas.addEventListener("mousedown", (e) => {
      if (e.button === 0) {
        const rect = this.canvas.getBoundingClientRect();
        const scaleX = this.canvas.width / rect.width;
        const scaleY = this.canvas.height / rect.height;
        this.mouseX = (e.clientX - rect.left) * scaleX;
        this.mouseY = (e.clientY - rect.top) * scaleY;
        this.mouseClickX = this.mouseX;
        this.mouseClickY = this.mouseY;
        e.preventDefault();
      }
    });

    this.canvas.addEventListener("mouseup", (e) => {
      if (e.button === 0) {
        this.mouseClickX = -Math.abs(this.mouseClickX);
        this.mouseClickY = -Math.abs(this.mouseClickY);
      }
    });
  }

  gameLoop() {
    if (!this.running) return;

    const now = performance.now();
    this.deltaTime = (now - this.lastTime) / 1000;
    this.time += this.deltaTime;
    this.lastTime = now;

    this.update();
    this.render();

    requestAnimationFrame(() => this.gameLoop());
  }

  updateDynamicTextures() {
    // Update video textures
    for (const { element, canvas, ctx, texture, width, height, isGif } of (this.videoElements || [])) {
      if (isGif && ctx) {
        ctx.clearRect(0, 0, width, height);
        ctx.drawImage(element, 0, 0, width, height);
        this.device.queue.copyExternalImageToTexture(
          { source: canvas },
          { texture },
          [width, height],
        );
      } else if (!isGif && element && element.readyState >= 2) {
        this.device.queue.copyExternalImageToTexture(
          { source: element },
          { texture },
          [width, height],
        );
      }
    }

    // Update camera textures
    for (const { element, texture, width, height } of (this.cameraVideoElements || [])) {
      if (element && element.readyState >= 2) {
        this.device.queue.copyExternalImageToTexture(
          { source: element },
          { texture },
          [width, height],
        );
      }
    }
  }

  update() {
    // Update dynamic textures (video/camera)
    this.updateDynamicTextures();

    // Write volatile input state to buffer (buttons + floats with mouse)
    const inputData = new ArrayBuffer(48 + 32); // buttons + floats (time, delta, w, h, mouse xyzw)
    const inputView = new DataView(inputData);

    // Write buttons
    for (let i = 0; i < 12; i++) {
      inputView.setInt32(i * 4, this.buttons[i], true);
    }

    // Write time data
    inputView.setFloat32(48, this.time, true);
    inputView.setFloat32(52, this.deltaTime, true);
    inputView.setFloat32(56, this.canvas.width, true);
    inputView.setFloat32(60, this.canvas.height, true);

    // Write mouse data (iMouse-style, at offset 64 within float section)
    inputView.setFloat32(64, this.mouseX, true);
    inputView.setFloat32(68, this.mouseY, true);
    inputView.setFloat32(72, this.mouseClickX, true);
    inputView.setFloat32(76, this.mouseClickY, true);

    this.device.queue.writeBuffer(this.engineBuffer, 0, inputData);

    // Write OSC values at their correct offset (after state + audio sections)
    const oscData = new Float32Array(64);
    for (let i = 0; i < 64; i++) {
      oscData[i] = this.oscValues[i];
    }
    this.device.queue.writeBuffer(this.engineBuffer, this.bufferOffsets.osc, oscData.buffer);

    // Write raw key state at its offset (after osc)
    this.device.queue.writeBuffer(this.engineBuffer, this.bufferOffsets.keys, this.keys.buffer);

    // Run compute shader
    const commandEncoder = this.device.createCommandEncoder();
    const computePass = commandEncoder.beginComputePass();
    computePass.setPipeline(this.updatePipeline);
    computePass.setBindGroup(0, this.renderBindGroup0); // Group 0 for sampler/textures (required by pipeline layout)
    computePass.setBindGroup(1, this.computeBindGroup1); // Group 1 for engine state
    computePass.dispatchWorkgroups(1);
    computePass.end();

    // Copy to staging buffer to read audio triggers
    if (this.audioCount > 0) {
      commandEncoder.copyBufferToBuffer(
        this.engineBuffer,
        this.bufferOffsets.audio,
        this.stagingBuffer,
        0,
        this.audioCount * 4,
      );
    }

    this.device.queue.submit([commandEncoder.finish()]);

    // Read audio triggers (async, will play next frame)
    if (this.audioCount > 0) {
      this.stagingBuffer
        .mapAsync(GPUMapMode.READ, 0, this.audioCount * 4)
        .then(() => {
          const audioData = new Uint32Array(
            this.stagingBuffer.getMappedRange(0, this.audioCount * 4),
          );

          // Play sounds and reset triggers
          for (let i = 0; i < this.audioCount; i++) {
            if (audioData[i] > 0) {
              this.playSound(i);
            }
          }

          this.stagingBuffer.unmap();

          // Reset audio triggers
          const zeros = new Uint32Array(this.audioCount);
          this.device.queue.writeBuffer(
            this.engineBuffer,
            this.bufferOffsets.audio,
            zeros,
          );
        })
        .catch((err) => {
          console.warn("Failed to read audio triggers:", err);
        });
    }
  }

  render() {
    const commandEncoder = this.device.createCommandEncoder();
    const textureView = this.context.getCurrentTexture().createView();

    const renderPass = commandEncoder.beginRenderPass({
      colorAttachments: [
        {
          view: textureView,
          clearValue: { r: 0.0, g: 0.0, b: 0.0, a: 1.0 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
      depthStencilAttachment: {
        view: this.depthView,
        depthClearValue: 1.0,
        depthLoadOp: "clear",
        depthStoreOp: "store",
      },
    });

    renderPass.setPipeline(this.renderPipeline);
    renderPass.setBindGroup(0, this.renderBindGroup0);
    renderPass.setBindGroup(1, this.renderBindGroup1);
    if (this.renderBindGroup2) {
      renderPass.setBindGroup(2, this.renderBindGroup2);
    }

    // Draw either model vertices or fullscreen triangle
    const vertexCount = this.modelVertexCount > 0 ? this.modelVertexCount : 3;
    renderPass.draw(vertexCount);
    renderPass.end();

    this.device.queue.submit([commandEncoder.finish()]);
  }

  /**
   * Set an OSC value (same as native /u/<name> message)
   * @param {string} path - OSC path like "/u/bass" or "/u/0"
   * @param {number} value - Float value
   */
  setOsc(path, value) {
    if (!path.startsWith("/u/")) return;

    const name = path.slice(3);
    const idx = this.oscParams.indexOf(name);
    if (idx >= 0) {
      this.oscValues[idx] = value;
    } else {
      // Try numeric index
      const numIdx = parseInt(name);
      if (!isNaN(numIdx) && numIdx >= 0 && numIdx < 64) {
        this.oscValues[numIdx] = value;
      }
    }
  }

  /**
   * Get an OSC value
   * @param {string} path - OSC path like "/u/bass" or index
   * @returns {number} The OSC value
   */
  getOsc(path) {
    if (path.startsWith("/u/")) {
      const name = path.slice(3);
      const idx = this.oscParams.indexOf(name);
      if (idx >= 0) return this.oscValues[idx];
      const numIdx = parseInt(name);
      if (!isNaN(numIdx) && numIdx >= 0 && numIdx < 64) return this.oscValues[numIdx];
    }
    return 0;
  }

  /**
   * Control video playback (same as native /vid/<name>/* messages)
   * @param {string} name - Video name (e.g., "britney.mp4")
   * @param {string} action - Action: "play", "pause", "stop", "seek", "position"
   * @param {number} value - For seek/position: time in seconds or x,y position
   */
  setVideo(name, action, value) {
    const idx = this.videoFiles.indexOf(name);
    if (idx < 0) return;

    const videoObj = this.videoElements?.[idx];
    const videoEl = videoObj?.element;
    if (!videoEl) return;

    switch (action) {
      case "play":
        videoEl.play();
        break;
      case "pause":
        videoEl.pause();
        break;
      case "stop":
        videoEl.pause();
        videoEl.currentTime = 0;
        break;
      case "seek":
        if (typeof value === "number") {
          videoEl.currentTime = value;
        }
        break;
    }
  }

  /**
   * Get video info
   * @param {string} name - Video name
   * @returns {Object} Video state (currentTime, duration, paused)
   */
  getVideo(name) {
    const idx = this.videoFiles.indexOf(name);
    if (idx < 0) return null;

    const videoObj = this.videoElements?.[idx];
    const videoEl = videoObj?.element;
    if (!videoEl) return null;

    return {
      currentTime: videoEl.currentTime,
      duration: videoEl.duration || 0,
      paused: videoEl.paused,
    };
  }

  /**
   * Reload the current shader (same as native /reload message)
   */
  async reload() {
    if (this.currentPath) {
      await this.loadGame(this.currentPath);
    }
  }

  /**
   * Load a different shader (same as native /shader message)
   * @param {string} path - Path to new shader
   */
  async setShader(path) {
    await this.loadGame(path);
  }

  stop() {
    this.running = false;
  }
}

/**
 * Create and start a WGSL game
 * @param {string} url - URL to the game (either .wgsl file or .zip containing main.wgsl)
 * @param {HTMLCanvasElement} canvas - Canvas element to render to
 * @param {Function} handleError - Optional error handler callback
 * @returns {Promise<Object>} Game metadata (title, width, height, sounds, textures)
 */
export async function createGame(url, canvas, handleError) {
  const engine = new WGSLGameEngine(canvas, handleError);
  const metadata = await engine.init(url);
  return {
    ...metadata,
    engine, // Return engine instance for advanced control (stop, etc.)
  };
}
