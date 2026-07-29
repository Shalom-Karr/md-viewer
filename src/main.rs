#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Id, Key, KeyboardShortcut, Layout, Margin,
    Modifiers, Pos2, Rect, RichText, Stroke, StrokeKind, TextStyle, Theme, Vec2,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Rendered markdown is capped at this width and centred. Long-form text is
/// unreadable at full window width on a wide monitor.
const MEASURE_READ: f32 = 760.0;
/// The raw editor gets a wider measure — markdown source has long lines.
const MEASURE_EDIT: f32 = 920.0;

const APP_TITLE: &str = "Markdown Viewer";

const CMD: Modifiers = Modifiers {
    alt: false,
    ctrl: false,
    shift: false,
    mac_cmd: false,
    command: true,
};
const CMD_SHIFT: Modifiers = Modifiers {
    alt: false,
    ctrl: false,
    shift: true,
    mac_cmd: false,
    command: true,
};

const SC_OPEN: KeyboardShortcut = KeyboardShortcut::new(CMD, Key::O);
const SC_SAVE: KeyboardShortcut = KeyboardShortcut::new(CMD, Key::S);
const SC_SAVE_AS: KeyboardShortcut = KeyboardShortcut::new(CMD_SHIFT, Key::S);
const SC_MODE: KeyboardShortcut = KeyboardShortcut::new(CMD, Key::E);

const FILE_FILTER: [&str; 3] = ["md", "markdown", "txt"];

fn main() -> eframe::Result {
    let initial = std::env::args_os().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(move |cc| {
            // Without this, images referenced in markdown render as broken
            // boxes with no error logged.
            egui_extras::install_image_loaders(&cc.egui_ctx);
            install_style(&cc.egui_ctx);
            let mut app = MdApp::default();
            if let Some(path) = initial {
                app.load(path);
            }
            Ok(Box::new(app))
        }),
    )
}

// ---------------------------------------------------------------- theming ---

