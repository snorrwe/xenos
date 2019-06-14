#![recursion_limit = "256"]
extern crate fern;
#[macro_use]
extern crate log;
extern crate screeps;
#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate serde;
extern crate arrayvec;
extern crate serde_json;

mod bt;
mod constructions;
mod creeps;
mod game_loop;
mod game_state;
mod logging;
mod prelude;
mod rooms;
mod structures;

use game_loop::game_loop;

fn main() {
    stdweb::initialize();
    logging::setup_logging(logging::Info);

    js! {
        const game_loop = @{game_loop};

        module.exports.loop = function() {
            try {
                const bucket = Game.cpu.bucket;
                if (bucket < 500) {
                    console.log("Bucket:", bucket);
                    console_error("Bucket is empty, skipping loop this tick");
                    return;
                }

                // Run the game logic
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
