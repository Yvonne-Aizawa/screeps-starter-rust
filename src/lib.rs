#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unused_mut)]
#![deny(unused_comparisons)]
#![deny(unused_braces)]
// #![deny(dead_code)]
// #![deny(unused_imports)]
pub mod roles;
mod structs;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::{JsString, Object, Reflect};
use log::*;
use screeps::TextStyle;
use screeps::Visual::Rect;
use screeps::{
    constants::{ErrorCode, Part, ResourceType},
    enums::StructureObject,
    find::{self, MY_SPAWNS, SOURCES, SOURCES_ACTIVE, STRUCTURES},
    game,
    local::ObjectId,
    look::ENERGY,
    objects::{Creep, Source, StructureController},
    prelude::*,
    spawn, RectStyle, Room, RoomVisual, Visual,
};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::{
    cell::RefCell,
    collections::{hash_map::Entry, HashMap, HashSet},
};
use structs::{
    creep::{CreepExtend, CreepMemory},
    room::RoomExtend,
    visual::VisualExtend,
};
use wasm_bindgen::prelude::*;

use crate::{
    roles::{builder, harvester, unknown, upgrader},
    structs::creep::CreepType,
};

mod logging;

// this is one way to persist data between ticks within Rust's memory, as opposed to
// keeping state in memory on game objects - but will be lost on global resets!
// thread_local! {
//     static CREEP_TARGETS: RefCell<HashMap<String, CreepTarget>> = RefCell::new(HashMap::new());
// }

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

// this enum will represent a creep's lock on a specific target object, storing a js reference
// to the object id so that we can grab a fresh reference to the object each successive tick,
// since screeps game objects become 'stale' and shouldn't be used beyond the tick they were fetched
#[derive(Debug, Serialize, Deserialize, Clone)]
enum CreepTarget {
    Upgrade(ObjectId<StructureController>),
    Harvest(ObjectId<Source>),
    Spawn(ObjectId<screeps::StructureSpawn>),
    None(),
}

