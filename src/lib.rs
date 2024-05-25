use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::Display,
};

use gloo_utils::format::JsValueSerdeExt;
use js_sys::{JsString, Object, Reflect};
use log::*;
use screeps::{
    constants::{ErrorCode, Part, ResourceType},
    enums::StructureObject,
    find, game,
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    prelude::*,
    raw_memory,
};
use serde::{Deserialize, Serialize};
use serde_json::{self, Error};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::console::warn;

mod creeps;
mod logging;

// this is one way to persist data between ticks within Rust's memory, as opposed to
// keeping state in memory on game objects - but will be lost on global resets!
thread_local! {
    static CREEP_TARGETS: RefCell<HashMap<String, CreepTarget>> = RefCell::new(HashMap::new());
}

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

// this enum will represent a creep's lock on a specific target object, storing a js reference
// to the object id so that we can grab a fresh reference to the object each successive tick,
// since screeps game objects become 'stale' and shouldn't be used beyond the tick they were fetched
#[derive(Clone)]
enum CreepTarget {
    Upgrade(ObjectId<StructureController>),
    Harvest(ObjectId<Source>),
}

// add wasm_bindgen to any function you would like to expose for call from js
// to use a reserved name as a function name, use `js_name`:
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Debug);
    });
    debug!("loop starting! CPU: {}", game::cpu::get_used());
    clean_memory();

    // mutably borrow the creep_targets refcell, which is holding our creep target locks
    // in the wasm heap
    CREEP_TARGETS.with(|creep_targets_refcell| {
        let mut creep_targets = creep_targets_refcell.borrow_mut();
        debug!("running creeps");
        for creep in game::creeps().values() {
            run_creep(&creep, &mut creep_targets);
        }
    });

    debug!("running spawns");
    let mut additional = 0;
    for spawn in game::spawns().values() {
        debug!("running spawn {}", spawn.name());

        let body = [Part::Move, Part::Move, Part::Carry, Part::Work];
        if spawn.room().unwrap().energy_available() >= body.iter().map(|p| p.cost()).sum() {
            // create a unique name, spawn.
            let name_base = game::time();
            let name = format!("{}-{}", name_base, additional);
            match spawn.spawn_creep(&body, &name) {
                Ok(()) => additional += 1,
                Err(e) => warn!("couldn't spawn: {:?}", e),
            }
        }
    }

    // memory cleanup; memory gets created for all creeps upon spawning, and any time move_to
    // is used; this should be removed if you're using RawMemory/serde for persistence

    info!(
        "done! cpu: {}/{}",
        game::cpu::get_used(),
        game::cpu::tick_limit()
    )
}

