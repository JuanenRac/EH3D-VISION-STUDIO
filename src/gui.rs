//! The window itself (built with the `gui` feature): it draws the state of `window::Window` and passes on what the person does. Everything that can be decided without a screen is
//! decided there, in code that is tested.

use crate::browse::{self, Browser};
use crate::config::Config;
use crate::image::Gray;
use crate::lang::{Lang, Msg};
use crate::window::{Section, Window, DESIGN_EXTENSIONS, IMAGE_EXTENSIONS, OUTPUT_EXTENSIONS};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke, TextureOptions,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// the colours of the theme Forge, the one the other programs of the suite start with: amber on charcoal
const BACKGROUND: Color32 = Color32::from_rgb(0x0f, 0x0d, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);
const MARK: Color32 = Color32::from_rgb(0x2f, 0xd8, 0xff);

/// The longest side of the texture the picture is drawn with; a larger picture is shown reduced (the marks are still placed on the full picture).
const TEXTURE_SIDE: usize = 4096;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Image,
    Design,
    Export,
}

enum Outcome {
    Cancel,
    Pick(PathBuf),
}

/// The window that opens a file or asks for the name of the file to write.
struct Dialog {
    kind: Kind,
    browser: Browser,
    /// The name being typed when the window asks for a file to write.
    export: Option<String>,
}

struct App {
    win: Window,
    dialog: Option<Dialog>,
    fit_pending: bool,
    texture: Option<(u64, egui::TextureHandle)>,
}

fn extension_is(path: &Path, list: &[&str]) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| list.contains(&e.as_str()))
}

impl App {
    fn new(cfg: Config, problem: Option<String>, files: &[PathBuf]) -> App {
        let lang = Lang::from_setting(&cfg.language);
        let mut app = App {
            win: Window::new(lang, cfg, problem),
            dialog: None,
            fit_pending: false,
            texture: None,
        };
        for f in files {
            app.open_any(f);
        }
        app
    }

    /// Opens a file by what it is: a picture or the points of a design.
    fn open_any(&mut self, path: &Path) {
        if extension_is(path, &DESIGN_EXTENSIONS) {
            self.win.load_design(path);
        } else if self.win.open_image(path) {
            self.fit_pending = true;
        }
    }

    fn folder(&self) -> PathBuf {
        self.win
            .image
            .as_ref()
            .and_then(|i| i.path.parent().map(|p| p.to_path_buf()))
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(browse::start_dir)
    }

