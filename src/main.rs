#![recursion_limit = "256"]
extern crate fern;
#[macro_use]
extern crate num_derive;
extern crate num;
#[macro_use]
extern crate log;
extern crate screeps;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde;
extern crate arrayvec;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

mod bt;
mod collections;
mod constructions;
mod creeps;
mod flags;
mod game_loop;
mod state;
mod logging;
mod prelude;
mod rooms;
mod stats;
mod structures;

use game_loop::game_loop;
use screeps::raw_memory;

pub const MAIN_SEGMENT: u32 = 0;
pub const CONSTRUCTIONS_SEGMENT: u32 = 1;
pub const STATISTICS_SEGMENT: u32 = 2;
pub const VERSION: &'static str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/VERSION"));
pub const COLLECT_STATS: bool = true;

lazy_static! {
    pub static ref DEPLOYMENT_TIME: u32 = { screeps::game::time() };
}

/// Run initialisation tasks
/// These are only called on script restart!
fn initialize() {
    raw_memory::set_active_segments(&[MAIN_SEGMENT, STATISTICS_SEGMENT, CONSTRUCTIONS_SEGMENT]);
}

fn main() {
    stdweb::initialize();
    logging::setup_logging(logging::Info);
    let dt = *DEPLOYMENT_TIME; // Init deployment time
    info!("Deployed version {} at {}", VERSION, dt);

    js! {
        const game_loop = @{game_loop};
        const initialize = @{initialize};

        initialize();

        module.exports.loop = function() {
            try {
                const bucket = Game.cpu.bucket;
                if (bucket < 500) {
                    console.log("Bucket:", bucket);
                    console_error("Bucket is empty, skipping loop this tick");
                    return;
                }

                game_loop();

            } catch (error) {
                console_error("caught exception:", error);
                if (error.stack) {
                    console_error("stack trace:", error.stack);
                }
                console_error("resetting VM next tick.");

                // reset the VM since we don't know if everything was cleaned up and don't
                // want an inconsistent state.
                module.exports.loop = function() {
                    wasm_module = null;
                    wasm_initialize();
                }
            }
        }
    }
}

