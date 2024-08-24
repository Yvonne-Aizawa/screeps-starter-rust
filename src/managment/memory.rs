use js_sys::{JsString, Object, Reflect};
use log::{info, trace, error};
use screeps::game;
use wasm_bindgen::JsCast as _;
use std::collections::HashSet;
pub fn memory_tick() {
    match clean_memory() {
        Err(e) => {
            error!("error cleaning memory: {e}")
        }
        Ok(_) => {
            info!("cleaned memory")
        }
    }
}
pub fn clean_memory() -> Result<(), Box<dyn std::error::Error>> {
    info!("running memory cleanup");
    let mut alive_creeps = HashSet::new();
    // add all living creep names to a hashset
    for creep_name in game::creeps().keys() {
        let res = alive_creeps.insert(creep_name.clone());
        match res {
            true => {
                trace!("added {creep_name} to alive_creeps");
            }
            false => {
                error!("for some reason adding {creep_name} to a hashset failed");
            }
        };
    }

    // grab `Memory.creeps` (if it exists)
    if let Ok(memory_creeps) = Reflect::get(&screeps::memory::ROOT, &JsString::from("creeps")) {
        // convert from JsValue to Object
        let memory_creeps: Object = memory_creeps.unchecked_into();
        // iterate memory creeps
        for creep_name_js in Object::keys(&memory_creeps).iter() {
            // convert to String (after converting to JsString)
            let creep_name = String::from(creep_name_js.dyn_ref::<JsString>().unwrap());

            // check the HashSet for the creep name, deleting if not alive
            if !alive_creeps.contains(&creep_name) {
                info!("deleting memory for dead creep {}", creep_name);
                let _ = Reflect::delete_property(&memory_creeps, &creep_name_js);
            }
        }
    };

    Ok(())
}
