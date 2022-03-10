# Struct Attributes

The struct attributes affect only the struct and are introduced before it:

```rust
#[attribute]
struct Name {
// ...
}
```

## Attributes

- `#[<model>_attr(...)]`: adds an attribute only for `model`.
    - `db`: Database model.
- `#[skip_default]`: disables the generation of the default derives.
