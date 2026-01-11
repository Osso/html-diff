# html-diff

Semantic HTML diff tool that compares DOM structure, ignoring cosmetic differences like whitespace and entity encoding.

## Features

- **Whitespace normalization**: Ignores indentation, newlines, and extra spaces
- **Entity decoding**: Treats `&#039;`, `&#x27;`, and `'` as equivalent
- **Attribute ordering**: `class="a" id="b"` equals `id="b" class="a"`
- **Selector-based ignoring**: Skip dynamic elements with `-i "#stats"` or `-i ".timestamp"`
- **Colored diff output**: Red for deletions, green for additions

## Installation

```bash
cargo install --git https://github.com/Osso/html-diff
```

## Usage

```bash
# Basic comparison
html-diff file1.html file2.html

# With more context lines
html-diff -C 5 file1.html file2.html

# Ignore specific elements
html-diff -i "#dynamic-stats" -i ".timestamp" file1.html file2.html
```

## Example

These two HTML files are considered **equal**:

```html
<!-- file1.html -->
<html>
  <body>
    <p class="greeting" id="main">It&#039;s a test</p>
  </body>
</html>
```

```html
<!-- file2.html -->
<html><body><p id="main" class="greeting">It&#x27;s a test</p></body></html>
```

## Use Cases

- Comparing server-side rendered HTML across framework migrations
- Verifying HTML output parity between implementations
- Testing template rendering consistency
