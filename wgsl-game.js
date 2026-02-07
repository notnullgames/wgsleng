import JSZip from "jszip";

class GameSource {
  constructor(files, baseUrl = "") {
    this.files = files;
    this.baseUrl = baseUrl;
  }

  static async fromZip(zipData) {
    const zip = await JSZip.loadAsync(zipData);
    const files = new Map();

    for (const [filename, file] of Object.entries(zip.files)) {
      if (!file.dir) {
        const data = await file.async("uint8array");
        files.set(filename, data);
      }
    }

    return new GameSource(files);
  }

  static async fromUrl(baseUrl) {
    return new GameSource(new Map(), baseUrl);
  }

  async readFile(path) {
    if (this.files.size > 0) {
      if (!this.files.has(path)) {
        throw new Error(`File not found in zip: ${path}`);
      }
      return this.files.get(path);
    } else {
      const url = new URL(path, this.baseUrl).href;
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to fetch ${url}: ${response.status}`);
      }
      return new Uint8Array(await response.arrayBuffer());
    }
  }

  async readText(path) {
    const data = await this.readFile(path);
    return new TextDecoder().decode(data);
  }
}

function parseMetadata(code) {
  const titleRegex = /\/\*\*\s*@title\s+(.+?)\s*\*\//;
  const textureRegex = /\/\*\*\s*@asset\s+texture\s+([^\s]+)\s*\*\//g;
  const soundRegex = /\/\*\*\s*@asset\s+sound\s+([^\s]+)\s*\*\//g;

  const titleMatch = titleRegex.exec(code);
  const title = titleMatch ? titleMatch[1] : "WGSL Shader Game";

  const textures = [];
  const sounds = [];

  let match;
  while ((match = textureRegex.exec(code)) !== null) {
    textures.push(match[1]);
  }
  while ((match = soundRegex.exec(code)) !== null) {
    sounds.push(match[1]);
  }

  return { title, textures, sounds };
}

async function preprocessShader(
  code,
  currentPath,
  gameSource,
  visited = new Set(),
) {
  const includeRegex = /\/\*\*\s*@include\s+([^\s]+)\s*\*\//gm;
  let result = "";
  let lastPos = 0;

  for (const match of code.matchAll(includeRegex)) {
    result += code.substring(lastPos, match.index);

    const includePath = match[1];

    let fullPath;
    let fileToRead;

    if (gameSource.baseUrl) {
      fullPath = new URL(includePath, new URL(currentPath, gameSource.baseUrl))
        .href;
      fileToRead = fullPath;
    } else {
      const dir = currentPath.includes("/")
        ? currentPath.substring(0, currentPath.lastIndexOf("/"))
        : "";
      fullPath = dir ? `${dir}/${includePath}` : includePath;
      fileToRead = includePath;
    }

    if (visited.has(fullPath)) {
      throw new Error(`Circular include: ${fullPath}`);
    }

    visited.add(fullPath);
    const includeCode = await gameSource.readText(fileToRead);

    result += `// --- Begin include: ${includePath} ---\n`;
    result += await preprocessShader(
      includeCode,
      fullPath,
      gameSource,
      visited,
    );
    result += `\n// --- End include: ${includePath} ---\n`;

    lastPos = match.index + match[0].length;
  }

  result += code.substring(lastPos);
  return result;
}

function playSound(audioContext, buffer) {
  const source = audioContext.createBufferSource();
  source.buffer = buffer;
  source.connect(audioContext.destination);
  source.start();
}

async function isZipFile(url) {
  if (url.endsWith(".zip")) return true;

  try {
    const response = await fetch(url, { method: "HEAD" });
    const contentType = response.headers.get("content-type");
    if (contentType === "application/zip") return true;

    const partialResponse = await fetch(url, {
      headers: { Range: "bytes=0-1" },
    });
    const bytes = new Uint8Array(await partialResponse.arrayBuffer());
    return bytes[0] === 0x50 && bytes[1] === 0x4b;
  } catch {
    return false;
  }
}

export default async function loadGame(url, canvas) {
  const isZip = await isZipFile(url);

  let gameSource;
  let shaderPath;

  if (isZip) {
    const response = await fetch(url);
    const zipData = await response.arrayBuffer();
    gameSource = await GameSource.fromZip(zipData);
    shaderPath = "main.wgsl";
  } else {
    const baseUrl = new URL(".", url).href;
    gameSource = await GameSource.fromUrl(baseUrl);
    shaderPath = url;
  }

  const adapter = await navigator.gpu?.requestAdapter();
  if (!adapter) {
    throw new Error("WebGPU not supported");
  }

  const device = await adapter.requestDevice();
  const context = canvas.getContext("webgpu");
  const format = navigator.gpu.getPreferredCanvasFormat();
  context.configure({ device, format });

  const buttons = new Uint32Array(12);
  const keyMap = {
    ArrowUp: 0,
    w: 0,
    W: 0,
    ArrowDown: 1,
    s: 1,
    S: 1,
    ArrowLeft: 2,
    a: 2,
    A: 2,
    ArrowRight: 3,
    d: 3,
    D: 3,
    z: 4,
    Z: 4,
    x: 5,
    X: 5,
    Enter: 10,
    Shift: 11,
  };

  addEventListener("keydown", (e) => {
    if (keyMap[e.key] !== undefined) {
      buttons[keyMap[e.key]] = 1;
      e.preventDefault();
    }
  });
  addEventListener("keyup", (e) => {
    if (keyMap[e.key] !== undefined) {
      buttons[keyMap[e.key]] = 0;
      e.preventDefault();
    }
  });

  const shaderCode = await gameSource.readText(shaderPath);
  const processedCode = await preprocessShader(
    shaderCode,
    shaderPath,
    gameSource,
  );
  const metadata = parseMetadata(processedCode);

  document.title = metadata.title;

  const module = device.createShaderModule({ code: processedCode });

  const audioContext = new AudioContext();
  const sounds = {};
  for (const soundFile of metadata.sounds) {
    const data = await gameSource.readFile(soundFile);
    const audioBuffer = await audioContext.decodeAudioData(
      data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength),
    );
    sounds[soundFile] = audioBuffer;
  }

