// Shared library for WGSL game engine processing
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use regex::Regex;
use zip::ZipArchive;

pub mod obj_loader;
pub use obj_loader::ObjModel;

pub const BTN_UP: usize = 0;
pub const BTN_DOWN: usize = 1;
pub const BTN_LEFT: usize = 2;
pub const BTN_RIGHT: usize = 3;
pub const BTN_A: usize = 4;
pub const BTN_B: usize = 5;
pub const BTN_X: usize = 6;
pub const BTN_Y: usize = 7;
pub const BTN_L: usize = 8;
pub const BTN_R: usize = 9;
pub const BTN_START: usize = 10;
pub const BTN_SELECT: usize = 11;

pub enum GameSource {
    Directory(std::path::PathBuf),
    Zip(ZipArchive<std::fs::File>),
}

impl GameSource {
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Check if it's a .wgsl file
        if path.ends_with(".wgsl") {
            let path_obj = std::path::Path::new(path);
            if let Some(parent) = path_obj.parent() {
                return Ok(GameSource::Directory(parent.to_path_buf()));
            }
            return Ok(GameSource::Directory(std::path::PathBuf::from(".")));
        }

        // Check if it's a zip file
        if path.ends_with(".zip") {
            let file = std::fs::File::open(path)?;
            let archive = ZipArchive::new(file)?;
            return Ok(GameSource::Zip(archive));
        }

        // Otherwise treat as directory
        Ok(GameSource::Directory(std::path::PathBuf::from(path)))
    }

    pub fn read_file(&mut self, file_path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        match self {
            GameSource::Directory(base_path) => {
                let requested = std::path::Path::new(file_path);
                if requested.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
                    return Err("Directory traversal not allowed".into());
                }
                Ok(fs::read(base_path.join(file_path))?)
            }
            GameSource::Zip(archive) => {
                let stripped = file_path.strip_prefix("./").unwrap_or(file_path);
                match archive.by_name(stripped) {
                    Ok(mut file) => {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents)?;
                        Ok(contents)
                    }
                    Err(_) => Err(format!("File not found in zip: {}", file_path).into())
                }
            }
        }
    }

    pub fn read_text(&mut self, file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = self.read_file(file_path)?;
        Ok(String::from_utf8(bytes)?)
    }
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub textures: Vec<String>,
    pub sounds: Vec<String>,
    pub models: Vec<String>,
    pub state_size: usize,
}

pub struct PreprocessorState {
    pub game_source: GameSource,
    imported_files: HashSet<String>,
}

impl PreprocessorState {
    pub fn new(game_source: GameSource) -> Self {
        Self {
            game_source,
            imported_files: HashSet::new(),
        }
    }

