use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct Tab {
    pub path: Option<PathBuf>,
    pub name: String,
    pub content: String,
    pub original_content: String,
}

impl Tab {
    pub fn is_dirty(&self) -> bool {
        self.content != self.original_content
    }

    pub fn new_untitled(counter: usize) -> Self {
        let name = if counter == 0 {
            "Untitled".to_string()
        } else {
            format!("Untitled {}", counter)
        };
        Self {
            path: None,
            name,
            content: String::new(),
            original_content: String::new(),
        }
    }

    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let content = fs::read_to_string(&path)?;
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown".to_string());
        Ok(Self {
            path: Some(path),
            name,
            content: content.clone(),
            original_content: content,
        })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(path) = &self.path {
            fs::write(path, &self.content)?;
            self.original_content = self.content.clone();
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No path specified for save",
            ))
        }
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        fs::write(&path, &self.content)?;
        let name = path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown".to_string());
        self.path = Some(path);
        self.name = name;
        self.original_content = self.content.clone();
        Ok(())
    }
}

pub struct EditApp {
    pub tabs: Vec<Tab>,
    pub selected_tab_index: usize,
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

impl EditApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_paths: Vec<PathBuf>) -> Self {
        // Apply our curated premium theme style
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(18, 20, 24); // deep rich slate
        visuals.window_fill = egui::Color32::from_rgb(26, 29, 36);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(26, 29, 36);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(32, 38, 48);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(67, 85, 235); // premium indigo
        visuals.widgets.hovered.fg_stroke.color = egui::Color32::WHITE;
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(85, 105, 255);
        visuals.widgets.active.fg_stroke.color = egui::Color32::WHITE;

        visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
        cc.egui_ctx.set_visuals(visuals);

        let mut app = Self {
            tabs: Vec::new(),
            selected_tab_index: 0,
            untitled_counter: 0,
            search_open: false,
            replace_open: false,
            search_text: String::new(),
            replace_text: String::new(),
            search_results: Vec::new(),
            search_result_index: 0,
            search_focus_triggered: false,
            status_message: "Welcome to Edit!".to_string(),
            status_time: Some(Instant::now()),
            font_size: 15.0,
            dark_mode: true,
            show_about: false,
        };

        // Open initial paths if provided
        for path in initial_paths {
            if let Ok(tab) = Tab::from_file(path) {
                app.tabs.push(tab);
            }
        }

        // Ensure we have at least one tab open
        if app.tabs.is_empty() {
            app.new_untitled_tab();
        }