  const imgData = await gameSource.readFile(metadata.textures[0]);
  const imgBlob = new Blob([imgData]);
  const img = await createImageBitmap(imgBlob);

  const texture = device.createTexture({
    size: [img.width, img.height],
    format: "rgba8unorm",
    usage:
      GPUTextureUsage.TEXTURE_BINDING |
      GPUTextureUsage.COPY_DST |
      GPUTextureUsage.RENDER_ATTACHMENT,
  });
  device.queue.copyExternalImageToTexture({ source: img }, { texture }, [
    img.width,
    img.height,
  ]);

  const sampler = device.createSampler({
    magFilter: "linear",
    minFilter: "linear",
  });

  const inputBuffer = device.createBuffer({
    size: 64,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });

  const stateBuffer = device.createBuffer({
    size: 24,
    usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(
    stateBuffer,
    0,
    new Float32Array([canvas.width / 2, canvas.height / 2, 0, 0, 0, 0]),
  );

  const audioBuffer = device.createBuffer({
    size: 4,
    usage:
      GPUBufferUsage.STORAGE |
      GPUBufferUsage.COPY_SRC |
      GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(audioBuffer, 0, new Uint32Array([0]));

  const audioReadBuffer = device.createBuffer({
    size: 4,
    usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
  });

  const computePipeline = device.createComputePipeline({
    layout: "auto",
    compute: { module, entryPoint: "update" },
  });

  const renderPipeline = device.createRenderPipeline({
    layout: "auto",
    vertex: { module, entryPoint: "vs_main" },
    fragment: { module, entryPoint: "fs_render", targets: [{ format }] },
    primitive: { topology: "triangle-list" },
  });

  const computeBindGroup = device.createBindGroup({
    layout: computePipeline.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: { buffer: inputBuffer } },
      { binding: 1, resource: { buffer: stateBuffer } },
      { binding: 2, resource: { buffer: audioBuffer } },
    ],
  });

  const renderTextureBindGroup = device.createBindGroup({
    layout: renderPipeline.getBindGroupLayout(0),
    entries: [
      { binding: 0, resource: texture.createView() },
      { binding: 1, resource: sampler },
    ],
  });

  const renderStateBindGroup = device.createBindGroup({
    layout: renderPipeline.getBindGroupLayout(1),
    entries: [{ binding: 0, resource: { buffer: stateBuffer } }],
  });

  let last = performance.now();
  let firstFrame = true;
  let lastBumpTrigger = 0;
  let audioReadPending = false;

  function frame() {
    const now = performance.now();
    let dt = (now - last) / 1000;
    last = now;

    if (firstFrame) {
      dt = 1 / 60;
      firstFrame = false;
    } else {
      dt = Math.min(dt, 0.1);
    }

    const inputData = new ArrayBuffer(64);
    const inputU32 = new Uint32Array(inputData);
    const inputF32 = new Float32Array(inputData);

    inputU32.set(buttons, 0);
    inputF32[12] = now / 1000;
    inputF32[13] = dt;
    inputF32[14] = canvas.width;
    inputF32[15] = canvas.height;

    device.queue.writeBuffer(inputBuffer, 0, inputData);

    const encoder = device.createCommandEncoder();

    const compute = encoder.beginComputePass();
    compute.setPipeline(computePipeline);
    compute.setBindGroup(0, computeBindGroup);
    compute.dispatchWorkgroups(1);
    compute.end();

    const render = encoder.beginRenderPass({
      colorAttachments: [
        {
          view: context.getCurrentTexture().createView(),
          loadOp: "clear",
          storeOp: "store",
        },
      ],
    });
    render.setPipeline(renderPipeline);
    render.setBindGroup(0, renderTextureBindGroup);
    render.setBindGroup(1, renderStateBindGroup);
    render.draw(3);
    render.end();

    encoder.copyBufferToBuffer(audioBuffer, 0, audioReadBuffer, 0, 4);
    device.queue.submit([encoder.finish()]);

    if (!audioReadPending) {
      audioReadPending = true;

      audioReadBuffer.mapAsync(GPUMapMode.READ).then(() => {
        const data = new Uint32Array(audioReadBuffer.getMappedRange());
        const trigger = data[0];

        if (trigger > lastBumpTrigger && metadata.sounds.length > 0) {
          playSound(audioContext, sounds[metadata.sounds[0]]);
          lastBumpTrigger = trigger;
        }

        audioReadBuffer.unmap();
        audioReadPending = false;
      });
    }

    requestAnimationFrame(frame);
  }

  frame();
}