/// Applied once at startup. egui 0.35 keeps a separate `Style` per theme, so
/// metrics go through `all_styles_mut` and colours through `style_mut_of`.
fn install_style(ctx: &egui::Context) {
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Heading, FontId::proportional(30.0)),
            (TextStyle::Body, FontId::proportional(16.0)),
            (TextStyle::Monospace, FontId::monospace(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(12.0)),
        ]
        .into();

        // egui_commonmark lays a paragraph out as a wrapping row of label
        // widgets, so `item_spacing.y` is the only lever on line pitch.
        s.spacing.item_spacing = Vec2::new(8.0, 5.0);
        s.spacing.button_padding = Vec2::new(9.0, 5.0);
        s.spacing.interact_size.y = 26.0;
        s.spacing.indent = 22.0;
        s.spacing.window_margin = Margin::same(16);
        s.spacing.menu_margin = Margin::same(8);

        for w in [
            &mut s.visuals.widgets.noninteractive,
            &mut s.visuals.widgets.inactive,
            &mut s.visuals.widgets.hovered,
            &mut s.visuals.widgets.active,
            &mut s.visuals.widgets.open,
        ] {
            w.corner_radius = CornerRadius::same(5);
        }
        s.visuals.window_corner_radius = CornerRadius::same(10);
        s.visuals.menu_corner_radius = CornerRadius::same(8);
        // Vertical rules beside every nested list are noise in prose.
        s.visuals.indent_has_left_vline = false;
        // Markdown links carry their target in a tooltip.
        s.url_in_tooltip = true;
    });

    ctx.style_mut_of(Theme::Dark, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_gray(27);
        v.window_fill = Color32::from_gray(34);
        v.window_stroke = Stroke::new(1.0, Color32::from_gray(58));
        // Chrome < editor surface < page, so the panes read as distinct.
        v.extreme_bg_color = Color32::from_gray(17);
        v.code_bg_color = Color32::from_gray(44);
        // egui_commonmark renders tables as a striped `Grid`, and egui takes
        // the stripe colour from here. The default is barely visible.
        v.faint_bg_color = Color32::from_gray(37);
        v.hyperlink_color = Color32::from_rgb(106, 172, 255);
        // Body copy at gray 140 (the default) is too dim to read at length.
        v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(198);
        v.widgets.noninteractive.bg_stroke.color = Color32::from_gray(48);
        v.widgets.noninteractive.bg_fill = Color32::from_gray(26);
        v.widgets.noninteractive.weak_bg_fill = Color32::from_gray(26);
        v.widgets.inactive.weak_bg_fill = Color32::from_gray(46);
        v.widgets.inactive.bg_fill = Color32::from_gray(46);
        v.widgets.inactive.fg_stroke.color = Color32::from_gray(190);
        v.widgets.hovered.weak_bg_fill = Color32::from_gray(62);
        v.widgets.hovered.bg_fill = Color32::from_gray(62);
        // `active.fg_stroke` is what `RichText::strong()` resolves to, which is
        // how egui_commonmark paints bold text and headings.
        v.widgets.active.fg_stroke.color = Color32::from_gray(252);
        v.selection.bg_fill = Color32::from_rgb(38, 84, 132);
        v.selection.stroke = Stroke::new(1.0, Color32::from_gray(240));
    });

    ctx.style_mut_of(Theme::Light, |s| {
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_gray(255);
        v.window_fill = Color32::from_gray(255);
        v.window_stroke = Stroke::new(1.0, Color32::from_gray(205));
        v.extreme_bg_color = Color32::from_gray(246);
        v.code_bg_color = Color32::from_gray(234);
        v.faint_bg_color = Color32::from_gray(244);
        v.hyperlink_color = Color32::from_rgb(0, 98, 190);
        v.widgets.noninteractive.fg_stroke.color = Color32::from_gray(48);
        v.widgets.noninteractive.bg_stroke.color = Color32::from_gray(214);
        v.widgets.noninteractive.bg_fill = Color32::from_gray(255);
        v.widgets.noninteractive.weak_bg_fill = Color32::from_gray(255);
        v.widgets.inactive.weak_bg_fill = Color32::from_gray(235);
        v.widgets.inactive.bg_fill = Color32::from_gray(235);
        v.widgets.inactive.fg_stroke.color = Color32::from_gray(60);
        v.widgets.hovered.weak_bg_fill = Color32::from_gray(224);
        v.widgets.hovered.bg_fill = Color32::from_gray(224);
        v.widgets.active.fg_stroke.color = Color32::from_gray(10);
        v.selection.bg_fill = Color32::from_rgb(178, 214, 255);
        v.selection.stroke = Stroke::new(1.0, Color32::from_gray(20));
    });
}

/// Toolbar and status bar sit slightly off the reading surface.
fn chrome_fill(v: &egui::Visuals) -> Color32 {
    if v.dark_mode {
        Color32::from_gray(20)
    } else {
        Color32::from_gray(239)
    }
}

/// Reading sizes are too large for UI chrome; scale that subtree down.
fn chrome_text(ui: &mut egui::Ui) {
    let s = ui.style_mut();
    s.text_styles
        .insert(TextStyle::Body, FontId::proportional(13.0));
    s.text_styles
        .insert(TextStyle::Monospace, FontId::monospace(12.0));
    s.spacing.item_spacing = Vec2::new(6.0, 3.0);
}

/// Lay `add` out in a column of at most `measure`, centred in the available
/// width. `f32::INFINITY` means "fill", which is what the split panes want.
fn column<R>(ui: &mut egui::Ui, measure: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let avail = ui.available_width();
    let width = avail.min(measure);
    let pad = ((avail - width) * 0.5).max(0.0);
    ui.horizontal_top(|ui| {
        ui.add_space(pad);
        ui.vertical(|ui| {
            ui.set_max_width(width);
            add(ui)
        })
        .inner
    })
    .inner
}

// -------------------------------------------------------------------- app ---

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Mode {
    #[default]
    View,
    Edit,
    Split,
}

impl Mode {
    fn next(self) -> Self {
        match self {
            Self::View => Self::Edit,
            Self::Edit => Self::Split,
            Self::Split => Self::View,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::View => "View",
            Self::Edit => "Edit",
            Self::Split => "Split",
        }
    }
}

