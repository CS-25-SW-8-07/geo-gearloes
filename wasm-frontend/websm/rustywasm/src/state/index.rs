use comms::{Bytes, Parquet};
use rusty_roads::road::Roads;
use wasm_bindgen::prelude::*;

use super::StateHandle;

#[wasm_bindgen]
pub fn insert_into_index(bytes: &[u8], state: StateHandle) -> Result<(), JsValue> {
    let roads = Roads::from_parquet(Bytes::copy_from_slice(bytes))
        .map_err(|x| JsValue::from_str(format!("{x:?}").as_str().into()))?;

    for (id, geom) in roads.id.iter().zip(roads.geom.iter()) {
        state.road_index.lock().unwrap().insert(*id, geom.clone());
    }

    Ok(())
}