    fn open_dialog(&mut self, kind: Kind) {
        let extensions: &[&'static str] = match kind {
            Kind::Image => &IMAGE_EXTENSIONS,
            Kind::Design => &DESIGN_EXTENSIONS,
            Kind::Export => &OUTPUT_EXTENSIONS,
        };
        let export = (kind == Kind::Export).then(|| self.win.suggested_output());
        self.dialog = Some(Dialog {
            kind,
            browser: Browser::new(self.folder(), extensions),
            export,
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("EH3D-VISION-STUDIO")
                        .strong()
                        .size(18.0)
                        .color(ACCENT),
                );
                ui.separator();
                if ui.button(Msg::OpenImage.tr(lang)).clicked() {
                    self.open_dialog(Kind::Image);
                }
                if ui.button(Msg::OpenDesign.tr(lang)).clicked() {
                    self.open_dialog(Kind::Design);
                }
                let can_export = self.win.isocam_lines().is_some();
                if ui
                    .add_enabled(can_export, egui::Button::new(Msg::ExportIsocam.tr(lang)))
                    .clicked()
                {
                    self.open_dialog(Kind::Export);
                }
                if ui
                    .add_enabled(
                        self.win.image.is_some(),
                        egui::Button::new(Msg::Fit.tr(lang)),
                    )
                    .clicked()
                {
                    self.fit_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(self.win.lang.name())
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.win.lang, l, l.name());
                            }
                        });
                    ui.label(egui::RichText::new(Msg::Language.tr(lang)).color(DIM));
                });
            });
        });
    }

    /// The fields of one section; true when the person left a field of the search (the marks are then looked for again).
    fn fields(&mut self, ui: &mut egui::Ui, section: Section) -> bool {
        let lang = self.win.lang;
        let mut left = false;
        for i in 0..self.win.fields.len() {
            if self.win.fields[i].section != section {
                continue;
            }
            let label = self.win.fields[i].label.tr(lang);
            let mut text = self.win.fields[i].text.clone();
            ui.label(label);
            let response =
                ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY));
            if response.changed() {
                self.win.set_field(i, text);
            }
            if response.lost_focus() {
                left = true;
            }
        }
        left
    }

    fn heading(ui: &mut egui::Ui, text: &str) {
        ui.add_space(8.0);
        ui.label(egui::RichText::new(text).strong().color(ACCENT));
        ui.separator();
    }

    fn search_section(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        Self::heading(ui, Msg::SearchTitle.tr(lang));
        let mut again = self.fields(ui, Section::Search) && self.win.image.is_some();
        let mut light = self.win.light_marks();
        if ui.checkbox(&mut light, Msg::LightMarks.tr(lang)).changed() {
            self.win.set_light_marks(light);
            again = self.win.image.is_some();
        }
        if ui
            .add_enabled(
                self.win.image.is_some(),
                egui::Button::new(Msg::Detect.tr(lang)),
            )
            .clicked()
        {
            again = true;
        }
        if again {
            self.win.detect();
        }
    }

    fn marks_section(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        Self::heading(ui, Msg::Marks.tr(lang));
        if self.win.image.is_none() {
            return;
        }
        if self.win.marks.is_empty() {
            ui.label(egui::RichText::new(Msg::NoMarks.tr(lang)).color(DIM));
            return;
        }
        let in_mm = self.win.marks_mm();
        egui::ScrollArea::vertical()
            .id_salt("marks")
            .max_height(170.0)
            .show(ui, |ui| {
                for i in 0..self.win.marks.len() {
                    let m = self.win.marks[i];
                    let mut line = format!(
                        "{}: ({:.1}, {:.1}) px  \u{2300} {:.1} px",
                        i + 1,
                        m.x,
                        m.y,
                        m.diameter
                    );
                    if let Some(mm) = in_mm.as_ref().and_then(|v| v.get(i)) {
                        line.push_str(&format!("  ->  ({:.3}, {:.3}) mm", mm.0, mm.1));
                    }
                    if ui
                        .selectable_label(self.win.selected == Some(i), line)
                        .clicked()
                    {
                        self.win.selected = Some(i);
                    }
                }
            });
    }

    fn scale_section(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        Self::heading(ui, Msg::ScaleTitle.tr(lang));
        self.fields(ui, Section::Scale);
        if ui
            .add_enabled(
                self.win.selected.is_some(),
                egui::Button::new(Msg::UseSelected.tr(lang)),
            )
            .clicked()
        {
            self.win.calibrate_selected();
        }
        let colour = if self.win.scale.is_some() { OK } else { DIM };
        ui.label(egui::RichText::new(self.win.scale_line()).color(colour));
    }

    fn distortion_section(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        Self::heading(ui, Msg::DistortionTitle.tr(lang));
        self.fields(ui, Section::Grid);
        if ui
            .add_enabled(
                self.win.image.is_some(),
                egui::Button::new(Msg::FitDistortion.tr(lang)),
            )
            .clicked()
        {
            self.win.fit_grid();
        }
        if let Some(line) = self.win.distortion_line() {
            ui.label(egui::RichText::new(line).color(OK));
        }
        let mut on = self.win.use_distortion;
        let changed = ui
            .add_enabled_ui(self.win.distortion.is_some(), |ui| {
                ui.checkbox(&mut on, Msg::ApplyDistortion.tr(lang))
                    .changed()
            })
            .inner;
        if changed {
            self.win.set_use_distortion(on);
        }
    }

    fn align_section(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        Self::heading(ui, Msg::AlignTitle.tr(lang));
        ui.label(egui::RichText::new(self.win.design_line()).color(DIM));
        let mut allow = self.win.cfg.allow_scale;
        if ui.checkbox(&mut allow, Msg::AllowScale.tr(lang)).changed() {
            self.win.set_allow_scale(allow);
        }
        self.fields(ui, Section::Align);
        if ui.button(Msg::Align.tr(lang)).clicked() {
            self.win.align();
        }
        for (text, problem) in self.win.alignment_lines() {
            let colour = if problem { DANGER } else { OK };
            ui.label(egui::RichText::new(text).color(colour));
        }
        if let Some(lines) = self.win.isocam_lines() {
            Self::heading(ui, Msg::IsocamTitle.tr(lang));
            let mut text = lines.join("\n");
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(lines.len().clamp(2, 8))
                    .desired_width(f32::INFINITY),
            );
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("tools")
            .default_size(340.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("tools")
                    .show(ui, |ui| {
                        self.search_section(ui);
                        self.marks_section(ui);
                        self.scale_section(ui);
                        self.distortion_section(ui);
                        self.align_section(ui);
                    });
            });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let colour = if self.win.status.is_problem() {
                    DANGER
                } else {
                    OK
                };
                ui.label(egui::RichText::new(self.win.status.text(lang)).color(colour));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::ZoomHint.tr(lang)).color(DIM));
                });
            });
        });
    }

    /// Makes the texture of the picture when a new one was opened.
    fn update_texture(&mut self, ctx: &egui::Context) {
        let serial = self.win.image_serial;
        if self.texture.as_ref().is_some_and(|(s, _)| *s == serial) {
            return;
        }
        self.texture = self
            .win
            .image
            .as_ref()
            .and_then(|i| texture_of(ctx, &i.gray))
            .map(|t| (serial, t));
        self.fit_pending = true;
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        let ctx = ui.ctx().clone();
        self.update_texture(&ctx);
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
            painter.rect_filled(rect, 0.0, BACKGROUND);
            if self.win.image.is_none() {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    Msg::DropHint.tr(lang),
                    egui::FontId::proportional(18.0),
                    DIM,
                );
                return;
            }
            if self.fit_pending && w > 40.0 && h > 40.0 {
                self.win.fit_view(w, h);
                self.fit_pending = false;
            }
            if response.double_clicked() {
                self.fit_pending = true;
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.win.viewport.pan_px(f64::from(d.x), f64::from(d.y));
            }
            if let Some(p) = response.hover_pos() {
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
                if (factor - 1.0).abs() > 1e-9 {
                    let at = p - rect.min;
                    self.win
                        .viewport
                        .zoom_at(f64::from(at.x), f64::from(at.y), w, h, factor);
                }
            }
            if response.clicked() {
                if let Some(p) = response.interact_pointer_pos() {
                    self.select_at(p, rect);
                }
            }
            self.paint(&painter, rect);
        });
    }

    /// Where a point of the picture is on the screen.
    fn screen(&self, rect: Rect, x: f64, y: f64) -> Pos2 {
        let (px, py) =
            self.win
                .viewport
                .to_screen(x, y, f64::from(rect.width()), f64::from(rect.height()));
        Pos2::new(rect.min.x + px as f32, rect.min.y + py as f32)
    }

    /// Selects the mark under a click, if there is one.
    fn select_at(&mut self, click: Pos2, rect: Rect) {
        let zoom = self.win.viewport.zoom;
        let mut best: Option<(usize, f32)> = None;
        for (i, m) in self.win.marks.iter().enumerate() {
            let p = self.win.mark_px(m);
            let at = self.screen(rect, p.0, p.1);
            let reach = ((m.diameter / 2.0 * zoom) as f32).max(6.0) + 4.0;
            let d = at.distance(click);
            if d <= reach && best.is_none_or(|(_, b)| d < b) {
                best = Some((i, d));
            }
        }
        if let Some((i, _)) = best {
            self.win.selected = Some(i);
        }
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect) {
        let painter = painter.with_clip_rect(rect);
        let Some(img) = &self.win.image else {
            return;
        };
        let zoom = self.win.viewport.zoom;
        if let Some((_, tex)) = &self.texture {
            let area = Rect::from_min_max(
                self.screen(rect, 0.0, 0.0),
                self.screen(rect, img.gray.w as f64, img.gray.h as f64),
            );
            let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
            painter.image(tex.id(), area, uv, Color32::WHITE);
        }
        let in_mm = self.win.marks_mm();
        for (i, m) in self.win.marks.iter().enumerate() {
            let p = self.win.mark_px(m);
            let at = self.screen(rect, p.0, p.1);
            let selected = self.win.selected == Some(i);
            let colour = if selected { ACCENT } else { MARK };
            let radius = ((m.diameter / 2.0 * zoom) as f32).max(4.0);
            painter.circle_stroke(
                at,
                radius,
                Stroke::new(if selected { 3.0 } else { 2.0 }, colour),
            );
            let arm = radius + 6.0;
            painter.line_segment(
                [at - egui::vec2(arm, 0.0), at + egui::vec2(arm, 0.0)],
                Stroke::new(1.0, colour),
            );
            painter.line_segment(
                [at - egui::vec2(0.0, arm), at + egui::vec2(0.0, arm)],
                Stroke::new(1.0, colour),
            );
            let text = match in_mm.as_ref().and_then(|v| v.get(i)) {
                Some(mm) => format!("{}  ({:.2}, {:.2}) mm", i + 1, mm.0, mm.1),
                None => format!("{}  ({:.1}, {:.1}) px", i + 1, p.0, p.1),
            };
            painter.text(
                at + egui::vec2(radius + 4.0, -radius - 4.0),
                egui::Align2::LEFT_BOTTOM,
                text,
                egui::FontId::proportional(13.0),
                colour,
            );
        }
        for d in self.win.design_px() {
            let at = self.screen(rect, d.0, d.1);
            let arm = 7.0;
            let stroke = Stroke::new(2.0, OK);
            painter.line_segment(
                [at + egui::vec2(-arm, -arm), at + egui::vec2(arm, arm)],
                stroke,
            );
            painter.line_segment(
                [at + egui::vec2(-arm, arm), at + egui::vec2(arm, -arm)],
                stroke,
            );
        }
    }

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match show_dialog(ctx, &mut dialog, self.win.lang) {
            None => self.dialog = Some(dialog),
            Some(Outcome::Cancel) => {}
            Some(Outcome::Pick(path)) => {
                let done = match dialog.kind {
                    Kind::Image => {
                        let ok = self.win.open_image(&path);
                        if ok {
                            self.fit_pending = true;
                        }
                        ok
                    }
                    Kind::Design => self.win.load_design(&path),
                    Kind::Export => self.win.export_isocam(&path),
                };
                if !done {
                    // the window stays open so that another file or name can be tried
                    self.dialog = Some(dialog);
                }
            }
        }
    }
}

