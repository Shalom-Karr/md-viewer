# egui 0.35 API notes

egui 0.35 changed enough that most tutorials, Stack Overflow answers, and LLM
output target an older API and will not compile. These were all confirmed
against the vendored crate source while building this app.

**Read the source rather than trusting memory:**

```
~/.cargo/registry/src/index.crates.io-*/egui-0.35.0/
~/.cargo/registry/src/index.crates.io-*/eframe-0.35.0/
~/.cargo/registry/src/index.crates.io-*/egui_commonmark-0.24.0/
```

Each ships an `examples/` directory showing current idiomatic usage.

## Breaking changes

**`App::update` is gone.** The trait now requires:

```rust
fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame);
```

The app receives a `Ui`, not a `Context`. Get the context with `ui.ctx()`. An
optional `fn logic(&mut self, ctx: &egui::Context, frame: &mut Frame)` handles
non-drawing work.

**`TopBottomPanel` and `SidePanel` no longer exist.** They are replaced by a
unified `egui::Panel`:

```rust
egui::Panel::top("toolbar").show(ui, |ui| { … });   // also ::bottom ::left ::right
```

`show` takes `ui: &mut Ui`, not `ctx: &Context`. `CentralPanel` still exists,
still derives `Default`, and its `show` also takes a `Ui`. `show_inside` is
deprecated in favour of `show`.

**Panel sizing** is `default_size` / `min_size` / `max_size` / `exact_size` —
not the old `*_width` / `*_height` pairs.

**No `Context::set_style` or `style_mut`.** Use `all_styles_mut`,
`style_mut_of(theme, …)`, or `global_style_mut`. Styles are per-theme and
persistent, so installing them once at startup survives theme switches.

**Renames:**

| Old | New |
| :-- | :-- |
| `Rounding` | `CornerRadius` (`u8` fields) |
| `Button::rounding` | `Button::corner_radius` |
| `ScrollArea::id_source` | `ScrollArea::id_salt` |

`Margin` fields are now `i8`. `Painter::rect_stroke` takes a fourth
`StrokeKind` argument.

**Shortcuts:** there is no `Context::consume_shortcut`. Use
`ctx.input_mut(|i| i.consume_shortcut(&sc))`. Matching ignores surplus
modifiers, so **`Ctrl+Shift+S` must be tested before `Ctrl+S`** or the plain
binding swallows it.

**Dropped files** live on `InputState::raw`, not `InputState`.

**`egui::Modal`** exists: `egui::Modal::new(id).show(ctx, …)`. Note it takes a
`&Context`, not a `Ui`. Its `should_close()` covers both backdrop clicks and
Escape.

## Silent failures

These produce no compiler error and no runtime warning.

**Images render as broken boxes** unless `egui_extras::install_image_loaders(&cc.egui_ctx)`
is called once at startup.

**Stale markdown layout.** `CommonMarkCache` holds per-document layout. Reset it
on file load *and* on every content change, or the viewer draws the previous
document's image positions.

**Bold is invisible.** `strong()` resolves to `widgets.active.fg_stroke.color`.
If body text is already at full contrast, bold and headings look identical to
normal text. Set a muted body colour and a high-contrast strong colour.

**Zebra striping looks disabled.** `egui_commonmark` sets `Grid::striped(true)`,
but egui takes the stripe colour from `faint_bg_color`, whose default is close
to invisible. Set it explicitly per theme.

**Centred multi-line labels lose alignment.** A centred layout centres each
galley row individually, so a multi-line label inside `vertical_centered` has
every line independently centred rather than sharing a left edge. Use a
fixed-width child with `Align::LEFT`.

**Version lockstep.** `egui_commonmark` must match `egui` exactly. A mismatch
surfaces as conflicting trait-impl linker errors, not a readable version
complaint.

## Borrow-checker traps

Rust evaluates function arguments left to right, so this raises **E0503**:

```rust
ui.toggle_value(&mut self.editing, if self.editing { "View" } else { "Edit" });
```

The mutable borrow in the first argument is still live when the second reads the
same field. Hoist it:

```rust
let label = if self.editing { "View" } else { "Edit" };
ui.toggle_value(&mut self.editing, label);
```

Nested closures passed to `Panel::show` are generally fine — Rust 2021 disjoint
capture handles sequential closures touching different fields of `self`.

## Corrections to older advice

**Text is no longer noticeably soft.** epaint 0.35 enables `font_hinting` and
`subpixel_binning` by default. Guidance about egui text looking blurry next to
native apps predates this.

**Tooltips can be promoted to their own OS viewport**, which will confuse
screen-capture tooling that assumes one window.

## Still open upstream

**Multi-monitor DPI drift** when dragging between displays with different scale
factors — [winit#4041](https://github.com/rust-windowing/winit/issues/4041).
Single-monitor and same-DPI setups are unaffected.

**Windows theme changes can be missed.** winit surfaces
`WindowEvent::ThemeChanged`, but `WM_SETTINGCHANGE` is sometimes dropped
([winit#4161](https://github.com/rust-windowing/winit/issues/4161)). Re-query the
registry on each notification rather than trusting the event payload.
