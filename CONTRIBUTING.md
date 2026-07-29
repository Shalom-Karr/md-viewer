# Contributing

Bug reports, fixes, and focused features are welcome.

Note the licence before you start: **AGPL-3.0 with the Commons Clause**. It is
source-available, not open source. Contributions are accepted under the same
terms, and the Commons Clause means neither you nor anyone else may sell this
or fold it into a paid product.

## Setup

You need the **MSVC** toolchain on Windows. The GNU toolchain has open compiler
bugs with egui ([rust-lang/rust#140237](https://github.com/rust-lang/rust/issues/140237)),
so `windows-gnu` is not supported.

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
winget install Microsoft.VisualStudio.2022.BuildTools   # select "C++ build tools"
```

Exclude the build directories from your antivirus before the first build. A
cold `cargo build` unpacks and compiles tens of thousands of small files, and
real-time scanning turns that into an I/O stall severe enough to hang the
machine. On Windows, in an elevated shell:

```powershell
Add-MpPreference -ExclusionPath "$env:USERPROFILE\.cargo"
Add-MpPreference -ExclusionPath "$(Get-Location)"
```

```powershell
cargo run              # debug; console window stays visible for println!
cargo build --release  # release; console suppressed
```

Roughly 100 s for a cold debug build, ~14 min for release (thin LTO and
`opt-level = "s"`). Incremental debug builds are seconds.

## Before opening a PR

```powershell
cargo fmt
cargo clippy -- -D warnings
cargo check
```

CI runs all three on every push and pull request, so anything failing locally
fails there too.

## House style

- Match the surrounding code. No new dependencies without a clear reason —
  the dependency tree is deliberately small and the binary ships as one file.
- Comments only where the *why* is not obvious. The code says what it does.
- Keep changes surgical. A bug fix is a bug fix, not a refactor.
- No `unsafe`, and no `RefCell` or full-document `clone()` to sidestep a borrow
  error. If the borrow checker objects, restructure.

## Working with egui 0.35

egui changed substantially in 0.35 and most tutorials, LLM output, and Stack
Overflow answers target older versions. **Read the vendored crate source**
rather than trusting memory:

```
~/.cargo/registry/src/index.crates.io-*/egui-0.35.0/
~/.cargo/registry/src/index.crates.io-*/eframe-0.35.0/
```

Each crate ships an `examples/` directory that shows current idiomatic usage.

The traps that cost the most time are listed in [docs/egui-0.35-notes.md](docs/egui-0.35-notes.md)
and in the README's Gotchas section. Read those first — several are silent
failures rather than compiler errors.

## Rendering limits

The markdown rendering is done by `egui_commonmark`, which hardcodes a number
of its visual decisions. Several styling details are **not reachable** without
patching that crate — code-block padding, table header emphasis, heading size
ratios, blockquote background fill, and image centring among them. They are
enumerated in [docs/architecture.md](docs/architecture.md).

If you want to change one of those, say so in the issue first. Forking a
dependency is a maintenance commitment, not a patch.

## Reporting bugs

Include your Windows version, whether you used the released binary or built
from source, and the exact steps. For rendering problems, attach the markdown
that triggers it — a file that reproduces the issue is worth more than a
screenshot.
