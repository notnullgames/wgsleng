// WGSL Game Engine Runtime for web
// Loads and runs expanded WGSL games with WebGPU

import { Unzip, AsyncUnzipInflate } from "fflate";

const decoder = new TextDecoder();

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
  KeyW: BTN_UP,
  ArrowDown: BTN_DOWN,
  KeyS: BTN_DOWN,
  ArrowLeft: BTN_LEFT,
  KeyA: BTN_LEFT,
  ArrowRight: BTN_RIGHT,
  KeyD: BTN_RIGHT,
  KeyZ: BTN_A,
  KeyK: BTN_A,
  KeyX: BTN_B,
  KeyL: BTN_B,
  KeyC: BTN_X,
  KeyI: BTN_X,
  KeyV: BTN_Y,
  KeyJ: BTN_Y,
  KeyQ: BTN_L,
  KeyU: BTN_L,
  KeyE: BTN_R,
  KeyO: BTN_R,
  Enter: BTN_START,
  ShiftRight: BTN_SELECT,
  ShiftLeft: BTN_SELECT,
};

class WGSLGameEngine {
  constructor(canvas, handleError = console.error) {
    this.canvas = canvas;
    this.handleError = handleError;

    this.buttons = new Int32Array(12);
    this.time = 0;
    this.lastTime = 0;
    this.deltaTime = 0;

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
      const fields = gameStateStruct.match(/:\s*\w+[^,;]*/g) || [];
      for (const field of fields) {
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
      if (gameStateStruct) {
        header += `    state: GameState, // user's game state that persists across frames\n`;
      }
      if (metadata.sounds.length > 0) {
        header += `    audio: array<u32, ${metadata.sounds.length}>, // audio trigger counters\n`;
      }
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

      // Add bindings
      header += `// Bindings: group 0 = textures, group 1 = engine state\n\n`;

      // Add sampler
      header += `@group(0) @binding(0) var _engine_sampler: sampler;\n`;

      // Add texture bindings
      metadata.textures.forEach((texName, i) => {
        header += `@group(0) @binding(${i + 1}) var _texture_${i}: texture_2d<f32>; // ${texName}\n`;
      });

      // Add engine buffer
      header += `\n@group(1) @binding(0) var<storage, read_write> _engine: GameEngineHost;\n\n`;

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
    source = source.replace(/@engine\.sampler/g, "_engine_sampler");

    // Replace game_state. with _engine.state.
    source = source.replace(/\bgame_state\./g, "_engine.state.");

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

    // Set metadata
    this.gameTitle = metadata.title;
    this.canvas.width = metadata.width;
    this.canvas.height = metadata.height;
    this.soundFiles = metadata.sounds;
    this.textureFiles = metadata.textures;
    this.audioCount = metadata.sounds.length;
    this.textureCount = metadata.textures.length;
    this.stateSize = metadata.stateSize;

    // Load textures
    await this.loadTextures();

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

    // Setup render pipeline first
    this.renderPipeline = this.device.createRenderPipeline({
      layout: "auto",
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
    });

    // Setup compute pipeline with same bind group layouts as render pipeline
    this.updatePipeline = this.device.createComputePipeline({
      layout: "auto",
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
    //   state: GameState at offset 64 (aligned to 8 bytes for vec2f)
    //   audio: array<u32, N> at offset 64 + stateSize

    const buttonSize = 12 * 4; // 48 bytes
    const floatDataSize = 4 * 4; // 16 bytes

    // GameState alignment: vec2f requires 8-byte alignment
    // GameState size must be multiple of its alignment (8 bytes)
    const stateAlignment = 8;
    const alignedStateSize =
      Math.ceil(this.stateSize / stateAlignment) * stateAlignment;

    const audioSize = this.audioCount * 4;

    // Total size must be multiple of 16 for storage buffer
    const totalSizeUnaligned =
      buttonSize + floatDataSize + alignedStateSize + audioSize;
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

    this.bufferOffsets = {
      buttons: 0,
      floats: buttonSize, // 48
      state: buttonSize + floatDataSize, // 64 (8-byte aligned)
      audio: buttonSize + floatDataSize + alignedStateSize,
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
    // Group 0: Textures and sampler
    const group0Entries = [
      {
        binding: 0,
        resource: this.sampler,
      },
    ];

    // Add texture bindings
    this.textures.forEach((texture, i) => {
      group0Entries.push({
        binding: i + 1,
        resource: texture.createView(),
      });
    });

    // Create bind groups for render pipeline
    this.renderBindGroup0 = this.device.createBindGroup({
      layout: this.renderPipeline.getBindGroupLayout(0),
      entries: group0Entries,
    });

    this.renderBindGroup1 = this.device.createBindGroup({
      layout: this.renderPipeline.getBindGroupLayout(1),
      entries: [
        {
          binding: 0,
          resource: {
            buffer: this.engineBuffer,
          },
        },
      ],
    });

    // Create bind group for compute pipeline (only needs group 1)
    this.computeBindGroup1 = this.device.createBindGroup({
      layout: this.updatePipeline.getBindGroupLayout(1),
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
    });

    window.addEventListener("keyup", (e) => {
      const btn = KEY_MAP[e.code];
      if (btn !== undefined) {
        this.buttons[btn] = 0;
        e.preventDefault();
      }
    });

    // Refocus canvas on click
    this.canvas.addEventListener("click", () => this.canvas.focus());
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

  update() {
    // Write input state to buffer
    const inputData = new ArrayBuffer(48 + 16); // buttons + floats
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

    this.device.queue.writeBuffer(this.engineBuffer, 0, inputData);

    // Run compute shader (only needs group 1, not group 0 since it doesn't use textures)
    const commandEncoder = this.device.createCommandEncoder();
    const computePass = commandEncoder.beginComputePass();
    computePass.setPipeline(this.updatePipeline);
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
    });

    renderPass.setPipeline(this.renderPipeline);
    renderPass.setBindGroup(0, this.renderBindGroup0);
    renderPass.setBindGroup(1, this.renderBindGroup1);
    renderPass.draw(3); // Fullscreen triangle
    renderPass.end();

    this.device.queue.submit([commandEncoder.finish()]);
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