    pub fn preprocess_shader(&mut self, source: &str, is_top_level: bool) -> Result<(String, Metadata), Box<dyn std::error::Error>> {
        let mut source = source.to_string();

        // Process @import directives first (recursive, like C #include)
        let import_re = Regex::new(r#"@import\("([^"]+)"\)"#)?;
        loop {
            // Collect captures into owned strings to avoid borrow issues
            let captures: Vec<(String, String)> = import_re.captures_iter(&source)
                .map(|cap| (cap.get(0).unwrap().as_str().to_string(), cap[1].to_string()))
                .collect();

            if captures.is_empty() {
                break;
            }

            for (full_match, filename) in captures {
                if self.imported_files.contains(&filename) {
                    source = source.replace(&full_match, &format!("// Already imported: {}", filename));
                    continue;
                }

                self.imported_files.insert(filename.clone());
                let imported_code = self.game_source.read_text(&filename)?;
                let (processed, _) = self.preprocess_shader(&imported_code, false)?;

                source = source.replace(&full_match, &format!("// Imported from {}\n{}\n", filename, processed));
            }
        }

        // Extract metadata
        let mut metadata = Metadata {
            title: "WGSL Game".to_string(),
            width: 800,
            height: 600,
            textures: Vec::new(),
            sounds: Vec::new(),
            models: Vec::new(),
            state_size: 16,
        };

        // Extract @set_title
        if let Some(cap) = Regex::new(r#"@set_title\("([^"]+)"\)"#)?.captures(&source) {
            metadata.title = cap[1].to_string();
        }

        // Extract @set_size
        if let Some(cap) = Regex::new(r#"@set_size\((\d+),\s*(\d+)\)"#)?.captures(&source) {
            metadata.width = cap[1].parse()?;
            metadata.height = cap[2].parse()?;
        }

        // Find all @sound() references
        let sound_re = Regex::new(r#"@sound\("([^"]+)"\)(?:\.(?:play|stop)\(\))?"#)?;
        for cap in sound_re.captures_iter(&source) {
            let sound_file = cap[1].to_string();
            if !metadata.sounds.contains(&sound_file) {
                metadata.sounds.push(sound_file);
            }
        }

        // Find all @texture() references
        let texture_re = Regex::new(r#"@texture\("([^"]+)"\)"#)?;
        for cap in texture_re.captures_iter(&source) {
            let texture_file = cap[1].to_string();
            if !metadata.textures.contains(&texture_file) {
                metadata.textures.push(texture_file);
            }
        }

        // Find all @model() references
        let model_re = Regex::new(r#"@model\("([^"]+)"\)"#)?;
        for cap in model_re.captures_iter(&source) {
            let model_file = cap[1].to_string();
            if !metadata.models.contains(&model_file) {
                metadata.models.push(model_file);
            }
        }

        // Remove @set_* directives
        source = Regex::new(r#"@set_title\([^)]+\)[^\n]*"#)?.replace_all(&source, "").to_string();
        source = Regex::new(r#"@set_size\([^)]+\)[^\n]*"#)?.replace_all(&source, "").to_string();

        // Find GameState struct
        let game_state_re = Regex::new(r"struct GameState\s*\{[^}]+\}")?;
        let game_state_struct = game_state_re.find(&source).map(|m| m.as_str().to_string());

        // Calculate GameState size
        if let Some(ref gs) = game_state_struct {
            // Match field types, including arrays with angle brackets
            let field_re = Regex::new(r":\s*(?:array<[^>]+>|[^,;\n]+)")?;
            let array_re = Regex::new(r"array<([^,>]+),\s*(\d+)>")?;
            let mut size = 0;
            let mut alignment = 4; // Track the largest member alignment

            for cap in field_re.captures_iter(gs) {
                let field = cap.get(0).unwrap().as_str();

                // Check if it's an array
                if let Some(array_cap) = array_re.captures(field) {
                    let element_type = &array_cap[1];
                    let count: usize = array_cap[2].parse().unwrap_or(1);

                    let (element_size, element_align) = if element_type.contains("vec4f") {
                        (16, 16)
                    } else if element_type.contains("vec3f") {
                        (16, 16) // vec3 aligns to 16 in arrays
                    } else if element_type.contains("vec2f") {
                        (8, 8)
                    } else {
                        (4, 4) // u32, i32, f32
                    };

                    alignment = alignment.max(element_align);
                    size += element_size * count;
                } else {
                    // Regular field
                    if field.contains("vec4f") {
                        size += 16;
                        alignment = alignment.max(16);
                    } else if field.contains("vec3f") {
                        size += 12;
                        alignment = alignment.max(16);
                    } else if field.contains("vec2f") {
                        size += 8;
                        alignment = alignment.max(8);
                    } else if field.contains("u32") || field.contains("i32") || field.contains("f32") {
                        size += 4;
                        alignment = alignment.max(4);
                    }
                }
            }

            // Round up to struct's alignment (largest member)
            metadata.state_size = ((size + alignment - 1) / alignment) * alignment;
        }

        // Build header (only for top-level)
        let mut header = String::new();
        if is_top_level {
            header.push_str("// Preprocessed WGSL - generated from macros\n\n");

            // Add GameState first
            if let Some(ref gs) = game_state_struct {
                header.push_str(gs);
                header.push_str("\n\n");
            }

            // Add GameEngineHost struct
            header.push_str("// Engine host struct that contains all engine state\n");
            header.push_str("struct GameEngineHost {\n");
            header.push_str("    buttons: array<i32, 12>, // the current state of virtual SNES gamepad (BTN_*)\n");
            header.push_str("    time: f32, // clock time\n");
            header.push_str("    delta_time: f32, // time since last frame\n");
            header.push_str("    screen_width: f32, // current screensize\n");
            header.push_str("    screen_height: f32, // current screensize\n");
            if game_state_struct.is_some() {
                header.push_str("    state: GameState, // user's game state that persists across frames\n");
            }
            if !metadata.sounds.is_empty() {
                header.push_str(&format!("    audio: array<u32, {}>, // audio trigger counters\n", metadata.sounds.len()));
            }
            header.push_str("}\n\n");

            // Add button constants
            header.push_str("// Button constants for input\n");
            header.push_str("const BTN_UP: u32 = 0u;\n");
            header.push_str("const BTN_DOWN: u32 = 1u;\n");
            header.push_str("const BTN_LEFT: u32 = 2u;\n");
            header.push_str("const BTN_RIGHT: u32 = 3u;\n");
            header.push_str("const BTN_A: u32 = 4u;\n");
            header.push_str("const BTN_B: u32 = 5u;\n");
            header.push_str("const BTN_X: u32 = 6u;\n");
            header.push_str("const BTN_Y: u32 = 7u;\n");
            header.push_str("const BTN_L: u32 = 8u;\n");
            header.push_str("const BTN_R: u32 = 9u;\n");
            header.push_str("const BTN_START: u32 = 10u;\n");
            header.push_str("const BTN_SELECT: u32 = 11u;\n\n");

            // Add bindings
            header.push_str("// Bindings: group 0 = textures, group 1 = engine state\n\n");
            header.push_str("@group(0) @binding(0) var _engine_sampler: sampler;\n");

            for (i, tex) in metadata.textures.iter().enumerate() {
                header.push_str(&format!("@group(0) @binding({}) var _texture_{}: texture_2d<f32>; // {}\n", i + 1, i, tex));
            }

            header.push_str("\n@group(1) @binding(0) var<storage, read_write> _engine: GameEngineHost;\n");

            // Add model buffers
            if !metadata.models.is_empty() {
                header.push_str("\n// Model data buffers\n");
                for (i, model) in metadata.models.iter().enumerate() {
                    let binding_base = 1 + i * 2;
                    header.push_str(&format!("struct Model{}Positions {{ data: array<vec3f> }}\n", i));
                    header.push_str(&format!("@group(2) @binding({}) var<storage, read> _model_{}_positions: Model{}Positions; // {}\n", binding_base, i, i, model));

                    header.push_str(&format!("struct Model{}Normals {{ data: array<vec3f> }}\n", i));
                    header.push_str(&format!("@group(2) @binding({}) var<storage, read> _model_{}_normals: Model{}Normals;\n", binding_base + 1, i, i));
                }
            }

            header.push_str("\n");

            // Remove GameState from source
            if game_state_struct.is_some() {
                source = game_state_re.replace(&source, "").to_string();
            }
        }

        // Replace macros
        source = source.replace("@engine.buttons", "_engine.buttons");
        source = source.replace("@engine.time", "_engine.time");
        source = source.replace("@engine.delta_time", "_engine.delta_time");
        source = source.replace("@engine.screen_width", "_engine.screen_width");
        source = source.replace("@engine.screen_height", "_engine.screen_height");
        source = source.replace("@engine.sampler", "_engine_sampler");
        source = source.replace("@engine.state", "_engine.state");

        // Replace @sound().play() and @sound().stop()
        for (i, sound) in metadata.sounds.iter().enumerate() {
            let escaped = sound.replace(".", "\\.");
            let play_re = Regex::new(&format!(r#"@sound\("{}"\)\.play\(\)"#, escaped))?;
            source = play_re.replace_all(&source, &format!("_engine.audio[{}]++", i)).to_string();

            let stop_re = Regex::new(&format!(r#"@sound\("{}"\)\.stop\(\)"#, escaped))?;
            source = stop_re.replace_all(&source, &format!("/* stop sound {} - not implemented */", i)).to_string();

            // Legacy @sound() syntax
            let legacy_re = Regex::new(&format!(r#"@sound\("{}"\)"#, escaped))?;
            source = legacy_re.replace_all(&source, &format!("_engine.audio[{}]", i)).to_string();
        }

        // Replace @texture()
        for (i, texture) in metadata.textures.iter().enumerate() {
            let escaped = texture.replace(".", "\\.");
            let texture_re = Regex::new(&format!(r#"@texture\("{}"\)"#, escaped))?;
            source = texture_re.replace_all(&source, &format!("_texture_{}", i)).to_string();
        }

        // Replace @model() - Note: This creates a struct-like accessor
        // Usage: @model("file.obj").positions[idx] becomes _model_0_positions.data[idx]
        for (i, model) in metadata.models.iter().enumerate() {
            let escaped = model.replace(".", "\\.");
            // Replace @model("file").positions with _model_N_positions.data
            let pos_re = Regex::new(&format!(r#"@model\("{}"\)\.positions"#, escaped))?;
            source = pos_re.replace_all(&source, &format!("_model_{}_positions.data", i)).to_string();

            // Replace @model("file").normals with _model_N_normals.data
            let norm_re = Regex::new(&format!(r#"@model\("{}"\)\.normals"#, escaped))?;
            source = norm_re.replace_all(&source, &format!("_model_{}_normals.data", i)).to_string();

            // Replace any remaining @model("file") with a comment about proper usage
            let model_re = Regex::new(&format!(r#"@model\("{}"\)"#, escaped))?;
            source = model_re.replace_all(&source, &format!("/* @model(\"{}\") - use .positions or .normals */", model)).to_string();
        }

        Ok((header + &source, metadata))
    }
}
