# Field Attributes

The field attributes affect only the field itself and are introduced before it:

```rust
struct Name {
    #[attribute]
    field: Type
}
```

## Attributes

- `#[<model>_attr(...)]`: adds an attribute only for the `model`'s field.
    - `db`: Database model.
- `#[skip_in_<model>]`: does not include this field into the `model`.
    - `db`: Database model.
- `#[db_name = ".."]`: renames this field using serde in the database model.
- `#[inner_model = "<value>"]`: specifies the inner model. Values are:
    - `data`: Not a model.
    - `struct`: A struct-like model.
    - `enum`: An enum-like model.
- `#[inner_type_<model> = ".."]`: specifies the name of the inner type for the `model`. This is used when the sub-model
  changes between models. Ignores the `db` model.