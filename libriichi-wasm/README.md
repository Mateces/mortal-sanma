# libriichi-wasm

WebAssembly port of [kiraliu7/MortalSanma](https://github.com/kiraliu7/MortalSanma)'s
`libriichi` Rust crate so that TypeScript code can use the Mortal riichi rules
engine without going through a Python/PyO3 bridge.

License: AGPL-3.0-or-later (upstream's license).

## What's in this Phase 1a

| Module | Source | Notes |
|---|---|---|
| `tile` | direct copy | tile enum, red dora, str ↔ id |
| `hand` | direct copy | hand operations |
| `chi_type` | direct copy | chi variant enum |
| `macros` | direct copy | crate-internal tile macros |
| `rankings` | direct copy | rank permutations |
| `vec_ops` | direct copy | small numeric helpers |
| `algo::shanten` | direct copy + data/*.bin.gz | shanten table |
| `algo::agari` | direct copy + data/agari.bin.gz | winning-hand decomposer |
| `algo::point` | direct copy | scoring (han/fu/payment) |
| `mjai::event` | direct copy | MJAI event types (serde-derived) |

These files are all pyo3-clean in upstream — they compiled to wasm32 without
modification. Phase 1b will add `state/*` (player state machine), which
requires cfg-gating `#[pyclass]` attributes on ~26 occurrences.

## Build

```bash
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target nodejs --out-dir pkg \
    target/wasm32-unknown-unknown/release/libriichi_wasm.wasm
```

Then from Node:

```js
const w = require('./pkg/libriichi_wasm');
w.tile_id_to_string(34);   // "5mr"
w.shanten([0,0,9,10,11,12,13,14,15,16,17,18,19]);  // 0 (tenpai)
w.mjai_event_from_json('{"type":"dahai","actor":0,"pai":"5mr","tsumogiri":true}');
```

## Toolchain prerequisites

- Rust with `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` (`cargo install wasm-bindgen-cli`)
- Node.js for running the JS-loaded module
