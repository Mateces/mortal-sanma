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
pub fn canonical_action_space() -> u32 {
    consts::CANONICAL_ACTION_SPACE as u32
}

/// Remap a canonical 47-dim mask down to the trained 3p 44-dim mask the DQN
/// consumes. canonical 0-38 -> trained 0-38; canonical 39/40/41 (chi) dropped;
/// canonical 42(pon)->39, 43(kan)->40, 44(agari)->41, 45(ryukyoku)->42,
/// 46(pass)->43.
#[wasm_bindgen]
pub fn remap_mask_canonical_to_model(mask47: &[u8], version: u32) -> Result<Vec<u8>, String> {
    if mask47.len() != consts::CANONICAL_ACTION_SPACE {
        return Err(format!(
            "mask len {} != canonical action space {}",
            mask47.len(),
            consts::CANONICAL_ACTION_SPACE
        ));
    }
    // TODO(phase-3/4p): when a 4-player libriichi variant exists, branch on
    // `version`/player-count and apply the 4p trained(46-dim) remap instead.
    // This crate is 3-player sanma only, so every version uses the 3p remap.
    let _ = version;
    let mut out = vec![0u8; consts::ACTION_SPACE];
    out[..=38].copy_from_slice(&mask47[..=38]);
    out[39] = mask47[42]; // pon
    out[40] = mask47[43]; // kan
    out[41] = mask47[44]; // agari
    out[42] = mask47[45]; // ryukyoku
    out[43] = mask47[46]; // pass
    Ok(out)
}

/// Interpret a trained 3p model output index (0..44) as a canonical index
/// (0..47). trained 0-38 -> canonical 0-38; trained 39->42, 40->43, 41->44,
/// 42->45, 43->46.
#[wasm_bindgen]
pub fn interpret_model_idx(model_idx: u32, version: u32) -> Result<u32, String> {
    // TODO(phase-3/4p): branch on version/player-count for the 4p (46-dim) map.
    let _ = version;
    let i = model_idx as usize;
    if i >= consts::ACTION_SPACE {
        return Err(format!(
            "model_idx {i} out of range (0..{})",
            consts::ACTION_SPACE
        ));
    }
    let canonical = if i <= 38 { i } else { 42 + (i - 39) };
    Ok(canonical as u32)
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

    /// Encode the action mask in the canonical 47-dim layout
    /// (forward-compatible; remaps the internal trained 44-dim mask up to 47).
    pub fn encode_obs_mask_canonical(&self, version: u32, at_kan_select: bool) -> Vec<u8> {
        let (_obs, mask) = self.inner.encode_obs(version, at_kan_select);
        let mut out = vec![0u8; consts::CANONICAL_ACTION_SPACE];
        for i in 0..=38usize {
            out[i] = mask[i] as u8;
        }
        out[42] = mask[39] as u8; // pon
        out[43] = mask[40] as u8; // kan
        out[44] = mask[41] as u8; // agari
        out[45] = mask[42] as u8; // ryukyoku
        out[46] = mask[43] as u8; // pass
        out
    }
}

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("{info}");
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_action_space_is_47() {
        assert_eq!(consts::CANONICAL_ACTION_SPACE, 47);
        assert_eq!(canonical_action_space(), 47);
    }

    #[test]
    fn remap_canonical_to_model_identity_region() {
        let mut canon = vec![0u8; 47];
        for &c in &[0usize, 5, 33, 34, 35, 36, 37, 38] {
            canon[c] = 1;
        }
        let model = remap_mask_canonical_to_model(&canon, 5).unwrap();
        assert_eq!(model.len(), 44);
        for &c in &[0usize, 5, 33, 34, 35, 36, 37, 38] {
            assert_eq!(model[c], 1, "canonical {c} must map to trained {c}");
        }
    }

    #[test]
    fn remap_canonical_to_model_shifted_region() {
        let mut canon = vec![0u8; 47];
        for &c in &[42usize, 43, 44, 45, 46] {
            canon[c] = 1;
        }
        let model = remap_mask_canonical_to_model(&canon, 5).unwrap();
        assert_eq!(model[39], 1); // pon  <- canonical 42
        assert_eq!(model[40], 1); // kan  <- 43
        assert_eq!(model[41], 1); // agari<- 44
        assert_eq!(model[42], 1); // ryukyoku <- 45
        assert_eq!(model[43], 1); // pass <- 46
    }

    #[test]
    fn remap_drops_chi_slots() {
        let mut canon = vec![0u8; 47];
        canon[39] = 1; // chi_low
        canon[40] = 1; // chi_mid
        canon[41] = 1; // chi_high
        let model = remap_mask_canonical_to_model(&canon, 5).unwrap();
        assert!(model.iter().all(|&b| b == 0), "chi must map to nothing in 3p");
    }

    #[test]
    fn remap_rejects_wrong_length() {
        assert!(remap_mask_canonical_to_model(&[0u8; 44], 5).is_err());
        assert!(remap_mask_canonical_to_model(&[0u8; 48], 5).is_err());
    }

    #[test]
    fn interpret_model_idx_identity_and_shift() {
        for t in 0..=38u32 {
            assert_eq!(interpret_model_idx(t, 5).unwrap(), t);
        }
        assert_eq!(interpret_model_idx(39, 5).unwrap(), 42); // pon
        assert_eq!(interpret_model_idx(40, 5).unwrap(), 43); // kan
        assert_eq!(interpret_model_idx(41, 5).unwrap(), 44); // agari
        assert_eq!(interpret_model_idx(42, 5).unwrap(), 45); // ryukyoku
        assert_eq!(interpret_model_idx(43, 5).unwrap(), 46); // pass
    }

    #[test]
    fn interpret_model_idx_rejects_out_of_range() {
        assert!(interpret_model_idx(44, 5).is_err());
    }

    #[test]
    fn round_trip_canonical_to_model_to_canonical() {
        // For every set of canonical actions that have a trained slot (all but chi),
        // remap down then interpret the argmax back: identity must hold.
        let mut canon = vec![0u8; 47];
        for &c in &[3usize, 34, 37, 38, 42, 43, 44, 45, 46] {
            canon[c] = 1;
        }
        let model = remap_mask_canonical_to_model(&canon, 5).unwrap();
        for mi in 0..44 {
            if model[mi] == 1 {
                let back = interpret_model_idx(mi as u32, 5).unwrap() as usize;
                assert_eq!(canon[back], 1, "trained {mi} -> canonical {back} not set");
            }
        }
    }
}