/// An action deferred behind the unsaved-changes prompt.
enum Pending {
    OpenDialog,
    OpenPath(PathBuf),
    Quit,
}

enum Choice {
    Save,
    Discard,
    Cancel,
}

#[derive(Default)]
struct Counts {
    words: usize,
    chars: usize,
    lines: usize,
}

#[derive(Default)]
struct MdApp {
    raw: String,
    path: Option<PathBuf>,
    mode: Mode,
    cache: CommonMarkCache,
    dirty: bool,
    counts: Counts,
    pending: Option<Pending>,
    /// message, is_error, raised_at
    flash: Option<(String, bool, Instant)>,
    /// Last title pushed to the viewport, so we only send it on change.
    title: String,
    /// Set just before we ask the window to close, so the guard lets it through.
    allow_close: bool,
}

impl MdApp {
    fn has_doc(&self) -> bool {
        self.path.is_some() || !self.raw.is_empty()
    }

    fn file_name(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
    }

    fn flash(&mut self, msg: impl Into<String>, error: bool) {
        self.flash = Some((msg.into(), error, Instant::now()));
    }

    fn recount(&mut self) {
        self.counts = Counts {
            words: self.raw.split_whitespace().count(),
            chars: self.raw.chars().count(),
            lines: self.raw.lines().count(),
        };
    }

