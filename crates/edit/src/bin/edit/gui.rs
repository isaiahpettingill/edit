use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct EditApp {
    pub path: Option<PathBuf>,
    pub name: String,
    pub content: String,
    pub original_content: String,
    pub untitled_counter: usize,

    // Find and Replace
    pub search_open: bool,
    pub replace_open: bool,
    pub search_text: String,
    pub replace_text: String,
    pub search_results: Vec<std::ops::Range<usize>>,
    pub search_result_index: usize,
    pub search_focus_triggered: bool,

    // Status bar & feedback
    pub status_message: String,
    pub status_time: Option<Instant>,

    // Styling & Theme
    pub font_size: f32,
    pub dark_mode: bool,

    // Dialogs
    pub show_about: bool,
}

fn load_system_fallbacks(fonts: &mut egui::FontDefinitions) {
    let mut emoji_fonts = Vec::new();
    let mut cjk_fonts = Vec::new();
    let mut general_fonts = Vec::new();

    // 1. Fast path: check known hardcoded system paths
    let hardcoded_paths = [
        // Windows
        ("C:\\Windows\\Fonts\\msyh.ttc", "cjk"),
        ("C:\\Windows\\Fonts\\msgothic.ttc", "cjk"),
        ("C:\\Windows\\Fonts\\seguiemj.ttf", "emoji"),
        ("C:\\Windows\\Fonts\\arial.ttf", "general"),
        // macOS
        ("/System/Library/Fonts/PingFang.ttc", "cjk"),
        ("/System/Library/Fonts/Apple Color Emoji.ttf", "emoji"),
        ("/Library/Fonts/Arial Unicode.ttf", "general"),
        // Linux / WSL
        ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", "cjk"),
        ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", "cjk"),
        ("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc", "cjk"),
        ("/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf", "emoji"),
        ("/usr/share/fonts/noto-emoji/NotoColorEmoji.ttf", "emoji"),
        ("/usr/share/fonts/truetype/droid/DroidSansFallback.ttf", "cjk"),
        ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "general"),
        ("/usr/share/ghostscript/10.07.0/Resource/CIDFSubst/DroidSansFallback.ttf", "cjk"),
        ("/mnt/c/Windows/Fonts/msyh.ttc", "cjk"),
        ("/mnt/c/Windows/Fonts/seguiemj.ttf", "emoji"),
        ("/mnt/c/Windows/Fonts/NotoColorEmoji_WindowsCompatible.ttf", "emoji"),
        ("/mnt/c/Windows/Fonts/arial.ttf", "general"),
    ];

    for &(path, category) in &hardcoded_paths {
        let path_buf = std::path::PathBuf::from(path);
        if path_buf.exists() {
            match category {
                "emoji" => {
                    if emoji_fonts.len() < 2 && !emoji_fonts.contains(&path_buf) {
                        emoji_fonts.push(path_buf);
                    }
                }
                "cjk" => {
                    if cjk_fonts.len() < 2 && !cjk_fonts.contains(&path_buf) {
                        cjk_fonts.push(path_buf);
                    }
                }
                "general" => {
                    if general_fonts.len() < 2 && !general_fonts.contains(&path_buf) {
                        general_fonts.push(path_buf);
                    }
                }
                _ => {}
            }
        }
    }

    // Helper to resolve HOME
    let expand_home = |path: &str| -> Option<PathBuf> {
        if path.starts_with("~/") || path == "~" {
            let home = if cfg!(windows) {
                std::env::var("USERPROFILE").ok()
            } else {
                std::env::var("HOME").ok()
            };
            if let Some(home_path) = home {
                let mut buf = PathBuf::from(home_path);
                if path.len() > 2 {
                    buf.push(&path[2..]);
                }
                return Some(buf);
            }
        }
        None
    };

    // 2. Slow path/Deep search: scan font directories if we still lack fonts
    if emoji_fonts.len() < 2 || cjk_fonts.len() < 2 || general_fonts.len() < 2 {
        let mut dirs_to_scan = Vec::new();

        if cfg!(windows) {
            if let Ok(windir) = std::env::var("WINDIR") {
                dirs_to_scan.push(PathBuf::from(windir).join("Fonts"));
            } else if let Ok(sysroot) = std::env::var("SYSTEMROOT") {
                dirs_to_scan.push(PathBuf::from(sysroot).join("Fonts"));
            } else {
                dirs_to_scan.push(PathBuf::from("C:\\Windows\\Fonts"));
            }
            if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
                dirs_to_scan.push(PathBuf::from(localappdata).join("Microsoft\\Windows\\Fonts"));
            }
        } else {
            // macOS
            dirs_to_scan.push(PathBuf::from("/System/Library/Fonts"));
            dirs_to_scan.push(PathBuf::from("/Library/Fonts"));
            if let Some(home_fonts) = expand_home("~/Library/Fonts") {
                dirs_to_scan.push(home_fonts);
            }

            // Linux
            dirs_to_scan.push(PathBuf::from("/usr/share/fonts"));
            dirs_to_scan.push(PathBuf::from("/usr/local/share/fonts"));
            if let Some(home_fonts) = expand_home("~/.local/share/fonts") {
                dirs_to_scan.push(home_fonts);
            }
            if let Some(home_fonts) = expand_home("~/.fonts") {
                dirs_to_scan.push(home_fonts);
            }

            // WSL Windows Fonts Mount
            dirs_to_scan.push(PathBuf::from("/mnt/c/Windows/Fonts"));
        }

        // Recursive directory scanning helper
        fn scan_dir_for_fonts(
            dir: &std::path::Path,
            depth: usize,
            emoji_fonts: &mut Vec<PathBuf>,
            cjk_fonts: &mut Vec<PathBuf>,
            general_fonts: &mut Vec<PathBuf>,
        ) {
            if depth > 4 {
                return;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Ok(metadata) = entry.metadata() {
                            if !metadata.is_symlink() {
                                scan_dir_for_fonts(&path, depth + 1, emoji_fonts, cjk_fonts, general_fonts);
                            }
                        }
                    } else if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                            let ext_lower = ext.to_lowercase();
                            if ext_lower == "ttf" || ext_lower == "ttc" || ext_lower == "otf" {
                                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                                    let name_lower = filename.to_lowercase();
                                    
                                    if name_lower.contains("emoji") || name_lower.contains("seguiemj") {
                                        if emoji_fonts.len() < 2 && !emoji_fonts.contains(&path) {
                                            emoji_fonts.push(path.clone());
                                        }
                                    } else if name_lower.contains("cjk") || name_lower.contains("wqy") 
                                        || name_lower.contains("msyh") || name_lower.contains("pingfang") 
                                        || name_lower.contains("droidsansfallback") || name_lower.contains("yahei") 
                                        || name_lower.contains("msgothic") 
                                    {
                                        if cjk_fonts.len() < 2 && !cjk_fonts.contains(&path) {
                                            cjk_fonts.push(path.clone());
                                        }
                                    } else if name_lower.contains("dejavu") || name_lower.contains("liberation") 
                                        || name_lower.contains("freesans") || name_lower.contains("arial") 
                                        || name_lower.contains("ubuntu") || name_lower.contains("notosans") 
                                        || name_lower.contains("notoserif") || name_lower.contains("roboto")
                                    {
                                        if general_fonts.len() < 2 && !general_fonts.contains(&path) {
                                            general_fonts.push(path.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if emoji_fonts.len() >= 2 && cjk_fonts.len() >= 2 && general_fonts.len() >= 2 {
                        break;
                    }
                }
            }
        }

        for dir in dirs_to_scan {
            if dir.exists() {
                scan_dir_for_fonts(&dir, 0, &mut emoji_fonts, &mut cjk_fonts, &mut general_fonts);
            }
            if emoji_fonts.len() >= 2 && cjk_fonts.len() >= 2 && general_fonts.len() >= 2 {
                break;
            }
        }
    }

    // 3. Load all selected fallback fonts into Egui
    let mut loaded_count = 0;
    let all_fallbacks = emoji_fonts.into_iter()
        .chain(cjk_fonts.into_iter())
        .chain(general_fonts.into_iter());

    for path in all_fallbacks {
        if let Ok(bytes) = std::fs::read(&path) {
            let name = format!("sys_fallback_{}", loaded_count);
            fonts.font_data.insert(
                name.clone(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );

            fonts.families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push(name.clone());
            fonts.families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push(name);
            loaded_count += 1;
        }
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Embed the NeoSpleen Nerd Font from our assets
    fonts.font_data.insert(
        "neospleen".to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../../../assets/NeoSpleen-NerdFont.ttf"
        ))),
    );

    // Load available system fallback fonts for CJK, Emoji, and Unicode coverage
    load_system_fallbacks(&mut fonts);

    // Put it first for both Proportional and Monospace families
    fonts.families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "neospleen".to_owned());
    fonts.families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "neospleen".to_owned());

    ctx.set_fonts(fonts);
}

