# Test Document

A file for manually verifying md-viewer renders all common Markdown constructs.

## Text formatting

This paragraph has **bold text**, *italic text*, and ***bold-italic text***.
Inline `code` also works.

## GFM table

| Language | Paradigm   | Year |
| :------- | :--------- | ---: |
| Rust     | Systems    | 2015 |
| Python   | Multi      | 1991 |
| Haskell  | Functional | 1990 |

## Task list

- [x] cargo check clean
- [x] cargo build succeeded
- [x] app launched without panic
- [ ] manual render verified in the viewer

## Fenced code block

```rust
fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
```

```python
def greet(name: str) -> str:
    return f"Hello, {name}!"
```

## Blockquote

> "Programs must be written for people to read, and only incidentally
> for machines to execute."
> — Harold Abelson

## Nested list

1. Open the file
   - Click **Open…**
   - Pick `test.md`
2. Toggle edit mode with the **Edit / View** button
3. Use the theme buttons (System / Dark / Light) in the top-right corner
