# Architecture

Single binary, single source file, no runtime dependencies. `src/main.rs` holds
everything: state, input handling, layout, and styling.

## Stack

| Crate | Version | Role |
| :--- | :--- | :--- |
| `eframe` / `egui` | 0.35 | Immediate-mode GUI, native, no WebView2 |
| `egui_commonmark` | 0.24 | Markdown rendering |
| `egui_extras` | 0.35 | Image loading |
| `rfd` | 0.17 | Native file dialogs (`IFileOpenDialog` on Windows) |

`egui_commonmark` uses `pulldown-cmark` to parse and `syntect` to highlight
fenced code blocks.

## Why egui

Two alternatives were weighed seriously:

- **iced** — nicer default aesthetics and a real theme system, but a 4–8 minute
  cold build against egui's 20–40 s, one maintainer, and no ready-made markdown
  renderer. Every element would have been hand-built, including image loading
  and caching.
- **Tauri** — would mean writing the editor in HTML/CSS/JS with Rust only as a
  shell, plus a WebView2 dependency. That is a different project.

`egui_commonmark` is the only ready-made markdown renderer in the Rust GUI
ecosystem, which effectively settled it.

**glow, not wgpu.** `eframe` defaults to the wgpu backend, which pulls in the
naga shader compiler and roughly doubles cold build time. OpenGL is fine on all
Windows 10/11 hardware.

## Immediate mode

The entire UI is rebuilt every frame. This is why the view/edit toggle is a
`bool` and an `if` rather than a state machine, and why there is no widget tree
to keep in sync. The cost is that anything expensive must be cached explicitly —
word and character counts are recomputed on change, not per frame.

`eframe::App` in 0.35 requires `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`.
There is no `update` method; the app receives a `Ui`, not a `Context`. An
optional `fn logic(&mut self, ctx, frame)` exists for non-drawing work.

## Reading measure

Rendered markdown is capped at **760 px** and centred; the editor uses **920 px**
because source lines run longer. Split panes pass `f32::INFINITY` to fill.

The mechanism: a `column()` helper centres a `set_max_width` child `Ui` inside a
`horizontal_top` row. `egui_commonmark` derives its wrap width from
`ui.available_width()`, so constraining the parent constrains the text.

## Typography

Heading 30 / Body 16 / Monospace 14 / Button 14 / Small 12. Chrome panels scale
Body down to 13 via a subtree `ui.style_mut()`.

Two egui-specific constraints shaped this:

**There is no global line-height.** `TextFormat::line_height` is per-run, and
`FontTweak::scale` shrinks the row box along with the glyphs, so it buys
nothing. What works: `egui_commonmark` lays a paragraph out as a wrapping row of
individual labels, so `spacing.item_spacing.y` *is* the leading control. Set
to 5.0.

**Bold is a colour, not a weight.** `strong()` resolves to
`widgets.active.fg_stroke.color`. Both themes therefore set a muted body colour
against a high-contrast strong colour — without that, `**bold**` and headings
do not read as emphasis at all.

## Theming

`egui::global_theme_preference_buttons(ui)` renders the System/Dark/Light
control and reads and writes the `Context`'s `ThemePreference` directly, so the
app struct holds no theme state. `ThemePreference::System` is the default and
follows the OS through winit.

Do **not** call `ctx.set_theme(...)` inside the frame loop — it runs every frame
and overrides system detection, breaking the toggle.

Styles in 0.35 are per-theme and persistent, installed once at startup via
`all_styles_mut` / `style_mut_of`. There is no `Context::set_style`.

Syntax highlighting follows the app theme automatically: `egui_commonmark`
re-reads `visuals.dark_mode` every frame to choose between `syntax_theme_dark`
and `syntax_theme_light`. Dark uses `base16-eighties.dark`, whose neutral grey
sits better against the page than the default's blue cast.

## Rendering limits

`egui_commonmark` owns the render loop and hardcodes several visual decisions.
Reachable through configuration:

- Table zebra striping (the crate sets `Grid::striped(true)`; egui takes the
  colour from `faint_bg_color`, whose default is effectively invisible)
- Inline-code chip, code-block border and corner radius
- Horizontal-rule stroke, bullets, blockquote bar, checkboxes
- Link colour and `url_in_tooltip`
- Image width clamped to the measure

**Not reachable without patching the crate:**

- Code-block padding — a `TextEdit` left at egui's default `Margin::symmetric(4,2)`,
  with no `Frame`. Also no language label, and the copy button cannot be removed.
- Blockquote background fill — bar width (3 px) and left margin (10 px) are literals.
- Table header emphasis, cell padding, column rules — the header uses the same
  code path as body rows, and `item_spacing.x` is forced to 0 document-wide.
- Asymmetric heading spacing and an h1/h2 hairline — the crate emits the same
  `ui.label("\n")` before and after every heading.
- Heading size *ratios* — fixed coefficients (0.835, 0.668, …), so H1→H2 is
  inherently flat.
- Image centring and rounding.

Changing any of these means vendoring and patching `egui_commonmark`, which is
a maintenance commitment rather than a patch.

## Build profile

```toml
[profile.release]
strip = true
lto = "thin"
opt-level = "s"
```

Optimised for size — this is a document viewer, not a hot loop. Release builds
take ~14 minutes as a result; debug builds are ~100 s cold and seconds
incrementally.

`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` must remain
the first line of `main.rs`. Without it, release builds spawn a console window
behind the app. The `cfg_attr` form keeps the console in debug so `println!`
still works.
