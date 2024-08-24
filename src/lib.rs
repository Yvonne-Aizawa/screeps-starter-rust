#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unused_mut)]
#![deny(unused_comparisons)]
#![deny(unused_braces)]
// #![deny(dead_code)]
#![deny(unused_imports)]
pub mod roles;
mod structs;

use gloo_utils::format::JsValueSerdeExt;

use log::*;

use managment::memory::memory_tick;
use screeps::{constants::Part, game, prelude::*, CircleStyle, Room, SpawnOptions};

use structs::visual::{draw_energy, draw_ui};
use managment::creep::CreepExtend;
use structs::memory::{GlobalMemory, RoomMemory};
use structs::{creep::CreepMemory, room::RoomExtend};
use wasm_bindgen::prelude::*;

use crate::{roles::unknown, structs::creep::CreepType};

mod logging;
mod managment;
static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

// add wasm_bindgen to any function you would like to expose for call from js
// to use a reserved name as a function name, use `js_name`:
#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Info);
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
    memory_tick();
    let my_rooms = game::rooms().entries().filter(|x| x.1.is_mine());
    for (_n, r) in my_rooms {
        draw_ui(&r);

        update_room_mem(&r);
        let best = r.get_best_source().unwrap().pos();
        let style = CircleStyle::default().fill("red");
        r.visual()
            .circle(best.x().0 as f32, best.y().0 as f32, Some(style));
        for source in r.clone().get_sources().iter() {
            draw_energy(source, &r);
        }
    }

    info!(
        "done! cpu: {:.2}/{}/{}",
        game::cpu::get_used(),
        game::cpu::limit(),
        game::cpu::tick_limit()
    );
    update_stats()
}
fn update_room_mem(room: &Room) {
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