    fn load(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
            Ok(text) => {
                self.raw = text;
                // `absolute` rather than `canonicalize`: the latter returns a
                // `\\?\`-prefixed path on Windows, which reads badly.
                self.path = Some(std::path::absolute(&path).unwrap_or(path));
                // CommonMarkCache holds per-document layout; a stale cache
                // renders the previous document's image positions.
                self.cache = CommonMarkCache::default();
                self.dirty = false;
                self.recount();
                if self.mode == Mode::Edit {
                    self.mode = Mode::View;
                }
                let name = self.file_name().unwrap_or_default();
                self.flash(format!("Opened {name}"), false);
            }
            Err(e) => self.flash(format!("Could not open: {e}"), true),
        }
    }

    /// Route an open through the unsaved-changes prompt when needed.
    fn request_open(&mut self, path: Option<PathBuf>) {
        if self.dirty {
            self.pending = Some(match path {
                Some(p) => Pending::OpenPath(p),
                None => Pending::OpenDialog,
            });
        } else {
            self.perform_open(path);
        }
    }

    fn perform_open(&mut self, path: Option<PathBuf>) {
        let path = path.or_else(|| {
            rfd::FileDialog::new()
                .set_title("Open markdown")
                .add_filter("Markdown", &FILE_FILTER)
                .pick_file()
        });
        if let Some(path) = path {
            self.load(path);
        }
    }

    fn save(&mut self) -> bool {
        match self.path.clone() {
            Some(p) => self.write_to(p),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) -> bool {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save markdown as")
            .add_filter("Markdown", &FILE_FILTER);
        match &self.path {
            Some(p) => {
                if let Some(dir) = p.parent() {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(name) = p.file_name() {
                    dialog = dialog.set_file_name(name.to_string_lossy());
                }
            }
            None => dialog = dialog.set_file_name("untitled.md"),
        }
        match dialog.save_file() {
            Some(p) => self.write_to(p),
            None => false,
        }
    }

    fn write_to(&mut self, path: PathBuf) -> bool {
        match fs::write(&path, self.raw.as_bytes()) {
            Ok(()) => {
                self.path = Some(path);
                self.dirty = false;
                let name = self.file_name().unwrap_or_default();
                self.flash(format!("Saved {name}"), false);
                true
            }
            Err(e) => {
                self.flash(format!("Could not save: {e}"), true);
                false
            }
        }
    }

    fn mark_edited(&mut self) {
        self.dirty = true;
        self.recount();
        self.cache = CommonMarkCache::default();
    }

    fn run_pending(&mut self, ctx: &egui::Context) {
        match self.pending.take() {
            Some(Pending::OpenDialog) => self.perform_open(None),
            Some(Pending::OpenPath(p)) => self.perform_open(Some(p)),
            Some(Pending::Quit) => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            None => {}
        }
    }

    // ------------------------------------------------------------ per-frame --

    fn handle_input(&mut self, ctx: &egui::Context) {
        if self.pending.is_none() {
            // Most specific first: `consume_shortcut` matches logically and
            // ignores a surplus Shift, so Ctrl+Shift+S must be tested first.
            if ctx.input_mut(|i| i.consume_shortcut(&SC_SAVE_AS)) {
                self.save_as();
            }
            if ctx.input_mut(|i| i.consume_shortcut(&SC_SAVE)) {
                self.save();
            }
            if ctx.input_mut(|i| i.consume_shortcut(&SC_OPEN)) {
                self.request_open(None);
            }
            if ctx.input_mut(|i| i.consume_shortcut(&SC_MODE)) {
                self.mode = self.mode.next();
            }

            let dropped = ctx.input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
            if let Some(path) = dropped {
                self.request_open(Some(path));
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) && self.dirty && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Quit);
        }
    }

    fn sync_title(&mut self, ctx: &egui::Context) {
        let title = match (self.file_name(), self.dirty) {
            (Some(n), true) => format!("{n} \u{2022} — {APP_TITLE}"),
            (Some(n), false) => format!("{n} — {APP_TITLE}"),
            (None, _) => APP_TITLE.to_owned(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::side_top_panel(ui.style())
            .fill(chrome_fill(ui.visuals()))
            .inner_margin(Margin::symmetric(12, 8));

        egui::Panel::top("toolbar").frame(frame).show(ui, |ui| {
            chrome_text(ui);
            ui.horizontal(|ui| {
                if ui
                    .button("Open")
                    .on_hover_text("Open a markdown file  (Ctrl+O)")
                    .clicked()
                {
                    self.request_open(None);
                }
                let has_doc = self.has_doc();
                if ui
                    .add_enabled(has_doc, egui::Button::new("Save"))
                    .on_hover_text("Save  (Ctrl+S)")
                    .clicked()
                {
                    self.save();
                }
                if ui
                    .add_enabled(has_doc, egui::Button::new("Save As…"))
                    .on_hover_text("Save to a new file  (Ctrl+Shift+S)")
                    .clicked()
                {
                    self.save_as();
                }

                if has_doc {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Segmented control: tight spacing reads as one control.
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        for mode in [Mode::View, Mode::Edit, Mode::Split] {
                            let selected = self.mode == mode;
                            let button = egui::Button::selectable(selected, mode.label())
                                .min_size(Vec2::new(58.0, 0.0));
                            if ui.add(button).on_hover_text("Cycle with Ctrl+E").clicked() {
                                self.mode = mode;
                            }
                        }
                    });

                    if let Some(name) = self.file_name() {
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.label(RichText::new(name).strong());
                        if self.dirty {
                            ui.label(
                                RichText::new("\u{2022} unsaved")
                                    .small()
                                    .color(ui.visuals().warn_fg_color),
                            );
                        }
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::global_theme_preference_buttons(ui);
                });
            });
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let frame = egui::Frame::side_top_panel(ui.style())
            .fill(chrome_fill(ui.visuals()))
            .inner_margin(Margin::symmetric(12, 6));

        egui::Panel::bottom("status").frame(frame).show(ui, |ui| {
            chrome_text(ui);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if self.has_doc() {
                        let c = &self.counts;
                        ui.label(
                            RichText::new(format!(
                                "{} words   {} chars   {} lines",
                                c.words, c.chars, c.lines
                            ))
                            .weak(),
                        );
                        ui.separator();
                        ui.label(RichText::new(self.mode.label()).weak());
                        ui.separator();
                        if self.dirty {
                            ui.label(RichText::new("Modified").color(ui.visuals().warn_fg_color));
                        } else {
                            ui.label(RichText::new("Saved").weak());
                        }
                        ui.separator();
                    }

                    // Remaining space, laid out from the left.
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let (text, color) = self.status_left(ui.visuals());
                        ui.add(egui::Label::new(RichText::new(text).color(color)).truncate());
                    });
                });
            });
        });
    }

    /// Transient result messages take over the path slot for a few seconds.
    fn status_left(&self, visuals: &egui::Visuals) -> (String, Color32) {
        if let Some((msg, error, at)) = &self.flash {
            let ttl = if *error { 8 } else { 3 };
            if at.elapsed() < Duration::from_secs(ttl) {
                let color = if *error {
                    visuals.error_fg_color
                } else {
                    visuals.widgets.noninteractive.fg_stroke.color
                };
                return (msg.clone(), color);
            }
        }
        match &self.path {
            Some(p) => (p.display().to_string(), visuals.weak_text_color()),
            None => (
                if self.raw.is_empty() {
                    "No document".to_owned()
                } else {
                    "Untitled".to_owned()
                },
                visuals.weak_text_color(),
            ),
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("preview")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                column(ui, MEASURE_READ, |ui| {
                    let width = ui.available_width().max(1.0) as usize;
                    CommonMarkViewer::new()
                        // Keeps images inside the reading measure.
                        .max_image_width(Some(width))
                        // egui_commonmark picks between these two by reading
                        // `visuals.dark_mode`, so code blocks follow the app
                        // theme. Both names come from syntect's built-in set.
                        .syntax_theme_dark("base16-eighties.dark")
                        .syntax_theme_light("base16-ocean.light")
                        .show(ui, &mut self.cache, &self.raw);
                    // Let the last line scroll clear of the status bar.
                    ui.add_space(56.0);
                });
            });
    }

    fn editor(&mut self, ui: &mut egui::Ui, measure: f32) {
        egui::ScrollArea::vertical()
            .id_salt("editor")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                column(ui, measure, |ui| {
                    let width = ui.available_width();
                    let changed = ui
                        .add(
                            egui::TextEdit::multiline(&mut self.raw)
                                .id_salt("md_source")
                                .code_editor()
                                .desired_width(width)
                                .desired_rows(28)
                                .margin(Margin::symmetric(12, 10)),
                        )
                        .changed();
                    if changed {
                        self.mark_edited();
                    }
                    ui.add_space(24.0);
                });
            });
    }

    fn empty_state(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() * 0.22).max(24.0));

            let (rect, _) = ui.allocate_exact_size(Vec2::new(78.0, 94.0), egui::Sense::hover());
            paint_doc_glyph(ui.painter(), rect, ui.visuals());

            ui.add_space(26.0);
            ui.label(
                RichText::new("No document open")
                    .size(22.0)
                    .color(ui.visuals().widgets.active.fg_stroke.color),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new("Drop a .md file anywhere in this window")
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(24.0);
            // A centred layout centres every row of a galley individually, so
            // the shortcut table needs its own left-aligned column.
            let weak = ui.visuals().weak_text_color();
            ui.allocate_ui_with_layout(
                Vec2::new(330.0, 0.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.label(
                        RichText::new(
                            "Ctrl+O    Open a file\n\
                             Ctrl+S    Save\n\
                             Ctrl+E    Cycle View / Edit / Split",
                        )
                        .monospace()
                        .color(weak),
                    );
                },
            );
        });
    }

    fn drop_overlay(&self, ui: &egui::Ui) {
        let hovering = ui
            .ctx()
            .input(|i| i.raw.hovered_files.iter().any(|f| f.path.is_some()));
        if !hovering {
            return;
        }
        let rect = ui.max_rect();
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_black_alpha(170));
        painter.rect_stroke(
            rect.shrink(16.0),
            CornerRadius::same(12),
            Stroke::new(2.0, Color32::from_white_alpha(190)),
            StrokeKind::Inside,
        );
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            "Drop to open",
            FontId::proportional(24.0),
            Color32::WHITE,
        );
    }

    fn prompt(&mut self, ctx: &egui::Context) {
        if self.pending.is_none() {
            return;
        }
        let name = self.file_name().unwrap_or_else(|| "Untitled".to_owned());
        let mut choice = None;

        let modal = egui::Modal::new(Id::new("unsaved_changes")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.label(
                RichText::new("Unsaved changes")
                    .size(20.0)
                    .color(ui.visuals().widgets.active.fg_stroke.color),
            );
            ui.add_space(8.0);
            ui.label(format!("{name} has been edited. Save before continuing?"));
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Save").clicked() {
                        choice = Some(Choice::Save);
                    }
                    if ui.button("Discard").clicked() {
                        choice = Some(Choice::Discard);
                    }
                    if ui.button("Cancel").clicked() {
                        choice = Some(Choice::Cancel);
                    }
                });
            });
        });
        if modal.should_close() {
            choice = Some(Choice::Cancel);
        }

        match choice {
            Some(Choice::Save) => {
                // A cancelled save dialog leaves the prompt up rather than
                // silently dropping the edits.
                if self.save() {
                    self.run_pending(ctx);
                }
            }
            Some(Choice::Discard) => {
                self.dirty = false;
                self.run_pending(ctx);
            }
            Some(Choice::Cancel) => self.pending = None,
            None => {}
        }
    }
}

