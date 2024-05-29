use std::collections::HashMap;

use gloo_utils::format::JsValueSerdeExt;
use serde::{Deserialize, Serialize};
use serde_json::Error;
use wasm_bindgen::JsValue;

use crate::structs::creep::CreepMemory;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GlobalMemory {
    pub creeps: std::collections::HashMap<String, CreepMemory>, //other vars not implemented yet
}
#[allow(dead_code)]
impl GlobalMemory {
    fn get_creeps(&self) -> HashMap<String, CreepMemory> {
        self.creeps.clone()
    }
    fn get() -> Result<GlobalMemory, Error> {
        let json_var: &JsValue = screeps::memory::ROOT.as_ref();
        json_var.into_serde()
    }
}