// add wasm_bindgen to any function you would like to expose for call from js
// to use a reserved name as a function name, use `js_name`:
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Trace);
    });

    debug!("loop starting! CPU: {:.2}", game::cpu::get_used());

    // mutably borrow the creep_targets refcell, which is holding our creep target locks
    // in the wasm heap

    for creep in game::creeps().values() {
        creep.run();
        match creep.get_type() {
            Ok(res) => match res {
                Some(creep_type) => match creep_type {
                    CreepType::Unknown => unknown::run(creep),
                    CreepType::Builder => builder::run(creep),
                    CreepType::Upgrader => upgrader::run(creep),
                    CreepType::Harvester => harvester::run(creep),
                },
                None => {
                    creep.set_type(Some(CreepType::Unknown));
                }
            },
            Err(e) => {
                error!("unable to get type from creep {e}")
            }
        }

        // run_creep(&creep, &mut creep_targets);
    }

    debug!("running spawns");
    let mut additional = 0;
    for spawn in game::spawns().values() {
        if game::creeps().entries().count() >= 5 {
            info!("not spawning more creeps ");
        } else {
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
    }

    // memory cleanup; memory gets created for all creeps upon spawning, and any time move_to
    // is used; this should be removed if you're using RawMemory/serde for persistence
    match clean_memory() {
        Err(e) => {
            error!("error cleaning memory: {e}")
        }
        Ok(_) => {
            info!("cleaned memory")
        }
    }
    let my_Rooms = game::rooms().entries().filter(|x| x.1.is_mine());
    for (r, n) in my_Rooms {
        draw_ui(n)
    }

    info!(
        "done! cpu: {:.2}/{}/{}",
        game::cpu::get_used(),
        game::cpu::limit(),
        game::cpu::tick_limit()
    )
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

// implementations
#[allow(dead_code)]

impl CreepExtend for Creep {
    fn get_type(&self) -> Result<Option<CreepType>, Error> {
        let mem = self.get_memory_obj();
        let creeptype = match mem {
            Ok(o) => o,
            Err(e) => {
                return Err(e);
            }
        };
        return Ok(creeptype._type);
    }
    fn has_room(&self) -> bool {
        self.store().get_free_capacity(None) != 0
    }
    fn is_empty(&self) -> bool {
        self.store().get_used_capacity(None) == 0
    }
    fn is_full(&self) -> bool {
        self.store().get_free_capacity(None) == 0
    }
    fn set_type(&self, new_type: Option<CreepType>) -> Result<(), Error> {
        let mem = self.get_memory_obj();
        match mem {
            Ok(mut o) => {
                o._type = new_type;
                self.set_memory_obj(o)
            }
            Err(e) => Err(e),
        }
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
    fn set_memory_obj(&self, memory: CreepMemory) -> Result<(), Error> {
        let val = JsValue::from_serde(&memory);
        match val {
            Ok(o) => {
                Self::set_memory(&self, &o);

                return Ok(());
            }
            Err(e) => {
                error!("error serializing JsValue to CreepMemory: {}", e);
                return Err(e);
            }
        }
    }
    fn set_working(&self, working: bool) -> Result<(), Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(mut o) => {
                o.working = Some(working);
                return self.set_memory_obj(o.clone());
            }
        }
    }
    fn get_working(&self) -> Result<Option<bool>, Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(o) => {
                return Ok(o.working);
            }
        }
    }

    fn get_target(&self) -> Result<Option<CreepTarget>, Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(o) => {
                return Ok(o.target);
            }
        }
    }

    fn set_target(&self, new_target: Option<CreepTarget>) -> Result<(), Error> {
        let mem = self.get_memory_obj();
        match mem {
            Err(e) => {
                return Err(e);
            }
            Ok(mut o) => {
                o.target = new_target;
                return self.set_memory_obj(o.clone());
            }
        }
    }

    fn run(&self) -> bool {
        match self.get_target() {
            Ok(o) => match o {
                Some(t) => {
                    match t {
                        CreepTarget::Upgrade(target) => {
                            match target.resolve() {
                                Some(controller) => {
                                    let res = self.upgrade_controller(&controller);
                                    match res {
                                        Ok(_) => {
                                            trace!("we upgraded the controller bois");
                                            return true;
                                        }
                                        Err(error) => {
                                            match error {
                                                ErrorCode::NotEnough => {
                                                    self.set_target(Some(CreepTarget::None()));
                                                    // self.set_target(Some(CreepTarget::Harvest(
                                                    //     self.room()
                                                    //         .unwrap()
                                                    //         .get_active_sources()
                                                    //         .first()
                                                    //         .unwrap()
                                                    //         .id(),
                                                    // )));
                                                } // maybe get some energy?
                                                ErrorCode::NotInRange => {
                                                    self.move_to(controller);
                                                } // fucking move to it
                                                _ => {
                                                    error!("unknown error for upgrading the controller: {error:?}")
                                                }
                                            }
                                        }
                                    }
                                }
                                None => {
                                    error!("seems like the controller does not exist");
                                    return false;
                                }
                            }
                        }
                        CreepTarget::Harvest(target) => {
                            //kay we have a target lets move to it.
                            if self.is_full() {
                                self.set_target(Some(CreepTarget::None()));
                                // if (game::time() % 2) == 0 {
                                //     self.set_target(Some(CreepTarget::Upgrade(
                                //         self.room().unwrap().controller().unwrap().id(),
                                //     )));
                                // } else {
                                //     self.set_target(Some(CreepTarget::Spawn(
                                //         self.room().unwrap().get_spawn().first().unwrap().id(),
                                //     )));
                                // }
                            }
                            match target.resolve() {
                                Some(source) => {
                                    let res = self.harvest(&source);
                                    match res {
                                        Err(e) => match e {
                                            ErrorCode::Full => {}
                                            ErrorCode::NotInRange => {
                                                self.move_to(source);
                                            }
                                            _ => {
                                                error!("error while harvesting: {e:?}")
                                            }
                                        },
                                        Ok(_) => {}
                                    }
                                }
                                None => {}
                            };
                        }
                        CreepTarget::Spawn(s) => match s.resolve() {
                            Some(spawn) => {
                                let res = self.transfer(
                                    &spawn,
                                    ResourceType::Energy,
                                    Some(
                                        self.store().get_used_capacity(Some(ResourceType::Energy)),
                                    ),
                                );
                                match res {
                                    Ok(_) => {
                                        self.set_target(Some(CreepTarget::None()));
                                        // self.set_target(Some(CreepTarget::Harvest(
                                        //     self.room()
                                        //         .unwrap()
                                        //         .get_active_sources()
                                        //         .first()
                                        //         .unwrap()
                                        //         .id(),
                                        // )));
                                    }
                                    Err(error) => match error {
                                        ErrorCode::NotEnough => {
                                            self.set_target(Some(CreepTarget::Harvest(
                                                self.room()
                                                    .unwrap()
                                                    .get_active_sources()
                                                    .first()
                                                    .unwrap()
                                                    .id(),
                                            )));
                                        }

                                        ErrorCode::Full => {
                                            self.set_target(Some(CreepTarget::Upgrade(
                                                self.room().unwrap().controller().unwrap().id(),
                                            )));
                                        }
                                        ErrorCode::NotInRange => {
                                            self.move_to(spawn);
                                        }
                                        _ => {
                                            error!("some kind of error happened: {error:?}");
                                        }
                                    },
                                }
                            }
                            None => {
                                error!("spawn not set?")
                            }
                        },
                        CreepTarget::None() => {
                            warn!("creep has no target set");
                            return false;
                        }
                    };
                }
                None => {
                    self.set_target(Some(CreepTarget::Upgrade(
                        self.room().unwrap().controller().unwrap().id(),
                    )));
                    return false;
                }
            },
            Err(e) => {
                warn!("cant get creep target{e:?}")
            }
        }
        return false;
    }
}

