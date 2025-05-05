# winnow-regex

[![crates.io](https://img.shields.io/crates/v/winnow-regex.svg)](https://crates.io/crates/winnow-regex)
[![docs.rs](https://docs.rs/winnow-regex/badge.svg)](https://docs.rs/winnow-regex)

A set of [`winnow`](https://crates.io/crates/winnow) parsers backed by the [`regex`](https://crates.io/crates/regex) crate.  
Provides two generic parsers:

- `regex(pattern)` – match a slice against a regular expression from the beginning.
- `captures(pattern)` – match and return captured groups.

Both parsers support complete‑and‑partial streaming via the `StreamIsPartial` trait.

## Quick Start

```rust
use winnow::prelude::*;
use winnow_regex::regex;

fn digits<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
    // matches one or more digits at the front
    regex(r"^\d+").parse_next(input)
}

assert_eq!(digits.parse_peek("42abc"), Ok(("abc", "42")));
assert!(digits.parse_peek("abc42").is_err());
```

### Captures Example
```rust
use winnow::prelude::*;
use winnow_regex::{captures, Captures};

fn dims<'i>(input: &mut &'i str) -> ModalResult<(i32, i32)> {
    // captures two number groups: width and height
    captures(r"^(\d+)x(\d+)")
        .map(|caps: Captures<_>| {
            let w: i32 = caps[1].parse().unwrap();
            let h: i32 = caps[2].parse().unwrap();
            (w, h)
        })
        .parse_next(input)
}

assert_eq!(dims.parse_peek("800x600rest"), Ok(("rest", (800, 600))));
```