        app
    }

    pub fn set_status(&mut self, msg: &str) {
        self.status_message = msg.to_string();
        self.status_time = Some(Instant::now());
    }

    pub fn new_untitled_tab(&mut self) {
        self.tabs.push(Tab::new_untitled(self.untitled_counter));
        self.untitled_counter += 1;
        self.selected_tab_index = self.tabs.len() - 1;
        self.set_status("Created new document");
    }

    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            let tab = &self.tabs[index];
            if tab.is_dirty() {
                // If dirty, ask before closing or just warn the user.
                // We'll prompt using native dialog.
                let name = tab.name.clone();
                let confirm = rfd::MessageDialog::new()
                    .set_title("Unsaved Changes")
                    .set_description(&format!("Do you want to save changes to {}?", name))
                    .set_buttons(rfd::MessageButtons::YesNoCancel)
                    .show();
                
                match confirm {
                    rfd::MessageDialogResult::Yes => {
                        self.selected_tab_index = index;
                        if self.save_current_tab() {
                            self.tabs.remove(index);
                        } else {
                            return; // User cancelled save dialog or save failed
                        }
                    }
                    rfd::MessageDialogResult::No => {
                        self.tabs.remove(index);
                    }
                    _ => return, // Cancel or close dialog - do nothing
                }
            } else {
                self.tabs.remove(index);
            }

            if self.tabs.is_empty() {
                self.new_untitled_tab();
            } else if self.selected_tab_index >= self.tabs.len() {
                self.selected_tab_index = self.tabs.len() - 1;
            }
            self.set_status("Closed document");
            self.update_search_matches();
        }
    }

    pub fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.open_file(path);
        }
    }

    pub fn open_file(&mut self, path: PathBuf) {
        // If file is already open, switch to it
        for (idx, tab) in self.tabs.iter().enumerate() {
            if let Some(p) = &tab.path {
                if p == &path {
                    self.selected_tab_index = idx;
                    self.set_status(&format!("Switched to {}", tab.name));
                    return;
                }
            }
        }

        match Tab::from_file(path) {
            Ok(tab) => {
                // If current tab is untitled and empty, replace it
                if self.tabs.len() == 1 && self.tabs[0].path.is_none() && self.tabs[0].content.is_empty() {
                    self.tabs[0] = tab;
                    self.selected_tab_index = 0;
                } else {
                    self.tabs.push(tab);
                    self.selected_tab_index = self.tabs.len() - 1;
                }
                let name = self.tabs[self.selected_tab_index].name.clone();
                self.set_status(&format!("Opened {}", name));
                self.update_search_matches();
            }
            Err(e) => {
                self.set_status(&format!("Failed to open file: {}", e));
            }
        }
    }

    pub fn save_current_tab(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        let tab = &mut self.tabs[self.selected_tab_index];
        if tab.path.is_some() {
            let name = tab.name.clone();
            match tab.save() {
                Ok(()) => {
                    self.set_status(&format!("Saved {}", name));
                    true
                }
                Err(e) => {
                    self.set_status(&format!("Save failed: {}", e));
                    false
                }
            }
        } else {
            self.save_current_tab_as()
        }
    }

    pub fn save_current_tab_as(&mut self) -> bool {
        if self.tabs.is_empty() {
            return false;
        }
        
        let initial_name = self.tabs[self.selected_tab_index].name.clone();
        let dialog = rfd::FileDialog::new()
            .set_file_name(&initial_name);
            
        if let Some(path) = dialog.save_file() {
            let tab = &mut self.tabs[self.selected_tab_index];
            let success = match tab.save_as(path) {
                Ok(()) => true,
                Err(e) => {
                    self.set_status(&format!("Save failed: {}", e));
                    false
                }
            };
            if success {
                let name = self.tabs[self.selected_tab_index].name.clone();
                self.set_status(&format!("Saved {}", name));
            }
            success
        } else {
            false
        }
    }

    pub fn update_search_matches(&mut self) {
        self.search_results.clear();
        if self.tabs.is_empty() || self.search_text.is_empty() {
            self.search_result_index = 0;
            return;
        }

        let content = &self.tabs[self.selected_tab_index].content;
        let query = &self.search_text;
        
        // Simple case-insensitive search
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
        if self.search_results.is_empty() || self.tabs.is_empty() {
            return;
        }
        
        let tab = &mut self.tabs[self.selected_tab_index];
        let r = &self.search_results[self.search_result_index];
        
        tab.content.replace_range(r.clone(), &self.replace_text);
        
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
        if self.search_text.is_empty() || self.tabs.is_empty() {
            return;
        }

        let tab = &mut self.tabs[self.selected_tab_index];
        let replaced = tab.content.replace(&self.search_text, &self.replace_text);
        tab.content = replaced;
        
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
    let default_color = Color32::from_rgb(220, 220, 220); // soft white
    
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Manage status bar timer
        if let Some(time) = self.status_time {
            if time.elapsed() > Duration::from_secs(4) {
                self.status_message.clear();
                self.status_time = None;
            }
        }

        // Keyboard Shortcuts
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::N)) {
            self.new_untitled_tab();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::O)) {
            self.open_file_dialog();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S)) {
            self.save_current_tab();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::S)) {
            self.save_current_tab_as();
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::W)) {
            self.close_tab(self.selected_tab_index);
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

        // Menu Bar Panel
        egui::Panel::top("menu_bar").show_inside(ui, |ui| {
            egui::containers::menu::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New File (Ctrl+N)").clicked() {
                        self.new_untitled_tab();
                        ui.close();
                    }
                    if ui.button("Open File... (Ctrl+O)").clicked() {
                        self.open_file_dialog();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Save (Ctrl+S)").clicked() {
                        self.save_current_tab();
                        ui.close();
                    }
                    if ui.button("Save As... (Ctrl+Shift+S)").clicked() {
                        self.save_current_tab_as();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Close Tab (Ctrl+W)").clicked() {
                        self.close_tab(self.selected_tab_index);
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
                        if self.dark_mode {
                            ui.ctx().set_visuals(egui::Visuals::dark());
                        } else {
                            ui.ctx().set_visuals(egui::Visuals::light());
                        }
                    }
                    ui.separator();
                    if ui.button("Increase Font Size").clicked() {
                        self.font_size += 1.0;
                    }
                    if ui.button("Decrease Font Size").clicked() {
                        if self.font_size > 8.0 {
                            self.font_size -= 1.0;
                        }
                    }
                    if ui.button("Reset Font Size").clicked() {
                        self.font_size = 15.0;
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

        // Search Panel (if open)
        if self.search_open {
            egui::Panel::top("search_panel")
                .frame(egui::Frame::window(ui.style())
                    .fill(ui.global_style().visuals.window_fill)
                    .inner_margin(egui::Margin::same(8))
                    .corner_radius(egui::CornerRadius::same(4))
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
        egui::Panel::bottom("status_bar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if !self.status_message.is_empty() {
                    ui.label(&self.status_message);
                } else {
                    ui.label("Ready");
                }
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.tabs.is_empty() && self.selected_tab_index < self.tabs.len() {
                        let tab = &self.tabs[self.selected_tab_index];
                        let id = egui::Id::new("editor_text_edit");
                        
                        let cursor_pos_str = if let Some(state) = egui::text_edit::TextEditState::load(ui.ctx(), id) {
                            if let Some(range) = state.cursor.char_range() {
                                let char_idx = range.primary.index;
                                
                                // Calculate Ln and Col
                                let mut ln = 1;
                                let mut col = 1;
                                for ch in tab.content.chars().take(char_idx) {
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
                        ui.label(format!("Chars: {}", tab.content.len()));
                        ui.separator();
                        ui.label("UTF-8");
                    }
                });
            });
        });

        // Central Panel: Tabs + Editor
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Draw Tabs
            let mut tab_to_close = None;
            let mut tab_to_select = None;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (idx, tab) in self.tabs.iter().enumerate() {
                    let is_selected = idx == self.selected_tab_index;
                    let mut tab_text = tab.name.clone();
                    if tab.is_dirty() {
                        tab_text.push('*');
                    }

                    let bg = if is_selected {
                        if self.dark_mode {
                            egui::Color32::from_rgb(45, 55, 180) // nice active indigo
                        } else {
                            egui::Color32::from_rgb(200, 210, 255)
                        }
                    } else {
                        if self.dark_mode {
                            egui::Color32::from_rgb(26, 29, 36)
                        } else {
                            egui::Color32::from_rgb(230, 230, 235)
                        }
                    };
                    
                    let border_color = if is_selected {
                        if self.dark_mode {
                            egui::Color32::from_rgb(85, 105, 255)
                        } else {
                            egui::Color32::from_rgb(67, 85, 235)
                        }
                    } else {
                        if self.dark_mode {
                            egui::Color32::from_rgb(45, 50, 60)
                        } else {
                            egui::Color32::from_rgb(200, 200, 205)
                        }
                    };

                    egui::Frame::canvas(ui.style())
                        .fill(bg)
                        .stroke(egui::Stroke::new(1.0_f32, border_color))
                        .corner_radius(egui::CornerRadius::same(6))
                        .inner_margin(egui::Margin::symmetric(10, 6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                
                                let text_color = if is_selected {
                                    if self.dark_mode { egui::Color32::WHITE } else { egui::Color32::BLACK }
                                } else {
                                    if self.dark_mode { egui::Color32::from_rgb(180, 185, 195) } else { egui::Color32::from_rgb(100, 100, 110) }
                                };
                                
                                ui.add(egui::Label::new(
                                    egui::RichText::new(tab_text)
                                        .color(text_color)
                                        .strong()
                                ));
                                
                                // Click detection on the entire tab frame
                                let click_rect = ui.min_rect();
                                let response = ui.interact(click_rect, ui.id().with(idx), egui::Sense::click());
                                if response.clicked() {
                                    tab_to_select = Some(idx);
                                }

                                // Close tab button
                                let close_btn = ui.add(
                                    egui::Button::new("×")
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::NONE)
                                        .small()
                                );
                                if close_btn.clicked() {
                                    tab_to_close = Some(idx);
                                }
                            });
                        });
                }

                // Add Tab Button
                let add_btn = ui.add(
                    egui::Button::new("+")
                        .fill(if self.dark_mode { egui::Color32::from_rgb(32, 38, 48) } else { egui::Color32::from_rgb(220, 220, 225) })
                        .corner_radius(egui::CornerRadius::same(6))
                );
                if add_btn.clicked() {
                    self.new_untitled_tab();
                }
            });

            if let Some(idx) = tab_to_select {
                self.selected_tab_index = idx;
                self.update_search_matches();
            }
            if let Some(idx) = tab_to_close {
                self.close_tab(idx);
            }

            ui.add_space(6.0);

            // Editor Space
            if !self.tabs.is_empty() && self.selected_tab_index < self.tabs.len() {
                let font_size = self.font_size;
                let search_text_clone = self.search_text.clone();
                let active_match_idx = self.search_result_index;
                let matches_clone = self.search_results.clone();
                
                // Temporarily get the content as a separate mutable reference
                // to avoid double borrow on self during UI construction
                let mut content = std::mem::take(&mut self.tabs[self.selected_tab_index].content);
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

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let text_edit = egui::TextEdit::multiline(&mut content)
                        .font(egui::FontId::monospace(font_size))
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(30)
                        .lock_focus(true)
                        .layouter(&mut layouter)
                        .id(egui::Id::new("editor_text_edit"));
                    
                    let response = ui.add(text_edit);
                    if response.changed() {
                        changed = true;
                    }
                });

                // Put content back
                self.tabs[self.selected_tab_index].content = content;

                if changed {
                    self.update_search_matches();
                }
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
                            .corner_radius(egui::CornerRadius::same(8))
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