fn update_font_sizes(ctx: &egui::Context, base_size: f32) {
    let mut style = (*ctx.global_style()).clone();

    style.text_styles = [
        (egui::TextStyle::Small, egui::FontId::new(base_size * 0.8, egui::FontFamily::Proportional)),
        (egui::TextStyle::Body, egui::FontId::new(base_size * 1.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Button, egui::FontId::new(base_size * 1.0, egui::FontFamily::Proportional)),
        (egui::TextStyle::Heading, egui::FontId::new(base_size * 1.4, egui::FontFamily::Proportional)),
        (egui::TextStyle::Monospace, egui::FontId::new(base_size, egui::FontFamily::Monospace)),
    ].into();

    ctx.set_global_style(style);
}

fn apply_theme(ctx: &egui::Context, dark_mode: bool) {
    let mut visuals = if dark_mode {
        let mut vis = egui::Visuals::dark();
        // Soft dark gray similar to Catppuccin Macchiato/Mocha without purple
        vis.panel_fill = egui::Color32::from_rgb(30, 30, 40); // Base: soft charcoal
        vis.window_fill = egui::Color32::from_rgb(20, 20, 27); // Crust/Mantle: soft dark gray
        vis.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 40);
        vis.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(205, 214, 244); // Text: ash / very light gray
        vis.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 56); // Surface0: slightly lighter gray
        vis.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(205, 214, 244);
        vis.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 60, 75); // Surface1: hovered gray
        vis.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(240, 240, 245);
        vis.widgets.active.bg_fill = egui::Color32::from_rgb(75, 75, 95); // Surface2: active gray
        vis.widgets.active.fg_stroke.color = egui::Color32::WHITE;
        vis.selection.bg_fill = egui::Color32::from_rgb(58, 76, 100); // Selection: soft blue-slate (no purple)
        vis.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(137, 180, 250)); // Soft blue highlight
        vis.hyperlink_color = egui::Color32::from_rgb(137, 220, 235); // Sky blue
        vis
    } else {
        let mut vis = egui::Visuals::light();
        vis.panel_fill = egui::Color32::from_rgb(245, 246, 248); // clean light slate
        vis.window_fill = egui::Color32::from_rgb(230, 233, 238);
        vis.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(230, 233, 238);
        vis.widgets.inactive.bg_fill = egui::Color32::from_rgb(215, 220, 228);
        vis.widgets.hovered.bg_fill = egui::Color32::from_rgb(190, 200, 215); // light steel blue hovered
        vis.widgets.hovered.fg_stroke.color = egui::Color32::BLACK;
        vis.widgets.active.bg_fill = egui::Color32::from_rgb(160, 175, 195); // active steel
        vis.widgets.active.fg_stroke.color = egui::Color32::BLACK;
        vis.selection.bg_fill = egui::Color32::from_rgb(180, 205, 230); // light steel selection
        vis.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(70, 120, 180));
        vis.hyperlink_color = egui::Color32::from_rgb(40, 100, 160);
        vis
    };

    // Zero-out all corner radii for flat aesthetics
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    visuals.window_corner_radius = egui::CornerRadius::ZERO;

    ctx.set_visuals(visuals);
}

