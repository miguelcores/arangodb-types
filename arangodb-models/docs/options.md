# Macro Options

The macro options are introduced at the beginning using the pattern:

```rust
#![attribute]
```

## Options

- `#![relative_imports]`: makes the imports be relative instead of absolute in the models.
- `#![build_<model>]`: generates a new model named `model` that basically removes the serde renames.
- `#![skip_impl]`: disables the generation of the database impls.
- `#![skip_fields]`: disables the generation of the database field enum for the model.
- `#![sync_level = "<level>"]`: enables the synchronization of the model or the collection. The values are:
    - `document`: only synchronizes at document level.
    - `collection`: only synchronizes at collection level.
    - `document_and_collection`: synchronizes at both document and collection levels.
- `#![sync_collection_key_method = ".."]`: replaces the default name for the method used to get the key of the document
  that represents the collection in DB.
- `#![collection_name = ".."]`: replaces the default name for the collection.
- `#![collection_type = ".."]`: replaces the default `CollectionKind` enum by another one.
- `#![collection_kind = ".."]`: replaces the default name for `CollectionKind` enum.
