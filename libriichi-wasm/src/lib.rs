pub mod consts;
pub mod tile;
pub mod array;
pub mod hand;
pub mod chi_type;
mod macros;
pub mod rankings;
pub mod vec_ops;
pub mod algo;
pub mod mjai;
pub mod state;

use std::str::FromStr;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn tile_id_to_string(id: u32) -> Result<String, JsError> {
    let t = tile::Tile::try_from(id as usize)
        .map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(t.to_string())
}

#[wasm_bindgen]
pub fn tile_string_to_id(s: &str) -> Result<u32, JsError> {
    let t = tile::Tile::from_str(s)
        .map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(t.as_u8() as u32)
}

#[wasm_bindgen]
pub fn action_space() -> u32 {
    consts::ACTION_SPACE as u32
}

#[wasm_bindgen]
pub fn obs_shape(version: u32) -> Result<Vec<u32>, JsError> {
    let s = consts::obs_shape(version);
    Ok(vec![s.0 as u32, s.1 as u32])
}

#[wasm_bindgen]
pub fn shanten(tile_ids: &[u32]) -> Result<i32, JsError> {
    let mut hand_arr = [0u8; 34];
    for &id in tile_ids {
        if id >= 34 { return Err(JsError::new("tile id must be 0..34")); }
        hand_arr[id as usize] += 1;
    }
    Ok(algo::shanten::calc_all(&hand_arr, hand_arr.iter().sum::<u8>() / 3) as i32)
}

#[wasm_bindgen]
pub fn mjai_event_from_json(json: &str) -> Result<JsValue, JsError> {
    let e: mjai::Event = serde_json::from_str(json)
        .map_err(|e| JsError::new(&format!("parse: {e}")))?;
    serde_wasm_bindgen::to_value(&e).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub struct PlayerStateHandle {
    inner: state::PlayerState,
}

#[wasm_bindgen]
impl PlayerStateHandle {
    #[wasm_bindgen(constructor)]
    pub fn new(player_id: u32) -> Result<PlayerStateHandle, JsError> {
        if player_id >= 3 {
            return Err(JsError::new("player_id must be 0..3"));
        }
        Ok(PlayerStateHandle {
            inner: state::PlayerState::new(player_id as u8),
        })
    }

    pub fn apply_event(&mut self, mjai_json: &str) -> Result<String, JsError> {
        let event: mjai::Event = serde_json::from_str(mjai_json)
            .map_err(|e| JsError::new(&format!("parse event: {e}")))?;
        let action = self.inner.update(&event)
            .map_err(|e| JsError::new(&format!("update: {e}")))?;
        serde_json::to_string(&action)
            .map_err(|e| JsError::new(&format!("serialize action: {e}")))
    }

    pub fn brief_info(&self) -> String {
        self.inner.brief_info()
    }

    pub fn encode_obs(&self, version: u32, at_kan_select: bool) -> Vec<f32> {
        let (obs, _mask) = self.inner.encode_obs(version, at_kan_select);
        obs.into_raw_vec_and_offset().0
    }

    pub fn encode_obs_mask(&self, version: u32, at_kan_select: bool) -> Vec<u8> {
        let (_obs, mask) = self.inner.encode_obs(version, at_kan_select);
        mask.into_raw_vec_and_offset().0.into_iter().map(|b| b as u8).collect()
    }
}

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("{info}");
    }));
}
