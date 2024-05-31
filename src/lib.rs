#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unused_mut)]
#![deny(unused_comparisons)]
#![deny(unused_braces)]
// #![deny(dead_code)]
// #![deny(unused_imports)]
pub mod roles;
mod structs;
use std::collections::HashSet;
use std::str::FromStr;

use anyhow::anyhow;
use gloo_utils::format::JsValueSerdeExt;
use js_sys::{JsString, Object, Reflect};
use log::*;
use screeps::{
    constants::{ErrorCode, Part, ResourceType},
    find::{MY_CONSTRUCTION_SITES, MY_SPAWNS, SOURCES, SOURCES_ACTIVE},
    game,
    local::ObjectId,
    objects::{Creep, Source, StructureController},
    prelude::*,
    ConstructionSite, RectStyle, Room, RoomVisual, SpawnOptions,
};
use screeps::{RoomName, TextStyle};
use serde::{Deserialize, Serialize};
use serde_json::Error;

use structs::{
    creep::CreepExtend,
    memory::{GlobalMemory, RoomMemory},
};
use structs::{creep::CreepMemory, room::RoomExtend, visual::VisualExtend};
use wasm_bindgen::prelude::*;

use crate::{roles::unknown, structs::creep::CreepType};

mod logging;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

// this enum will represent a creep's lock on a specific target object, storing a js reference
// to the object id so that we can grab a fresh reference to the object each successive tick,
// since screeps game objects become 'stale' and shouldn't be used beyond the tick they were fetched
#[derive(Debug, Serialize, Deserialize, Clone)]
enum CreepTarget {
    Upgrade(ObjectId<StructureController>),
    Harvest(ObjectId<Source>),
    Spawn(ObjectId<screeps::StructureSpawn>),
    Build(ObjectId<ConstructionSite>),
}
impl CreepTarget {
    fn run(self, creep: &Creep) -> bool {
        match self {
            CreepTarget::Build(object_id) => match object_id.resolve() {
                Some(target) => {
                    let res = creep.build(&target);
                    if let Err(errorcode) = res {
                        match errorcode {
                            ErrorCode::NotFound => {
                                let res = creep.set_target(None);
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            ErrorCode::NotEnough => {
                                let res = creep.set_target(None);
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            ErrorCode::NotInRange => {
                                let res = creep.move_to(target);
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e:?}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            _e => {
                                error!("error building {_e:?}");
                                return false;
                            }
                        };
                    };
                    return false;
                }
                None => {
                    return false;
                }
            },
            CreepTarget::Harvest(object_id) => {
                //kay we have a target lets move to it.
                if creep.is_full() {
                    let res = creep.set_target(None);
                    match res {
                        Err(e) => {
                            error!("could not set target: {e}");
                            return false;
                        }
                        Ok(_) => {
                            trace!("sucessfully set target of creep: {}", creep.name());
                            return true;
                        }
                    }
                }
                match object_id.resolve() {
                    Some(source) => {
                        let res = creep.harvest(&source);
                        match res {
                            Err(e) => match e {
                                ErrorCode::Full => {}
                                ErrorCode::NotInRange => {
                                    let res = creep.move_to(source);
                                    match res {
                                        Err(e) => {
                                            error!("could not set target: {e:?}");
                                            return false;
                                        }
                                        Ok(_) => {
                                            trace!(
                                                "sucessfully set target of creep: {}",
                                                creep.name()
                                            );
                                            return true;
                                        }
                                    }
                                }
                                _ => {
                                    error!("error while harvesting: {e:?}");
                                    return true;
                                }
                            },
                            Ok(_) => {
                                return true;
                            }
                        }
                    }
                    None => {
                        return false;
                    }
                };
                return false;
            }
            CreepTarget::Spawn(object_id) => match object_id.resolve() {
                Some(spawn) => {
                    let res = creep.transfer(&spawn, ResourceType::Energy, creep.get_energy());
                    match res {
                        Ok(_) => {
                            let res = creep.set_target(None);
                            match res {
                                Err(e) => {
                                    error!("could not set target: {e}");
                                    return false;
                                }
                                Ok(_) => {
                                    trace!("sucessfully set target of creep: {}", creep.name());
                                    return true;
                                }
                            }
                        }
                        Err(error) => match error {
                            ErrorCode::NotEnough => {
                                let res = creep.set_target(Some(CreepTarget::Harvest(
                                    creep
                                        .room()
                                        .unwrap()
                                        .get_active_sources()
                                        .first()
                                        .unwrap()
                                        .id(),
                                )));
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }

                            ErrorCode::Full => {
                                let res = creep.set_target(Some(CreepTarget::Upgrade(
                                    creep.room().unwrap().controller().unwrap().id(),
                                )));
                                match res {
                                    Err(e) => {
                                        error!("could not set target: {e}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            ErrorCode::NotInRange => {
                                let res = creep.move_to(spawn);
                                match res {
                                    Err(e) => {
                                        error!("could not move to spawn: {e:?}");
                                        return false;
                                    }
                                    Ok(_) => {
                                        trace!("sucessfully set target of creep: {}", creep.name());
                                        return true;
                                    }
                                }
                            }
                            _ => {
                                error!("some kind of error happened: {error:?}");
                                return false;
                            }
                        },
                    }
                }
                None => {
                    error!("spawn not set?");
                    return false;
                }
            },
            CreepTarget::Upgrade(object_id) => {
                match object_id.resolve() {
                    Some(controller) => {
                        let res = creep.upgrade_controller(&controller);
                        match res {
                            Ok(_) => {
                                trace!("we upgraded the controller bois");
                                return true;
                            }
                            Err(error) => {
                                match error {
                                    ErrorCode::NotEnough => {
                                        let res = creep.set_target(None);
                                        match res {
                                            Err(e) => {
                                                error!("could not set target: {e}");
                                                return false;
                                            }
                                            Ok(_) => {
                                                trace!(
                                                    "sucessfully set target of creep: {}",
                                                    creep.name()
                                                );
                                                return true;
                                            }
                                        }
                                    } // maybe get some energy?
                                    ErrorCode::NotInRange => {
                                        let res = creep.move_to(controller);
                                        match res {
                                            Err(e) => {
                                                error!("could not move to:  {e:?}");
                                                return false;
                                            }
                                            Ok(_) => {
                                                trace!(
                                                    "creep moved to controller {}",
                                                    creep.name()
                                                );
                                                return true;
                                            }
                                        }
                                    } // fucking move to it
                                    ErrorCode::Busy => {
                                        trace!("seems like creep is spawning");
                                        return false;
                                    }
                                    _ => {
                                        error!(
                                            "unknown error for upgrading the controller: {error:?}"
                                        );
                                        return false;
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
        }
    }
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
                    t => t.run(creep),
                },
                None => {
                    unknown::run(creep);
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
                let memory = JsValue::from_serde(
                    &CreepMemory::default()
                        .set_homeroom(Some(spawn.room().unwrap()))
                        .set_type(Some(CreepType::Upgrader)),
                )
                .unwrap();
                let opts = SpawnOptions::default().memory(memory);
                match spawn.spawn_creep_with_options(&body, &name, &opts) {
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
    let my_rooms = game::rooms().entries().filter(|x| x.1.is_mine());
    for (_n, r) in my_rooms {
        draw_ui(&r);
        update_room_mem(r);
    }

    info!(
        "done! cpu: {:.2}/{}/{}",
        game::cpu::get_used(),
        game::cpu::limit(),
        game::cpu::tick_limit()
    );
    update_stats()
}
fn update_room_mem(room: Room) {
    let memory = RoomMemory {
        ..Default::default()
    };
    let res = &room.clone().set_memory_obj(memory);
    match res {
        Err(e) => {
            error!("could not set room memory: {e}");
        }
        Ok(_) => {
            trace!("memory set of {}", room.name());
        }
    }
}
fn update_stats() {
    // get global mem
    let mem = GlobalMemory::get();
    match mem {
        Err(e) => error!("{e}"),
        Ok(o) => o.update_stats(),
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

impl RoomExtend for Room {
    fn get_sources(self) -> Vec<Source> {
        self.find(SOURCES, None)
    }
    fn get_memory_obj(self) -> anyhow::Result<RoomMemory, anyhow::Error> {
        match self.memory().into_serde() {
            Err(e) => {
                return Err(anyhow!(
                    "could not convert jsvalue to room memory struct{e}"
                ))
            }
            Ok(o) => Ok(o),
        }
    }
    fn get_controller_id(&self) -> Option<ObjectId<StructureController>> {
        match self.controller() {
            Some(s) => Some(s.id()),
            None => None,
        }
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
    fn get_construction_sites(self) -> Vec<ConstructionSite> {
        self.find(MY_CONSTRUCTION_SITES, None)
    }

    fn set_memory_obj(self, memory: RoomMemory) -> anyhow::Result<(), anyhow::Error> {
        let val = JsValue::from_serde(&memory);
        match val {
            Ok(o) => {
                Self::set_memory(&self, &o);

                return Ok(());
            }
            Err(e) => {
                error!("error serializing JsValue to CreepMemory: {}", e);
                return Err(anyhow!("error serializing JsValue to CreepMemory: {}", e));
            }
        }
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

fn draw_ui(room: &Room) {
    trace!("drawing ui for {}", room.name());
    let procent = (game::cpu::get_used() / game::cpu::limit() as f64) as f32;
    let color = match procent {
        0.0..0.1 => "green",
        0.1..0.5 => "yellow",
        0.5..0.9 => "orange",
        0.9..1.0 => "red",
        _ => "blue",
    };
    let spawns = &room.clone().get_spawn();
    match spawns.first() {
        Some(spawn) => match spawn.spawning() {
            Some(s) => room.visual().text(1.0, 1.0, format!("{}", s.name()), None),
            None => {}
        },
        None => {
            warn!("no spawns in room: {}", &room.name())
        }
    }
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
    fn get_home_room(&self) -> anyhow::Result<Option<Room>, anyhow::Error> {
        match self.get_memory_obj() {
            Err(e) => Err(anyhow!("could not read memory: {e}")),
            Ok(o) => {
                let homeroom = o.homeroom;
                match homeroom {
                    Some(r) => {
                        let roomname = RoomName::from_str(&r);
                        match roomname {
                            Err(e) => {
                                return Err(anyhow!("room not found? {e}"));
                            }
                            Ok(o) => {
                                return Ok(game::rooms().get(o));
                            }
                        }
                        // return Ok(game::rooms().get()?);
                    }
                    None => {
                        return Err(anyhow!("room not defined"));
                    }
                };
            }
        }
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
                        t => t.run(self),
                    };
                }
                None => {
                    let res = self.set_target(Some(CreepTarget::Upgrade(
                        self.room().unwrap().controller().unwrap().id(),
                    )));
                    match res {
                        Err(e) => {
                            error!("could not set target: {e}");
                            return false;
                        }
                        Ok(_) => {
                            trace!("sucessfully set target of creep: {}", self.name());
                            return true;
                        }
                    }
                }
            },
            Err(e) => {
                warn!("cant get creep target{e:?}")
            }
        }
        return false;
    }

    fn total_of_type(&self, homeroom: bool) -> anyhow::Result<u32, anyhow::Error> {
        match self.get_type() {
            Ok(t) => match t {
                Some(s) => match s.amount_alive(match homeroom {
                    false => None,
                    true => self.get_home_room().unwrap_or_default(),
                }) {
                    Err(e) => Err(anyhow!(e.to_string())),
                    Ok(o) => Ok(o),
                },
                None => Err(anyhow!("creep has no type")),
            },
            Err(e) => {
                warn!("creep has no type cant get amount of creeps");
                return Err(anyhow!("{}", e.to_string()));
            }
        }
    }

    fn get_energy(&self) -> Option<u32> {
        let total = self.store().get_used_capacity(Some(ResourceType::Energy));
        match total {
            0 => None,
            _t => Some(_t),
        }
    }

    fn has_resource(&self, resourcetype: ResourceType) -> Option<u32> {
        let total = self.store().get_used_capacity(Some(resourcetype));
        match total {
            0 => None,
            _t => Some(_t),
        }
    }
}