fn run_creep(creep: &Creep, creep_targets: &mut HashMap<String, CreepTarget>) {
    if creep.spawning() {
        return;
    }
    let memory = creep.get_memory_obj();
    match memory {
        Ok(mut o) => {
            o.working = Some(true);
            creep.set_memory_obj(o);
        }
        Err(e) => {
            error!("error getting memory: {e}");
        }
    };
    let name = creep.name();
    debug!("running creep {}", name);

    let target = creep_targets.entry(name);
    match target {
        Entry::Occupied(entry) => {
            let creep_target = entry.get();
            match creep_target {
                CreepTarget::Upgrade(controller_id)
                    if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    if let Some(controller) = controller_id.resolve() {
                        creep
                            .upgrade_controller(&controller)
                            .unwrap_or_else(|e| match e {
                                ErrorCode::NotInRange => {
                                    let _ = creep.move_to(&controller);
                                }
                                _ => {
                                    warn!("couldn't upgrade: {:?}", e);
                                    entry.remove();
                                }
                            });
                    } else {
                        entry.remove();
                    }
                }
                CreepTarget::Harvest(source_id)
                    if creep.store().get_free_capacity(Some(ResourceType::Energy)) > 0 =>
                {
                    if let Some(source) = source_id.resolve() {
                        if creep.pos().is_near_to(source.pos()) {
                            creep.harvest(&source).unwrap_or_else(|e| {
                                warn!("couldn't harvest: {:?}", e);
                                entry.remove();
                            });
                        } else {
                            let _ = creep.move_to(&source);
                        }
                    } else {
                        entry.remove();
                    }
                }
                _ => {
                    entry.remove();
                }
            };
        }
        Entry::Vacant(entry) => {
            // no target, let's find one depending on if we have energy
            let room = creep.room().expect("couldn't resolve creep room");
            if creep.store().get_used_capacity(Some(ResourceType::Energy)) > 0 {
                for structure in room.find(find::STRUCTURES, None).iter() {
                    if let StructureObject::StructureController(controller) = structure {
                        entry.insert(CreepTarget::Upgrade(controller.id()));
                        break;
                    }
                }
            } else if let Some(source) = room.find(find::SOURCES_ACTIVE, None).first() {
                entry.insert(CreepTarget::Harvest(source.id()));
            }
        }
    }
}
trait CreepExtend {
    fn getType(&self) -> CreepType;
    fn get_memory_obj(&self) -> Result<CreepMemory, Error>;
    fn set_memory_obj(&self, memory: CreepMemory);
}
impl CreepExtend for Creep {
    fn getType(&self) -> CreepType {
        CreepType::builder
    }
    fn get_memory_obj(&self) -> Result<CreepMemory, Error> {
        let js_val = &Self::memory(&self);
        let js_string: Result<CreepMemory, serde_json::Error> = js_val.into_serde();
        match js_string {
            Ok(yay) => {
                return Ok(yay);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    fn set_memory_obj(&self, memory: CreepMemory) {
        let val = JsValue::from_serde(&memory);
        match val {
            Ok(o) => {
                let res = &Self::set_memory(&self, &o);
            }
            Err(e) => {
                error!("error serializing JsValue to CreepMemory: {}", e)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CreepMemory {
    _move: Option<CreepMemoryMove>,
    working: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CreepMemoryMove {
    time: u128,
    dest: Option<CreepMemoryMoveDest>,
}
#[derive(Debug, Serialize, Deserialize, Default)]

pub struct CreepMemoryMoveDest {
    x: u64,
    y: u64,
    room: String,
}
pub enum CreepType {
    builder,
    upgrader,
}
impl CreepType {
    fn short_name(&self) -> String {
        match self {
            CreepType::builder => format!("bu"),
            CreepType::upgrader => format!("up"),
        }
    }
}
impl Display for CreepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreepType::builder => write!(f, "builder"),
            CreepType::upgrader => write!(f, "upgrader"),
        }
    }
}

pub fn clean_memory() -> Result<(), Box<dyn std::error::Error>> {
    info!("running memory cleanup");
    let mut alive_creeps = HashSet::new();
    // add all living creep names to a hashset
    for creep_name in game::creeps().keys() {
        alive_creeps.insert(creep_name);
    }

    // grab `Memory.creeps` (if it exists)
    if let Ok(memory_creeps) = Reflect::get(&screeps::memory::ROOT, &JsString::from("creeps")) {
        // convert from JsValue to Object
        let memory_creeps: Object = memory_creeps.unchecked_into();
        // iterate memory creeps
        for creep_name_js in Object::keys(&memory_creeps).iter() {
            let res = Reflect::has(&memory_creeps, &creep_name_js);
            warn!("{res:?}");
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
#[derive(Debug, Serialize, Deserialize, Default)]
struct GlobalMemory {
    creeps: HashMap<String, CreepMemory>, //other vars not implemented yet
}
impl GlobalMemory {
    fn get() -> Result<GlobalMemory, Error> {
        let json_var: &JsValue = screeps::memory::ROOT.as_ref();
        json_var.into_serde()
    }
    fn set(&mut self) -> Result<(), Error> {
        let val = js_sys::Reflect::has(&screeps::memory::ROOT, &"banana".into());
        warn!("{val:?}");
        Ok(())
    }
}
