#![recursion_limit = "256"]
extern crate fern;
#[macro_use]
extern crate log;
extern crate screeps;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde;
extern crate serde_json;

mod bt;
mod constructions;
mod creeps;
mod game_loop;
mod logging;
mod structures;
mod game_state;
mod rooms;

use game_loop::game_loop;

fn main() {
    stdweb::initialize();
    logging::setup_logging(logging::Debug);

    js! {
        const game_loop = @{game_loop};

        module.exports.loop = function() {
            // Provide actual error traces.
            try {
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
                    __initialize(new WebAssembly.Module(require("xenos_bg")), false);
                    module.exports.loop();
                }
            }
        }
    }
}
