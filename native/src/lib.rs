// Shared library for WGSL game engine processing
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use regex::Regex;
use zip::ZipArchive;

pub mod obj_loader;
pub use obj_loader::ObjModel;

/// Number of named OSC float slots accessible via @osc("name") or @engine.osc[N]
pub const OSC_FLOAT_COUNT: usize = 64;

/// Size of the raw key state array — one slot per winit KeyCode variant (in enum order)
pub const KEY_ARRAY_SIZE: usize = 194;

/// Map a winit KeyCode variant name (= web e.code string) to its canonical index.
/// Both hosts use this same ordering so shader KEY_* constants are identical.
pub fn keycode_index(code: &str) -> Option<usize> {
    Some(match code {
        "Backquote" => 0,
        "Backslash" => 1,
        "BracketLeft" => 2,
        "BracketRight" => 3,
        "Comma" => 4,
        "Digit0" => 5,
        "Digit1" => 6,
        "Digit2" => 7,
        "Digit3" => 8,
        "Digit4" => 9,
        "Digit5" => 10,
        "Digit6" => 11,
        "Digit7" => 12,
        "Digit8" => 13,
        "Digit9" => 14,
        "Equal" => 15,
        "IntlBackslash" => 16,
        "IntlRo" => 17,
        "IntlYen" => 18,
        "KeyA" => 19,
        "KeyB" => 20,
        "KeyC" => 21,
        "KeyD" => 22,
        "KeyE" => 23,
        "KeyF" => 24,
        "KeyG" => 25,
        "KeyH" => 26,
        "KeyI" => 27,
        "KeyJ" => 28,
        "KeyK" => 29,
        "KeyL" => 30,
        "KeyM" => 31,
        "KeyN" => 32,
        "KeyO" => 33,
        "KeyP" => 34,
        "KeyQ" => 35,
        "KeyR" => 36,
        "KeyS" => 37,
        "KeyT" => 38,
        "KeyU" => 39,
        "KeyV" => 40,
        "KeyW" => 41,
        "KeyX" => 42,
        "KeyY" => 43,
        "KeyZ" => 44,
        "Minus" => 45,
        "Period" => 46,
        "Quote" => 47,
        "Semicolon" => 48,
        "Slash" => 49,
        "AltLeft" => 50,
        "AltRight" => 51,
        "Backspace" => 52,
        "CapsLock" => 53,
        "ContextMenu" => 54,
        "ControlLeft" => 55,
        "ControlRight" => 56,
        "Enter" => 57,
        "SuperLeft" => 58,
        "SuperRight" => 59,
        "ShiftLeft" => 60,
        "ShiftRight" => 61,
        "Space" => 62,
        "Tab" => 63,
        "Convert" => 64,
        "KanaMode" => 65,
        "Lang1" => 66,
        "Lang2" => 67,
        "Lang3" => 68,
        "Lang4" => 69,
        "Lang5" => 70,
        "NonConvert" => 71,
        "Delete" => 72,
        "End" => 73,
        "Help" => 74,
        "Home" => 75,
        "Insert" => 76,
        "PageDown" => 77,
        "PageUp" => 78,
        "ArrowDown" => 79,
        "ArrowLeft" => 80,
        "ArrowRight" => 81,
        "ArrowUp" => 82,
        "NumLock" => 83,
        "Numpad0" => 84,
        "Numpad1" => 85,
        "Numpad2" => 86,
        "Numpad3" => 87,
        "Numpad4" => 88,
        "Numpad5" => 89,
        "Numpad6" => 90,
        "Numpad7" => 91,
        "Numpad8" => 92,
        "Numpad9" => 93,
        "NumpadAdd" => 94,
        "NumpadBackspace" => 95,
        "NumpadClear" => 96,
        "NumpadClearEntry" => 97,
        "NumpadComma" => 98,
        "NumpadDecimal" => 99,
        "NumpadDivide" => 100,
        "NumpadEnter" => 101,
        "NumpadEqual" => 102,
        "NumpadHash" => 103,
        "NumpadMemoryAdd" => 104,
        "NumpadMemoryClear" => 105,
        "NumpadMemoryRecall" => 106,
        "NumpadMemoryStore" => 107,
        "NumpadMemorySubtract" => 108,
        "NumpadMultiply" => 109,
        "NumpadParenLeft" => 110,
        "NumpadParenRight" => 111,
        "NumpadStar" => 112,
        "NumpadSubtract" => 113,
        "Escape" => 114,
        "Fn" => 115,
        "FnLock" => 116,
        "PrintScreen" => 117,
        "ScrollLock" => 118,
        "Pause" => 119,
        "BrowserBack" => 120,
        "BrowserFavorites" => 121,
        "BrowserForward" => 122,
        "BrowserHome" => 123,
        "BrowserRefresh" => 124,
        "BrowserSearch" => 125,
        "BrowserStop" => 126,
        "Eject" => 127,
        "LaunchApp1" => 128,
        "LaunchApp2" => 129,
        "LaunchMail" => 130,
        "MediaPlayPause" => 131,
        "MediaSelect" => 132,
        "MediaStop" => 133,
        "MediaTrackNext" => 134,
        "MediaTrackPrevious" => 135,
        "Power" => 136,
        "Sleep" => 137,
        "AudioVolumeDown" => 138,
        "AudioVolumeMute" => 139,
        "AudioVolumeUp" => 140,
        "WakeUp" => 141,
        "Meta" => 142,
        "Hyper" => 143,
        "Turbo" => 144,
        "Abort" => 145,
        "Resume" => 146,
        "Suspend" => 147,
        "Again" => 148,
        "Copy" => 149,
        "Cut" => 150,
        "Find" => 151,
        "Open" => 152,
        "Paste" => 153,
        "Props" => 154,
        "Select" => 155,
        "Undo" => 156,
        "Hiragana" => 157,
        "Katakana" => 158,
        "F1" => 159,
        "F2" => 160,
        "F3" => 161,
        "F4" => 162,
        "F5" => 163,
        "F6" => 164,
        "F7" => 165,
        "F8" => 166,
        "F9" => 167,
        "F10" => 168,
        "F11" => 169,
        "F12" => 170,
        "F13" => 171,
        "F14" => 172,
        "F15" => 173,
        "F16" => 174,
        "F17" => 175,
        "F18" => 176,
        "F19" => 177,
        "F20" => 178,
        "F21" => 179,
        "F22" => 180,
        "F23" => 181,
        "F24" => 182,
        "F25" => 183,
        "F26" => 184,
        "F27" => 185,
        "F28" => 186,
        "F29" => 187,
        "F30" => 188,
        "F31" => 189,
        "F32" => 190,
        "F33" => 191,
        "F34" => 192,
        "F35" => 193,
        _ => return None,
    })
}

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
    /// Ordered list of @osc("name") parameters; index in this vec = osc slot index
    pub osc_params: Vec<String>,
    /// Ordered list of @video("file") filenames; index = video binding slot
    pub videos: Vec<String>,
    /// Sorted list of @camera(N) indices; index = camera binding slot
    pub cameras: Vec<u32>,
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
            state_size: 0, // set to 0 so no buffer space is reserved unless GameState is found
            osc_params: Vec::new(),
            videos: Vec::new(),
            cameras: Vec::new(),
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

        // Find all @texture_index() references (also load these textures)
        let texture_index_re = Regex::new(r#"@texture_index\("([^"]+)"\)"#)?;
        for cap in texture_index_re.captures_iter(&source) {
            let texture_file = cap[1].to_string();
            if !metadata.textures.contains(&texture_file) {
                metadata.textures.push(texture_file);
            }
        }

        // Find all @video() references
        let video_re = Regex::new(r#"@video\("([^"]+)"\)"#)?;
        for cap in video_re.captures_iter(&source) {
            let f = cap[1].to_string();
            if !metadata.videos.contains(&f) {
                metadata.videos.push(f);
            }
        }

        // Find all @camera() references
        let camera_re = Regex::new(r#"@camera\((\d+)\)"#)?;
        for cap in camera_re.captures_iter(&source) {
            let idx: u32 = cap[1].parse()?;
            if !metadata.cameras.contains(&idx) {
                metadata.cameras.push(idx);
            }
        }
        metadata.cameras.sort();

        // Find all @model() references
        let model_re = Regex::new(r#"@model\("([^"]+)"\)"#)?;
        for cap in model_re.captures_iter(&source) {
            let model_file = cap[1].to_string();
            if !metadata.models.contains(&model_file) {
                metadata.models.push(model_file);
            }
        }

        // Find all @osc() references and assign sequential slot indices
        let osc_ref_re = Regex::new(r#"@osc\("([^"]+)"\)"#)?;
        for cap in osc_ref_re.captures_iter(&source) {
            let param_name = cap[1].to_string();
            if !metadata.osc_params.contains(&param_name) {
                metadata.osc_params.push(param_name);
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
            header.push_str("    mouse: vec4f, // mouse state (iMouse): xy=pos, z=click_x (neg if not pressed), w=click_y\n");
            if game_state_struct.is_some() {
                header.push_str("    state: GameState, // user's game state that persists across frames\n");
            }
            if !metadata.sounds.is_empty() {
                header.push_str(&format!("    audio: array<u32, {}>, // audio trigger counters\n", metadata.sounds.len()));
            }
            header.push_str(&format!("    osc: array<f32, {}>, // OSC float uniforms: /u/name or /u/N\n", OSC_FLOAT_COUNT));
            header.push_str(&format!("    keys: array<u32, {}>, // raw key state: 1=down, 0=up, indexed by KEY_* constants\n", KEY_ARRAY_SIZE));
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

            // Key constants — indices match winit KeyCode enum order / web e.code strings
            header.push_str("// Key constants for @engine.keys[] — same on native and web\n");
            header.push_str("const KEY_BACKQUOTE: u32 = 0u;\n");
            header.push_str("const KEY_BACKSLASH: u32 = 1u;\n");
            header.push_str("const KEY_BRACKET_LEFT: u32 = 2u;\n");
            header.push_str("const KEY_BRACKET_RIGHT: u32 = 3u;\n");
            header.push_str("const KEY_COMMA: u32 = 4u;\n");
            header.push_str("const KEY_0: u32 = 5u;\n");
            header.push_str("const KEY_1: u32 = 6u;\n");
            header.push_str("const KEY_2: u32 = 7u;\n");
            header.push_str("const KEY_3: u32 = 8u;\n");
            header.push_str("const KEY_4: u32 = 9u;\n");
            header.push_str("const KEY_5: u32 = 10u;\n");
            header.push_str("const KEY_6: u32 = 11u;\n");
            header.push_str("const KEY_7: u32 = 12u;\n");
            header.push_str("const KEY_8: u32 = 13u;\n");
            header.push_str("const KEY_9: u32 = 14u;\n");
            header.push_str("const KEY_EQUAL: u32 = 15u;\n");
            header.push_str("const KEY_INTL_BACKSLASH: u32 = 16u;\n");
            header.push_str("const KEY_INTL_RO: u32 = 17u;\n");
            header.push_str("const KEY_INTL_YEN: u32 = 18u;\n");
            header.push_str("const KEY_A: u32 = 19u;\n");
            header.push_str("const KEY_B: u32 = 20u;\n");
            header.push_str("const KEY_C: u32 = 21u;\n");
            header.push_str("const KEY_D: u32 = 22u;\n");
            header.push_str("const KEY_E: u32 = 23u;\n");
            header.push_str("const KEY_F: u32 = 24u;\n");
            header.push_str("const KEY_G: u32 = 25u;\n");
            header.push_str("const KEY_H: u32 = 26u;\n");
            header.push_str("const KEY_I: u32 = 27u;\n");
            header.push_str("const KEY_J: u32 = 28u;\n");
            header.push_str("const KEY_K: u32 = 29u;\n");
            header.push_str("const KEY_L: u32 = 30u;\n");
            header.push_str("const KEY_M: u32 = 31u;\n");
            header.push_str("const KEY_N: u32 = 32u;\n");
            header.push_str("const KEY_O: u32 = 33u;\n");
            header.push_str("const KEY_P: u32 = 34u;\n");
            header.push_str("const KEY_Q: u32 = 35u;\n");
            header.push_str("const KEY_R: u32 = 36u;\n");
            header.push_str("const KEY_S: u32 = 37u;\n");
            header.push_str("const KEY_T: u32 = 38u;\n");
            header.push_str("const KEY_U: u32 = 39u;\n");
            header.push_str("const KEY_V: u32 = 40u;\n");
            header.push_str("const KEY_W: u32 = 41u;\n");
            header.push_str("const KEY_X: u32 = 42u;\n");
            header.push_str("const KEY_Y: u32 = 43u;\n");
            header.push_str("const KEY_Z: u32 = 44u;\n");
            header.push_str("const KEY_MINUS: u32 = 45u;\n");
            header.push_str("const KEY_PERIOD: u32 = 46u;\n");
            header.push_str("const KEY_QUOTE: u32 = 47u;\n");
            header.push_str("const KEY_SEMICOLON: u32 = 48u;\n");
            header.push_str("const KEY_SLASH: u32 = 49u;\n");
            header.push_str("const KEY_ALT_LEFT: u32 = 50u;\n");
            header.push_str("const KEY_ALT_RIGHT: u32 = 51u;\n");
            header.push_str("const KEY_BACKSPACE: u32 = 52u;\n");
            header.push_str("const KEY_CAPS_LOCK: u32 = 53u;\n");
            header.push_str("const KEY_CONTEXT_MENU: u32 = 54u;\n");
            header.push_str("const KEY_CTRL_LEFT: u32 = 55u;\n");
            header.push_str("const KEY_CTRL_RIGHT: u32 = 56u;\n");
            header.push_str("const KEY_ENTER: u32 = 57u;\n");
            header.push_str("const KEY_SUPER_LEFT: u32 = 58u;\n");
            header.push_str("const KEY_SUPER_RIGHT: u32 = 59u;\n");
            header.push_str("const KEY_SHIFT_LEFT: u32 = 60u;\n");
            header.push_str("const KEY_SHIFT_RIGHT: u32 = 61u;\n");
            header.push_str("const KEY_SPACE: u32 = 62u;\n");
            header.push_str("const KEY_TAB: u32 = 63u;\n");
            header.push_str("const KEY_DELETE: u32 = 72u;\n");
            header.push_str("const KEY_END: u32 = 73u;\n");
            header.push_str("const KEY_HOME: u32 = 75u;\n");
            header.push_str("const KEY_INSERT: u32 = 76u;\n");
            header.push_str("const KEY_PAGE_DOWN: u32 = 77u;\n");
            header.push_str("const KEY_PAGE_UP: u32 = 78u;\n");
            header.push_str("const KEY_DOWN: u32 = 79u;\n");
            header.push_str("const KEY_LEFT: u32 = 80u;\n");
            header.push_str("const KEY_RIGHT: u32 = 81u;\n");
            header.push_str("const KEY_UP: u32 = 82u;\n");
            header.push_str("const KEY_ESCAPE: u32 = 114u;\n");
            header.push_str("const KEY_F1: u32 = 159u;\n");
            header.push_str("const KEY_F2: u32 = 160u;\n");
            header.push_str("const KEY_F3: u32 = 161u;\n");
            header.push_str("const KEY_F4: u32 = 162u;\n");
            header.push_str("const KEY_F5: u32 = 163u;\n");
            header.push_str("const KEY_F6: u32 = 164u;\n");
            header.push_str("const KEY_F7: u32 = 165u;\n");
            header.push_str("const KEY_F8: u32 = 166u;\n");
            header.push_str("const KEY_F9: u32 = 167u;\n");
            header.push_str("const KEY_F10: u32 = 168u;\n");
            header.push_str("const KEY_F11: u32 = 169u;\n");
            header.push_str("const KEY_F12: u32 = 170u;\n");
            header.push_str("\n");

            // Add bindings
            header.push_str("// Bindings: group 0 = textures, group 1 = engine state\n\n");
            header.push_str("@group(0) @binding(0) var _engine_sampler: sampler;\n");

            for (i, tex) in metadata.textures.iter().enumerate() {
                header.push_str(&format!("@group(0) @binding({}) var _texture_{}: texture_2d<f32>; // {}\n", i + 1, i, tex));
            }

            let video_base = metadata.textures.len() + 1;
            for (i, vid) in metadata.videos.iter().enumerate() {
                header.push_str(&format!(
                    "@group(0) @binding({}) var _video_{}: texture_2d<f32>; // {}\n",
                    video_base + i, i, vid
                ));
            }

            let camera_base = metadata.textures.len() + metadata.videos.len() + 1;
            for (i, cam) in metadata.cameras.iter().enumerate() {
                header.push_str(&format!(
                    "@group(0) @binding({}) var _camera_{}: texture_2d<f32>; // camera {}\n",
                    camera_base + i, i, cam
                ));
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
        source = source.replace("@engine.mouse", "_engine.mouse");
        source = source.replace("@engine.keys", "_engine.keys");
        source = source.replace("@engine.sampler", "_engine_sampler");
        source = source.replace("@engine.state", "_engine.state");
        source = source.replace("@engine.osc", "_engine.osc");

        // Replace @osc("name") with indexed slot access
        for (i, name) in metadata.osc_params.iter().enumerate() {
            let escaped = regex::escape(name);
            let osc_name_re = Regex::new(&format!(r#"@osc\("{}"\)"#, escaped))?;
            source = osc_name_re.replace_all(&source, &format!("_engine.osc[{}]", i)).to_string();
        }

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

        // Replace @texture_index() with texture binding number
        for (i, texture) in metadata.textures.iter().enumerate() {
            let escaped = texture.replace(".", "\\.");
            let texture_index_re = Regex::new(&format!(r#"@texture_index\("{}"\)"#, escaped))?;
            source = texture_index_re.replace_all(&source, &format!("{}u", i)).to_string();
        }

        // Replace @video()
        for (i, video) in metadata.videos.iter().enumerate() {
            let escaped = video.replace(".", "\\.");
            let re = Regex::new(&format!(r#"@video\("{}"\)"#, escaped))?;
            source = re.replace_all(&source, &format!("_video_{}", i)).to_string();
        }

        // Replace @camera()
        for (i, cam_idx) in metadata.cameras.iter().enumerate() {
            let re = Regex::new(&format!(r#"@camera\({}\)"#, cam_idx))?;
            source = re.replace_all(&source, &format!("_camera_{}", i)).to_string();
        }

        // Replace @str() with fixed-size array of character codes (padded with zeros)
        let str_re = Regex::new(r#"@str\("((?:[^"\\]|\\.)*)"\)"#)?;
        let str_matches: Vec<(String, String)> = str_re.captures_iter(&source)
            .map(|cap| (cap.get(0).unwrap().as_str().to_string(), cap[1].to_string()))
            .collect();

        for (full_match, string) in str_matches {
            // Unescape the string
            let unescaped = string
                .replace("\\n", "\n")
                .replace("\\r", "\r")
                .replace("\\t", "\t")
                .replace("\\\"", "\"")
                .replace("\\\\", "\\");

            // Convert to character codes
            let mut char_codes: Vec<u32> = unescaped.chars().map(|c| c as u32).collect();

            // Pad with zeros to 128
            while char_codes.len() < 128 {
                char_codes.push(0);
            }

            // Create array literal
            let codes_str = char_codes.iter()
                .map(|c| format!("{}u", c))
                .collect::<Vec<_>>()
                .join(", ");
            let replacement = format!("array<u32, 128>({})", codes_str);

            source = source.replace(&full_match, &replacement);
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