impl EditApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_paths: Vec<PathBuf>) -> Self {
        let mut font_size = 16.0;
        let mut dark_mode = true;

        if let Some(storage) = cc.storage {
            if let Some(fs_str) = storage.get_string("font_size") {
                if let Ok(fs) = fs_str.parse::<f32>() {
                    font_size = fs;
                }
            }
            if let Some(dm_str) = storage.get_string("dark_mode") {
                if let Ok(dm) = dm_str.parse::<bool>() {
                    dark_mode = dm;
                }
            }
        }

        // Apply our curated premium theme style
        apply_theme(&cc.egui_ctx, dark_mode);
        setup_custom_fonts(&cc.egui_ctx);
        update_font_sizes(&cc.egui_ctx, font_size);

        let mut app = Self {
            path: None,
            name: "Untitled".to_string(),
            content: String::new(),
            original_content: String::new(),
            untitled_counter: 1,
            search_open: false,
            replace_open: false,
            search_text: String::new(),
            replace_text: String::new(),
            search_results: Vec::new(),
            search_result_index: 0,
            search_focus_triggered: false,
            status_message: "Welcome to Edit!".to_string(),
            status_time: Some(Instant::now()),
            font_size,
            dark_mode,
            show_about: false,
        };

        if let Some(path) = initial_paths.into_iter().next() {
            app.open_file(path);
        }