fn paint_doc_glyph(painter: &egui::Painter, rect: Rect, visuals: &egui::Visuals) {
    let line = visuals.widgets.noninteractive.fg_stroke.color;
    let stroke = Stroke::new(1.6, line.gamma_multiply(0.55));
    let fold = 22.0;
    let outline = vec![
        Pos2::new(rect.left(), rect.top()),
        Pos2::new(rect.right() - fold, rect.top()),
        Pos2::new(rect.right(), rect.top() + fold),
        Pos2::new(rect.right(), rect.bottom()),
        Pos2::new(rect.left(), rect.bottom()),
    ];
    painter.add(egui::Shape::convex_polygon(
        outline.clone(),
        visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .gamma_multiply(0.45),
        Stroke::NONE,
    ));
    painter.add(egui::Shape::closed_line(outline, stroke));
    painter.add(egui::Shape::line(
        vec![
            Pos2::new(rect.right() - fold, rect.top()),
            Pos2::new(rect.right() - fold, rect.top() + fold),
            Pos2::new(rect.right(), rect.top() + fold),
        ],
        stroke,
    ));
    let rule = Stroke::new(2.0, line.gamma_multiply(0.4));
    for i in 0..3 {
        let y = rect.top() + fold + 18.0 + i as f32 * 13.0;
        let right = rect.right() - if i == 2 { 26.0 } else { 14.0 };
        painter.line_segment(
            [Pos2::new(rect.left() + 14.0, y), Pos2::new(right, y)],
            rule,
        );
    }
}