/// The texture of a picture; a picture larger than the limit is reduced for it.
fn texture_of(ctx: &egui::Context, gray: &Gray) -> Option<egui::TextureHandle> {
    let side = gray.w.max(gray.h);
    let image = if side > TEXTURE_SIDE {
        let f = TEXTURE_SIDE as f64 / side as f64;
        let (w, h) = (
            ((gray.w as f64 * f).round() as usize).max(1),
            ((gray.h as f64 * f).round() as usize).max(1),
        );
        let small = gray.resize(w, h).ok()?;
        egui::ColorImage::from_gray([small.w, small.h], &small.data)
    } else {
        egui::ColorImage::from_gray([gray.w, gray.h], &gray.data)
    };
    Some(ctx.load_texture("picture", image, TextureOptions::LINEAR))
}

/// Draws the window that opens a file (or asks for the name of the file to write) and says what the person chose.
fn show_dialog(ctx: &egui::Context, d: &mut Dialog, lang: Lang) -> Option<Outcome> {
    let mut outcome = None;
    let mut open = true;
    let title = match d.kind {
        Kind::Image => Msg::OpenImageTitle,
        Kind::Design => Msg::OpenDesignTitle,
        Kind::Export => Msg::ExportTitle,
    };
    egui::Window::new(title.tr(lang))
        .open(&mut open)
        .collapsible(false)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(Msg::Up.tr(lang)).clicked() {
                    d.browser.up();
                }
                let edit = egui::TextEdit::singleline(&mut d.browser.path_text)
                    .desired_width(ui.available_width());
                let response = ui.add(edit);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(p) = d.browser.go_typed() {
                        if d.export.is_none() {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                }
            });
            if let Some(e) = &d.browser.error {
                ui.colored_label(DANGER, e.as_str());
            }
            if ui
                .checkbox(&mut d.browser.show_all, Msg::AllFiles.tr(lang))
                .changed()
            {
                d.browser.refresh();
            }
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if d.browser.entries.is_empty() {
                        ui.label(egui::RichText::new(Msg::EmptyFolder.tr(lang)).color(DIM));
                    }
                    for e in d.browser.entries.clone() {
                        let label = if e.is_dir {
                            format!("{}/", e.name)
                        } else {
                            format!("{}    ({})", e.name, browse::size_text(e.size))
                        };
                        if !ui.selectable_label(false, label).clicked() {
                            continue;
                        }
                        if e.is_dir {
                            d.browser.enter(&e.name);
                        } else if d.export.is_some() {
                            d.export = Some(e.name.clone());
                        } else if let Some(p) = d.browser.enter(&e.name) {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(name) = d.export.as_mut() {
                    ui.label(Msg::FileName.tr(lang));
                    ui.add(egui::TextEdit::singleline(name).desired_width(260.0));
                    if ui.button(Msg::Export.tr(lang)).clicked() && !name.trim().is_empty() {
                        outcome = Some(Outcome::Pick(d.browser.dir.join(name.trim())));
                    }
                }
                if ui.button(Msg::Cancel.tr(lang)).clicked() {
                    outcome = Some(Outcome::Cancel);
                }
            });
        });
    if !open && outcome.is_none() {
        outcome = Some(Outcome::Cancel);
    }
    outcome
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // a file dropped on the window is opened
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        for p in &dropped {
            self.open_any(p);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::O)) {
            self.open_dialog(Kind::Image);
        }
        self.top_bar(ui);
        self.status_bar(ui);
        self.side_panel(ui);
        self.canvas(ui);
        self.dialog_window(&ctx);
    }
}

/// The icon of the program for the window, from the file of the brand.
fn window_icon() -> Option<egui::IconData> {
    let bytes: &[u8] = include_bytes!("../images/brand/icon-256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

/// The fonts of the system that draw Japanese and Chinese are added to the ones of egui, which have no such letters.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".into(), Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// The colours of the theme Forge for the controls of egui.
fn apply_theme(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x11, 0x0e);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110);
    v.selection.stroke = Stroke::new(1.5, ACCENT);
    v.widgets.inactive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110));
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x28, 0x1c);
    v.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Opens the window. The files given here are opened at once: a picture, and a file of points as the design.
pub fn run(files: Vec<PathBuf>, cfg: Config, problem: Option<String>) -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 780.0])
        .with_min_inner_size([860.0, 520.0])
        .with_title("EH3D-VISION-STUDIO");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EH3D-VISION-STUDIO",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(App::new(cfg, problem, &files)))
        }),
    )
}