        app
    }

    pub fn is_dirty(&self) -> bool {
        self.content != self.original_content
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_time = Some(Instant::now());
    }

    pub fn new_untitled_document(&mut self) {
        if self.is_dirty() {
            let confirm = rfd::MessageDialog::new()
                .set_title("Unsaved Changes")
                .set_description(&format!("Do you want to save changes to {}?", self.name))
                .set_buttons(rfd::MessageButtons::YesNoCancel)
                .show();

            match confirm {
                rfd::MessageDialogResult::Yes => {
                    if !self.save_document() {
                        return; // Cancel or failed
                    }
                }
                rfd::MessageDialogResult::No => {}
                _ => return, // Cancel - do nothing
            }
        }

        let name = if self.untitled_counter == 1 {
            "Untitled".to_string()
        } else {
            format!("Untitled {}", self.untitled_counter)
        };
        self.untitled_counter += 1;
        self.path = None;
        self.name = name;
        self.content = String::new();
        self.original_content = String::new();
        self.set_status("Created new document");
        self.update_search_matches();
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_file(path);
        }
    }

    pub fn open_file(&mut self, path: PathBuf) {
        if self.is_dirty() {
            let confirm = rfd::MessageDialog::new()
                .set_title("Unsaved Changes")
                .set_description(&format!("Do you want to save changes to {}?", self.name))
                .set_buttons(rfd::MessageButtons::YesNoCancel)
                .show();

            match confirm {
                rfd::MessageDialogResult::Yes => {
                    if !self.save_document() {
                        return;
                    }
                }
                rfd::MessageDialogResult::No => {}
                _ => return,
            }
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Unknown".to_string());
                self.path = Some(path);
                self.name = name;
                self.content = content.clone();
                self.original_content = content;
                self.set_status(&format!("Opened {}", self.name));
                self.update_search_matches();
            }
            Err(e) => {
                self.set_status(&format!("Failed to open file: {}", e));
            }
        }
    }

    pub fn save_document(&mut self) -> bool {
        if let Some(path) = &self.path {
            match fs::write(path, &self.content) {
                Ok(()) => {
                    self.original_content = self.content.clone();
                    self.set_status(&format!("Saved {}", self.name));
                    true
                }
                Err(e) => {
                    self.set_status(&format!("Save failed: {}", e));
                    false
                }
            }
        } else {
            self.save_document_as()
        }
    }

    pub fn save_document_as(&mut self) -> bool {
        let dialog = rfd::FileDialog::new()
            .set_file_name(&self.name);

        if let Some(path) = dialog.save_file() {
            match fs::write(&path, &self.content) {
                Ok(()) => {
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Unknown".to_string());
                    self.path = Some(path);
                    self.name = name;
                    self.original_content = self.content.clone();
                    self.set_status(&format!("Saved {}", self.name));
                    true
                }
                Err(e) => {
                    self.set_status(&format!("Save failed: {}", e));
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn close_document(&mut self) {
        if self.is_dirty() {
            let confirm = rfd::MessageDialog::new()
                .set_title("Unsaved Changes")
                .set_description(&format!("Do you want to save changes to {}?", self.name))
                .set_buttons(rfd::MessageButtons::YesNoCancel)
                .show();

            match confirm {
                rfd::MessageDialogResult::Yes => {
                    if !self.save_document() {
                        return;
                    }
                }
                rfd::MessageDialogResult::No => {}
                _ => return,
            }
        }
        self.new_untitled_document();
        self.update_search_matches();
        self.set_status("Closed document");
    }

    pub fn update_search_matches(&mut self) {
        self.search_results.clear();
        if self.search_text.is_empty() {
            self.search_result_index = 0;
            return;
        }

        let content = &self.content;
        let query = &self.search_text;
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        let mut start = 0;
        while let Some(pos) = content_lower[start..].find(&query_lower) {
            let match_start = start + pos;
            let match_end = match_start + query.len();
            self.search_results.push(match_start..match_end);
            start = match_end;
            if query.is_empty() {
                break;
            }
        }

        if self.search_result_index >= self.search_results.len() {
            self.search_result_index = 0;
        }
    }

    pub fn find_next(&mut self, ctx: &egui::Context) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_result_index = (self.search_result_index + 1) % self.search_results.len();
        self.scroll_to_active_match(ctx);
    }

    pub fn find_prev(&mut self, ctx: &egui::Context) {
        if self.search_results.is_empty() {
            return;
        }
        if self.search_result_index == 0 {
            self.search_result_index = self.search_results.len() - 1;
        } else {
            self.search_result_index -= 1;
        }
        self.scroll_to_active_match(ctx);
    }

    pub fn replace_current(&mut self, ctx: &egui::Context) {
        if self.search_results.is_empty() {
            return;
        }

        let r = &self.search_results[self.search_result_index];
        self.content.replace_range(r.clone(), &self.replace_text);

        self.update_search_matches();
        if !self.search_results.is_empty() {
            if self.search_result_index >= self.search_results.len() {
                self.search_result_index = 0;
            }
            self.scroll_to_active_match(ctx);
        }
        self.set_status("Replaced occurrence");
    }

    pub fn replace_all(&mut self) {
        if self.search_text.is_empty() {
            return;
        }

        self.content = self.content.replace(&self.search_text, &self.replace_text);
        self.update_search_matches();
        self.set_status("Replaced all occurrences");
    }

    fn scroll_to_active_match(&mut self, ctx: &egui::Context) {
        if self.search_results.is_empty() {
            return;
        }
        let active_range = &self.search_results[self.search_result_index];
        let id = egui::Id::new("editor_text_edit");
        if let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) {
            let min = egui::text::CCursor::new(active_range.start);
            let max = egui::text::CCursor::new(active_range.end);
            let char_range = egui::text::CCursorRange::two(min, max);
            state.cursor.set_char_range(Some(char_range));
            state.store(ctx, id);
        }
    }
}

// Custom layouter that highlights search matches
fn highlight_layouter(
    ui: &egui::Ui,
    text: &str,
    wrap_width: f32,
    search_text: &str,
    active_match_idx: usize,
    matches: &[std::ops::Range<usize>],
    font_size: f32,
) -> std::sync::Arc<egui::Galley> {
    use egui::text::{LayoutJob, TextFormat};
    use egui::Color32;

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;

    let default_font = egui::FontId::monospace(font_size);
    let default_color = ui.style().visuals.widgets.noninteractive.text_color();

    let default_format = TextFormat {
        font_id: default_font.clone(),
        color: default_color,
        background: Color32::TRANSPARENT,
        ..Default::default()
    };

    if search_text.is_empty() || matches.is_empty() {
        job.append(text, 0.0, default_format);
        return ui.ctx().fonts_mut(|f| f.layout_job(job));
    }

    let mut last_idx = 0;
    for (i, m) in matches.iter().enumerate() {
        if m.start >= text.len() || m.end > text.len() {
            continue;
        }

        // Append text before match
        if m.start > last_idx {
            job.append(&text[last_idx..m.start], 0.0, default_format.clone());
        }

        // Highlight for the match
        let is_active = i == active_match_idx;
        let bg_color = if is_active {
            Color32::from_rgb(200, 100, 0) // orange for active match
        } else {
            Color32::from_rgb(70, 70, 30) // subtle dark yellow for standard matches
        };
        let text_color = if is_active {
            Color32::WHITE
        } else {
            Color32::from_rgb(255, 230, 150)
        };

        let match_format = TextFormat {
            font_id: default_font.clone(),
            color: text_color,
            background: bg_color,
            ..Default::default()
        };

        job.append(&text[m.start..m.end], 0.0, match_format);
        last_idx = m.end;
    }

    if last_idx < text.len() {
        job.append(&text[last_idx..], 0.0, default_format);
    }

    ui.ctx().fonts_mut(|f| f.layout_job(job))
}

impl eframe::App for EditApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("font_size", self.font_size.to_string());
        storage.set_string("dark_mode", self.dark_mode.to_string());
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut font_changed = false;

        // Manage status bar timer
        if let Some(time) = self.status_time {
            if time.elapsed() > Duration::from_secs(4) {
                self.status_message.clear();
                self.status_time = None;
            }
        }

        // Keyboard Shortcuts
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.new_untitled_document();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.open_file_dialog();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_document();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::S)) {
            self.save_document_as();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)) {
            self.close_document();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::F)) {
            self.search_open = !self.search_open;
            if self.search_open {
                self.search_focus_triggered = true;
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::H))
            || ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::R)) {
            self.replace_open = !self.replace_open;
            if self.replace_open {
                self.search_open = true;
                self.search_focus_triggered = true;
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Plus))
            || ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Equals)) {
            self.font_size += 1.0;
            font_changed = true;
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Minus)) {
            if self.font_size > 8.0 {
                self.font_size -= 1.0;
                font_changed = true;
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Num0)) {
            self.font_size = 16.0;
            font_changed = true;
        }

        // Menu Bar Panel (integrated undecorated title bar controls + drag region)
        egui::Panel::top("menu_bar")
            .frame(egui::Frame::NONE
                .fill(ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::symmetric(0, 0))
                .corner_radius(egui::CornerRadius::ZERO)
            )
            .show_inside(ui, |ui| {
                let panel_height = 28.0;
                let (panel_rect, _response) = ui.allocate_at_least(egui::vec2(ui.available_width(), panel_height), egui::Sense::hover());

                // 1. Background drag interaction (drawn first, so it is in the background)
                let drag_response = ui.interact(panel_rect, ui.id().with("panel_drag"), egui::Sense::click_and_drag());
                if drag_response.dragged() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag_response.double_clicked() {
                    let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }

                // 2. Menu buttons on the left (offset by 8.0px horizontal margin for clean alignment)
                let menu_width = (panel_rect.width() - 100.0).max(0.0);
                let menu_rect = egui::Rect::from_min_max(
                    egui::pos2(panel_rect.min.x + 8.0, panel_rect.min.y),
                    egui::pos2(panel_rect.min.x + 8.0 + menu_width, panel_rect.max.y)
                );

                ui.scope_builder(egui::UiBuilder::default().max_rect(menu_rect), |ui| {
                    ui.spacing_mut().button_padding = egui::vec2(8.0, 4.0);

                    let dark_mode = ui.visuals().dark_mode;
                    let (hover_bg, active_bg) = if dark_mode {
                        (
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                        )
                    } else {
                        (
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 12),
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 20),
                        )
                    };

                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.hovered.bg_fill = hover_bg;
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hover_bg;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.active.bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.open.bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.open.weak_bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.open.bg_stroke = egui::Stroke::NONE;

                    ui.horizontal(|ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("New File (Ctrl+N)").clicked() {
                                self.new_untitled_document();
                                ui.close();
                            }
                            if ui.button("Open File... (Ctrl+O)").clicked() {
                                self.open_file_dialog();
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Save (Ctrl+S)").clicked() {
                                self.save_document();
                                ui.close();
                            }
                            if ui.button("Save As... (Ctrl+Shift+S)").clicked() {
                                self.save_document_as();
                                ui.close();
                            }
                            ui.separator();
                            if ui.button("Close File (Ctrl+W)").clicked() {
                                self.close_document();
                                ui.close();
                            }
                            if ui.button("Exit (Ctrl+Q)").clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });

                        ui.menu_button("Edit", |ui| {
                            if ui.button("Find (Ctrl+F)").clicked() {
                                self.search_open = true;
                                self.search_focus_triggered = true;
                                ui.close();
                            }
                            if ui.button("Replace (Ctrl+H)").clicked() {
                                self.replace_open = true;
                                self.search_open = true;
                                self.search_focus_triggered = true;
                                ui.close();
                            }
                        });

                        ui.menu_button("View", |ui| {
                            if ui.checkbox(&mut self.dark_mode, "Dark Mode").changed() {
                                apply_theme(ui.ctx(), self.dark_mode);
                                update_font_sizes(ui.ctx(), self.font_size);
                            }
                            ui.separator();
                            if ui.button("Increase Font Size (Ctrl++)").clicked() {
                                self.font_size += 1.0;
                                font_changed = true;
                            }
                            if ui.button("Decrease Font Size (Ctrl+-)").clicked() {
                                if self.font_size > 8.0 {
                                     self.font_size -= 1.0;
                                     font_changed = true;
                                }
                            }
                            if ui.button("Reset Font Size (Ctrl+0)").clicked() {
                                self.font_size = 16.0;
                                font_changed = true;
                            }
                        });

                        ui.menu_button("Help", |ui| {
                            if ui.button("About").clicked() {
                                self.show_about = true;
                                ui.close();
                            }
                        });
                    });
                });

                // 3. Window control buttons on the far right (. +/- X)
                let controls_width = 84.0; // 3 buttons * 28.0px
                let controls_rect = egui::Rect::from_min_max(
                    egui::pos2(panel_rect.max.x - controls_width, panel_rect.min.y),
                    panel_rect.max
                );

                ui.scope_builder(egui::UiBuilder::default().max_rect(controls_rect), |ui| {
                    let dark_mode = ui.visuals().dark_mode;
                    let (hover_bg, active_bg) = if dark_mode {
                        (
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12),
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                        )
                    } else {
                        (
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 12),
                            egui::Color32::from_rgba_unmultiplied(0, 0, 0, 20),
                        )
                    };

                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                    ui.style_mut().visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.hovered.bg_fill = hover_bg;
                    ui.style_mut().visuals.widgets.hovered.weak_bg_fill = hover_bg;
                    ui.style_mut().visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.active.bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.active.weak_bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.active.bg_stroke = egui::Stroke::NONE;

                    ui.style_mut().visuals.widgets.open.bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.open.weak_bg_fill = active_bg;
                    ui.style_mut().visuals.widgets.open.bg_stroke = egui::Stroke::NONE;

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        let btn_size = egui::vec2(28.0, panel_rect.height());

                        // Minimize button (.)
                        let min_btn = egui::Button::new(".")
                            .frame(false)
                            .corner_radius(egui::CornerRadius::ZERO);
                        let min_resp = ui.add_sized(btn_size, min_btn);
                        if min_resp.clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }

                        // Maximize button (+ when window normal, - when window maximized)
                        let is_maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                        let max_char = if is_maximized { "-" } else { "+" };
                        let max_btn = egui::Button::new(max_char)
                            .frame(false)
                            .corner_radius(egui::CornerRadius::ZERO);
                        let max_resp = ui.add_sized(btn_size, max_btn);
                        if max_resp.clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                        }

                        // Close button (X) with custom hover red style
                        let mut close_style = ui.style().as_ref().clone();
                        close_style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(180, 50, 50);
                        close_style.visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
                        close_style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(140, 30, 30);
                        close_style.visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;

                        ui.scope_builder(egui::UiBuilder::default().max_rect(ui.available_rect_before_wrap()).style(std::sync::Arc::new(close_style)), |ui| {
                            let close_btn = egui::Button::new("X")
                                .frame(false)
                                .corner_radius(egui::CornerRadius::ZERO);
                            let close_resp = ui.add_sized(btn_size, close_btn);
                            if close_resp.clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    });
                });

                // 4. Center-aligned title text
                if panel_rect.width() > 250.0 {
                    let title_text = if let Some(path) = &self.path {
                        path.file_name().unwrap_or_default().to_string_lossy().into_owned()
                    } else {
                        self.name.clone()
                    };
                    let name_with_dirty = if self.is_dirty() {
                        format!("{}*", title_text)
                    } else {
                        title_text
                    };

                    let painter = ui.painter();
                    let font_id = egui::FontId::proportional(14.0);
                    let text_color = ui.style().visuals.widgets.noninteractive.text_color();
                    painter.text(
                        panel_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        name_with_dirty,
                        font_id,
                        text_color
                    );
                }
            });

        // Search Panel (if open)
        if self.search_open {
            egui::Panel::top("search_panel")
                .frame(egui::Frame::NONE
                    .fill(ui.style().visuals.window_fill)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .corner_radius(egui::CornerRadius::ZERO)
                )
                .show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        let text_edit = egui::TextEdit::singleline(&mut self.search_text)
                            .desired_width(150.0);
                        let response = ui.add(text_edit);

                        if self.search_focus_triggered {
                            response.request_focus();
                            self.search_focus_triggered = false;
                        }

                        if response.changed() {
                            self.update_search_matches();
                        }

                        if ui.button("⏶").on_hover_text("Previous Match").clicked() {
                            self.find_prev(ui.ctx());
                        }
                        if ui.button("⏷").on_hover_text("Next Match").clicked() {
                            self.find_next(ui.ctx());
                        }

                        if !self.search_results.is_empty() {
                            ui.label(format!(
                                "{} of {}",
                                self.search_result_index + 1,
                                self.search_results.len()
                            ));
                        } else if !self.search_text.is_empty() {
                            ui.label("No matches");
                        }

                        ui.separator();

                        if self.replace_open {
                            ui.label("Replace:");
                            ui.text_edit_singleline(&mut self.replace_text);
                            if ui.button("Replace").clicked() {
                                self.replace_current(ui.ctx());
                            }
                            if ui.button("All").clicked() {
                                self.replace_all();
                            }
                        } else {
                            if ui.button("Replace Mode").clicked() {
                                self.replace_open = true;
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("×").clicked() {
                                self.search_open = false;
                                self.replace_open = false;
                                self.search_text.clear();
                                self.update_search_matches();
                            }
                        });
                    });
                });
        }

        // Status Bar Panel (at the bottom)
        egui::Panel::bottom("status_bar")
            .frame(egui::Frame::NONE
                .fill(ui.style().visuals.window_fill)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .corner_radius(egui::CornerRadius::ZERO)
            )
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    if !self.status_message.is_empty() {
                        ui.label(&self.status_message);
                    } else {
                        ui.label("Ready");
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let id = egui::Id::new("editor_text_edit");

                        let cursor_pos_str = if let Some(state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
                            if let Some(range) = state.cursor.char_range() {
                                let char_idx = range.primary.index;

                                // Calculate Ln and Col
                                let mut ln = 1;
                                let mut col = 1;
                                for ch in self.content.chars().take(char_idx) {
                                    if ch == '\n' {
                                        ln += 1;
                                        col = 1;
                                    } else {
                                        col += 1;
                                    }
                                }
                                format!("Ln {}, Col {}", ln, col)
                            } else {
                                "Ln 1, Col 1".to_string()
                            }
                        } else {
                            "Ln 1, Col 1".to_string()
                        };

                        ui.label(cursor_pos_str);
                        ui.separator();
                        ui.label(format!("Chars: {}", self.content.len()));
                        ui.separator();
                        let filename_display = if let Some(path) = &self.path {
                            path.file_name().unwrap_or_default().to_string_lossy().into_owned()
                        } else {
                            self.name.clone()
                        };
                        let name_with_dirty = if self.is_dirty() {
                            format!("{}*", filename_display)
                        } else {
                            filename_display
                        };
                        ui.label(name_with_dirty);
                        ui.separator();
                        ui.label("UTF-8");
                    });
                });
            });

        if font_changed {
            update_font_sizes(ui.ctx(), self.font_size);
        }

        // Central Panel: Editor Space
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE
                .fill(ui.style().visuals.panel_fill)
                .inner_margin(egui::Margin::same(6)) // minimal non-zero margin
                .corner_radius(egui::CornerRadius::ZERO)
            )
            .show_inside(ui, |ui| {
                let font_size = self.font_size;
                let search_text_clone = self.search_text.clone();
                let active_match_idx = self.search_result_index;
                let matches_clone = self.search_results.clone();

                let mut content = std::mem::take(&mut self.content);
                let mut changed = false;

                let mut layouter = move |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                    highlight_layouter(
                        ui,
                        text.as_str(),
                        wrap_width,
                        &search_text_clone,
                        active_match_idx,
                        &matches_clone,
                        font_size,
                    )
                };

                let available_height = ui.available_height();
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .max_height(f32::INFINITY)
                    .show(ui, |ui| {
                        let text_edit = egui::TextEdit::multiline(&mut content)
                            .font(egui::FontId::monospace(font_size))
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(1)
                            .margin(egui::Margin::same(6)) // minimal non-zero padding inside editor
                            .lock_focus(true)
                            .layouter(&mut layouter)
                            .id(egui::Id::new("editor_text_edit"));

                        let response = ui.add_sized([ui.available_width(), available_height], text_edit);
                        if response.changed() {
                            changed = true;
                        }
                    });

                // Put content back
                self.content = content;

                if changed {
                    self.update_search_matches();
                }
            });

        // About Dialog Modal Window
        if self.show_about {
            let mut show_about = true;
            let mut close_clicked = false;
            egui::Window::new("About Edit")
                .collapsible(false)
                .resizable(false)
                .open(&mut show_about)
                .show(ui.ctx(), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("Edit");
                        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                        ui.add_space(8.0);

                        let gradient_bg = if self.dark_mode {
                            egui::Color32::from_rgb(32, 38, 48)
                        } else {
                            egui::Color32::from_rgb(240, 240, 245)
                        };

                        egui::Frame::canvas(ui.style())
                            .fill(gradient_bg)
                            .corner_radius(egui::CornerRadius::ZERO)
                            .inner_margin(egui::Margin::same(12))
                            .show(ui, |ui| {
                                ui.label(
                                    "A modern, premium graphical text editor built with egui and eframe.\n\
                                    Paying homage to the classic MS-DOS Editor, but re-imagined with a state-of-the-art GUI layout."
                                );
                            });

                        ui.add_space(12.0);
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                    });
                });

            if !show_about || close_clicked {
                self.show_about = false;
            }
        }
    }
}