impl eframe::App for MdApp {
    // egui 0.35: the App trait provides a `Ui`, not a `Context`. Panels take
    // that `ui` rather than the old `ctx`, and `TopBottomPanel` is now the
    // unified `Panel::top`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_input(&ctx);
        self.sync_title(&ctx);

        self.toolbar(ui);
        self.status_bar(ui);

        let split = self.mode == Mode::Split && self.has_doc();
        if split {
            let frame =
                egui::Frame::central_panel(ui.style()).inner_margin(Margin::symmetric(14, 12));
            egui::Panel::left("source_pane")
                .resizable(true)
                .default_size(430.0)
                .min_size(280.0)
                .frame(frame)
                .show(ui, |ui| {
                    self.editor(ui, f32::INFINITY);
                });
        }

        let frame = egui::Frame::central_panel(ui.style()).inner_margin(Margin::symmetric(16, 12));
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            if !self.has_doc() {
                self.empty_state(ui);
            } else if self.mode == Mode::Edit {
                self.editor(ui, MEASURE_EDIT);
            } else {
                self.preview(ui);
            }
            self.drop_overlay(ui);
        });

        self.prompt(&ctx);

        // Nothing else drives a repaint, so expire the status flash on time.
        if self.flash.is_some() {
            ctx.request_repaint_after(Duration::from_millis(500));
        }
    }
}