impl RoomExtend for Room {
    fn get_sources(self) -> Vec<Source> {
        self.find(SOURCES, None)
    }
    fn get_spawn(self) -> Vec<screeps::StructureSpawn> {
        self.find(MY_SPAWNS, None)
    }
    fn is_mine(&self) -> bool {
        match self.controller() {
            Some(c) => c.my(),
            None => false,
        }
    }
    fn get_active_sources(self) -> Vec<Source> {
        self.find(SOURCES_ACTIVE, None)
    }
}

impl VisualExtend for RoomVisual {
    fn draw_progress_bar(
        self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        procent: f32,
        front_style: Option<RectStyle>,
        back_style: Option<RectStyle>,
        label: Option<String>,
    ) -> Self {
        self.rect(
            x + 0.1,
            y + 0.1,
            (width - 0.2) * procent,
            height - 0.2,
            front_style,
        );
        self.rect(x, y, width, height, back_style);
        if let Some(l) = label {
            let style = Some(
                TextStyle::default()
                    .align(screeps::TextAlign::Left)
                    .font(0.5),
            );
            self.text(x, y, l, style)
        }
        self
    }
}

fn draw_ui(room: Room) {
    trace!("drawing ui for {}", room.name());
    let procent = (game::cpu::get_used() / game::cpu::limit() as f64) as f32;
    warn!("{procent}");
    let color = match procent {
        0.0..0.1 => "green",
        0.1..0.5 => "yellow",
        0.5..0.9 => "orange",
        0.9..1.0 => "red",
        _ => "blue",
    };
    let front_style = Some(RectStyle::default().fill(color));
    let back_style = Some(RectStyle::default().fill("black"));
    room.visual().draw_progress_bar(
        1.0,
        1.0,
        10.0,
        0.5,
        procent,
        front_style,
        back_style,
        Some("cpu usage".to_string()),
    );
}
