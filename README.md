# md-viewer

A markdown viewer/editor for Windows, in Rust. Opens `.md` files, renders them
in a centred reading column, edits the raw source side by side with a live
preview, saves, and follows the OS light/dark theme.

Status: **builds and runs.** See [Build status](#build-status).

## Features

- **Three modes** — View (rendered), Edit (raw source), Split (source left,
  live preview right, draggable divider). Cycle with `Ctrl+E`.
- **Readable measure** — rendered markdown is capped at 760 px and centred, so
  prose does not stretch across a wide monitor. The editor uses a wider 920 px
  measure because source lines are longer.
- **Saving** — `Ctrl+S` / `Ctrl+Shift+S`, with an unsaved-changes marker in the
  toolbar, the status bar, and the window title. Opening another file or
  closing the window with unsaved edits raises a modal prompt instead of
  discarding them.
- **Drag and drop** — drop a file onto the window to open it.
- **Command-line argument** — `md-viewer path\to\file.md` opens that file, so
  the exe works as an "Open with" target.
- **Status bar** — full file path, word/character/line counts, current mode,
  saved state, and transient save/open results.
- **Typography** — a real size hierarchy, comfortable line pitch, and a
  monospace face visually distinct from body text.
- **Syntax-highlighted code blocks** that follow the app theme.

## Keyboard shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl+O` | Open a file |
| `Ctrl+S` | Save |
| `Ctrl+Shift+S` | Save As |
| `Ctrl+E` | Cycle View → Edit → Split |
| `Esc` | Dismiss the unsaved-changes prompt |

They are discoverable in three places: button tooltips, the empty state, and
this table.

## Stack

| Crate | Version | Role |
| :--- | :--- | :--- |
| `eframe` / `egui` | 0.35 | GUI, immediate mode, native (no WebView2) |
| `egui_commonmark` | 0.24 | Markdown rendering — the only ready-made renderer in the Rust GUI ecosystem |
| `egui_extras` | 0.35 | Image loading inside markdown |
| `rfd` | 0.17 | Native file dialogs (`IFileOpenDialog` on Windows) |

`egui_commonmark` uses `pulldown-cmark` underneath, and `syntect` for fenced
code blocks via the `better_syntax_highlighting` feature. No dependencies were
added for any of the features above, and no font or theme assets are vendored.

## Prerequisites

Rust with the **MSVC** toolchain. The GNU toolchain has active compiler crash
bugs with egui ([rust-lang/rust#140237](https://github.com/rust-lang/rust/issues/140237)).

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
winget install Microsoft.VisualStudio.2022.BuildTools   # select "C++ build tools"
```

Before the first build, exclude the build directories from Defender. A cold
`cargo build` unpacks and compiles tens of thousands of small files, and
real-time scanning turns that into an I/O stall. Elevated PowerShell:

```powershell
Add-MpPreference -ExclusionPath 'C:\Users\nates\.cargo'
Add-MpPreference -ExclusionPath 'C:\Users\nates\Downloads\Claude'
```

## Build

```powershell
cargo run -- test.md    # debug, console window visible for println! output
cargo build --release   # release, console suppressed
```

Expect 20–40 s for a cold debug build with the glow backend, seconds
incrementally.

## Design notes

**Why glow, not wgpu.** `eframe` defaults to the wgpu backend, which pulls in
the naga shader compiler and roughly doubles cold build time. `glow` (OpenGL)
is well supported on all Windows 10/11 hardware. Switch to wgpu in
`Cargo.toml` if you later want the DirectX 12 path.

**Reading measure.** `column()` in `main.rs` centres a fixed-width child `Ui`
inside a `horizontal_top` row and calls `set_max_width` on it.
`egui_commonmark` derives its wrap width from `ui.available_width()`, so
constraining the parent constrains the rendered text. Passing
`f32::INFINITY` as the measure makes the helper fill instead — that is what
the split panes use.

**Line pitch.** egui 0.35 has no global line-height setting; `TextFormat::line_height`
is per-run only and `FontTweak::scale` shrinks the row box along with the
glyphs, so it buys nothing. What *does* work: `egui_commonmark` lays a
paragraph out as a wrapping row of individual label widgets
(`Layout::left_to_right(Align::BOTTOM).with_main_wrap(true)`), so
`Style::spacing::item_spacing.y` is the effective leading control. It is set to
5.0 against a 16 px body, and overridden back down inside the toolbar and
status bar via `chrome_text()`.

**Heading sizes.** `egui_commonmark` computes H2–H6 by linearly interpolating
between `TextStyle::Body` and `TextStyle::Heading` with fixed coefficients
(0.835, 0.668, 0.501, 0.334, 0.167); H1 uses `TextStyle::Heading` directly.
Body 16 / Heading 30 therefore gives 30 / 27.7 / 25.4 / 23.0 / 20.7 / 18.3.
The *ratios* between levels cannot be changed from outside the crate — see
[Markdown styling limits](#markdown-styling-limits).

**Bold is a colour, not a weight.** egui's `RichText::strong()` resolves to
`Visuals::widgets.active.fg_stroke.color`; it does not synthesise a heavier
face, and only one weight of the bundled proportional font ships with egui.
Both themes therefore set a muted body colour and a high-contrast strong
colour so that `**bold**` and headings actually read as emphasis.

**Theme handling.** `egui::global_theme_preference_buttons(ui)` renders the
System/Dark/Light control and reads and writes the `Context`'s
`ThemePreference` directly, so the app struct holds no theme state.
`ThemePreference::System` is the default and follows the OS via winit. Styling
is installed once at startup: metrics through `Context::all_styles_mut`, colours
through `Context::style_mut_of(Theme::Dark | Theme::Light, …)`. Do not set
styles every frame — egui 0.35 keeps a separate persistent `Style` per theme
and one-shot installation survives theme switches.

**Split view is two panels, not a hand-rolled splitter.** `Panel::left` inside
the same root `Ui` as the `CentralPanel` gives a draggable, size-persistent
divider for free.

## Markdown styling limits

`egui_commonmark` owns the render loop, so only some of the rendered output is
reachable from the host app. This is what was verified against the 0.24 source.

**Reachable, and used here:**

- Code-block fill, border and corner radius (`Visuals::extreme_bg_color`,
  `widgets.noninteractive.bg_stroke` / `.corner_radius`) — note that with
  `better_syntax_highlighting` the syntect theme's own background *overwrites*
  `extreme_bg_color` for code blocks.
- **Syntax theme follows the app theme automatically.** `egui_commonmark` picks
  between `syntax_theme_dark` and `syntax_theme_light` by reading
  `ui.style().visuals.dark_mode` every frame. This app sets
  `base16-eighties.dark` (neutral grey, unlike the default bluish
  `base16-ocean.dark`) and `base16-ocean.light`. syntect's built-in set is
  exactly: `base16-ocean.dark`, `base16-eighties.dark`, `base16-mocha.dark`,
  `base16-ocean.light`, `InspiredGitHub`, `Solarized (dark)`,
  `Solarized (light)`. Others need
  `CommonMarkCache::add_syntax_theme_from_bytes` and a vendored `.tmTheme`.
- Inline code chip — `Visuals::code_bg_color`.
- Table zebra striping — the crate hardcodes `Grid::striped(true)`, and egui
  takes the stripe colour from `Visuals::faint_bg_color`. The default is
  `from_additive_luminance(5)` (invisible), so both themes set a real value.
- Table container border — `Frame::group`, i.e.
  `widgets.noninteractive.bg_stroke` / `.corner_radius`.
- Horizontal rules — `egui::Separator`, i.e. `widgets.noninteractive.bg_stroke`.
- Bullets, list numbers and blockquote bars — `Visuals::strong_text_color()`
  and `weak_text_color()` respectively.
- Task-list checkboxes — a real widget drawn from `widgets.noninteractive`.
- Links — `Visuals::hyperlink_color`; `Style::url_in_tooltip` is enabled so the
  target shows on hover.
- Image width — `CommonMarkViewer::max_image_width`, set to the reading measure.
- Block rhythm — `Style::spacing::item_spacing.y`.

**Not reachable without patching the crate:**

- **Code-block padding.** The block is a `TextEdit` whose margin is left at
  egui's default `Margin::symmetric(4, 2)`; the crate never calls `.margin()`
  and wraps nothing in a `Frame`. Code blocks are therefore tighter than ideal.
- **A language label on code blocks.** The fence's language is only used to
  select a syntect syntax and is never rendered.
- **Disabling or moving the code-block copy button.** Always drawn, top-right.
- **Blockquote background fill.** `Frame::new()` with no fill; the left bar is a
  hardcoded 3 px line at a hardcoded 10 px left margin.
- **Table header emphasis.** The header row goes through exactly the same code
  path as body rows — no bold, no fill, no rule beneath it. Cell padding is a
  literal `ui.label("  ")`, and `item_spacing.x` is forced to 0 for the whole
  document, so columns cannot be loosened either.
- **Asymmetric heading spacing.** The crate emits the same `ui.label("\n")`
  before and after every heading, so "more space above than below" is not
  expressible. There is likewise no hook for a hairline rule under h1/h2.
- **Heading size ratios** — the interpolation coefficients above are constants.
- **Image centring and rounding.** Images flow inline in a left-to-right layout.

Getting past that list means writing a renderer directly against
`pulldown-cmark` events. That is a much larger project than styling, and it was
deliberately not started here.

## Gotchas

1. **Console window.** `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
   must stay as the first line of `main.rs`, or release builds spawn a black
   console behind the window. The `cfg_attr` form keeps it visible in debug so
   `println!` still works.
2. **Broken images.** `egui_extras::install_image_loaders(&cc.egui_ctx)` must be
   called once at startup. Without it, images fail silently — no error, no log.
3. **Version lockstep.** `egui_commonmark` must match `egui` exactly. A mismatch
   surfaces as conflicting trait impl linker errors, not a readable version
   complaint.
4. **Cache invalidation.** `CommonMarkCache` holds per-document layout. Reset it
   on file load and on every edit keystroke or the viewer renders the previous
   document's image positions. `CommonMarkCache` has no general `clear()` —
   only `clear_scrollable()` — so replacing the whole value is the way.
5. **`rfd` blocks the render loop** while the dialog is open. Fine for a
   single-user tool — the dialog is modal anyway. Move it to a thread with an
   `mpsc` channel if that ever becomes annoying.
6. **Font hinting is on by default in epaint 0.35** (`TextOptions::font_hinting`,
   plus `subpixel_binning`), so text is noticeably crisper than it was in older
   egui releases. There is still no DirectWrite/ClearType subpixel rendering.
7. **Multi-monitor DPI drift** when dragging between displays with different
   scale factors — [winit#4041](https://github.com/rust-windowing/winit/issues/4041),
   not egui-specific.
8. **egui 0.35: `App::update` is gone.** The trait now requires
   `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`. `ui.ctx()` gets the
   `Context`. There is an optional `fn logic(&mut self, ctx, frame)` for
   non-drawing work.
9. **egui 0.35: `TopBottomPanel` and `SidePanel` are gone.** Replaced by a
   unified `egui::Panel` with `Panel::top/bottom/left/right(id)`. `show` takes
   `ui: &mut Ui`, not `ctx`. `show_inside` is deprecated in favour of `show`.
10. **egui 0.35: panel sizing methods are `*_size`, not `*_width`/`*_height`.**
    `default_size`, `min_size`, `max_size`, `exact_size`, `size_range` — one set
    for all four orientations.
11. **Borrow conflict with `toggle_value`.** Rust evaluates function arguments
    left-to-right, so `ui.toggle_value(&mut self.editing, if self.editing { … })`
    raises E0503. Hoist the label to a `let` first. The same shape bites
    `selectable_value` and any `Button::selectable(self.x == y, …)`; read the
    field into a local before the call.
12. **egui 0.35: there is no `Context::set_style` or `Context::style_mut`.** Use
    `all_styles_mut` (both themes), `style_mut_of(theme, …)` / `set_style_of`,
    or `global_style_mut` (current theme only). `Ui::style_mut` still exists and
    scopes to that subtree, which is how the chrome text scale is applied.
13. **egui 0.35 renamed `Rounding` to `CornerRadius`,** whose fields are `u8`
    (`CornerRadius::same(5)`), and `Margin`'s fields are `i8`
    (`Margin::symmetric(12, 8)`). `Button::rounding` is now
    `Button::corner_radius`.
14. **egui 0.35: `Painter::rect_stroke` takes a fourth `StrokeKind` argument.**
15. **`Context::consume_shortcut` does not exist.** Go through
    `ctx.input_mut(|i| i.consume_shortcut(&sc))`. Test the most specific
    shortcut first — matching ignores a surplus Shift, so `Ctrl+Shift+S` must be
    checked before `Ctrl+S`.
16. **Dropped files live on `InputState::raw`,** not on `InputState` itself:
    `ctx.input(|i| i.raw.dropped_files.clone())`, and `i.raw.hovered_files` for
    the drag-over state. On Windows only `DroppedFile::path` is populated.
17. **`ScrollArea::id_source` is gone; use `id_salt`.** Two scroll areas that
    are never shown at the same time still need distinct salts if their content
    height differs, or the scroll offset carries across.
18. **A centred layout centres each row of a galley individually.** A
    multi-line label inside `vertical_centered` will not keep its columns
    aligned; allocate a fixed-width `Layout::top_down(Align::LEFT)` child and
    put the label in that.
19. **egui 0.35 tooltips can be promoted to their own OS viewport.** Harmless in
    use, but it means external tooling that grabs the process's "main window"
    by title can momentarily latch onto a tooltip-sized window.

## Build status

Verified 2026-07-28 on Windows 11, `stable-x86_64-pc-windows-msvc`, cargo 1.96.1,
2560×1600 display at 200 % scaling:

- `cargo check` — clean, **zero warnings** (checked after `touch src/main.rs`, so
  not a cached result)
- `cargo build` — succeeded
- `cargo run -- test.md` — window opened, no panic, empty stderr

Exercised against `test.md` by driving the real window with synthetic input and
capturing the framebuffer:

- All three modes (View / Edit / Split), both themes, and the theme switcher
- The reading measure centring, and the split divider
- Editing: dirty marker in the toolbar, status bar and title bar; live word /
  character / line counts
- `Ctrl+S` — confirmed by diffing the file on disk before and after
- `Ctrl+O` with unsaved edits — the modal prompt appears; `Esc` cancels it and
  leaves the buffer intact
- Rendered output: headings, bold/italic, inline code, GFM table with zebra
  striping, task list, syntax-highlighted code blocks in **both** themes,
  blockquote, nested list

**Not verified:** drag and drop and the drag-over overlay (a real shell drag
cannot be synthesised from a script — the code path is written against
`InputState::raw.dropped_files` / `hovered_files` but has not been exercised);
the close-with-unsaved-changes guard; Save As and the Open file dialog, since
`rfd` opens a native modal that blocks the harness; and the release profile.

## Ideas

- File watching for live reload — `notify` 8.2 plus `notify-debouncer-full`
  (editors write a file in several operations, so a single save fires 3–5 raw
  events)
- Recent-files list
- Export to HTML — `comrak::markdown_to_html`
- Synchronised scrolling between the split panes
- Trim `syntect` with `default-features = false` and shipped `.packdump`
  assets if binary size becomes a concern
