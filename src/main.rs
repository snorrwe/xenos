#![recursion_limit = "128"]
extern crate fern;
#[macro_use]
extern crate log;
extern crate screeps;
#[macro_use]
extern crate stdweb;

mod bt;
mod creeps;
mod game_loop;
mod logging;
mod spawns;

use game_loop::game_loop;

fn main() {
    stdweb::initialize();
    logging::setup_logging(logging::Debug);

    js! {
        var game_loop = @{game_loop};

        module.exports.loop = function() {
            // Provide actual error traces.
            try {
                game_loop();
            } catch (error) {
                // console_error function provided by 'screeps-game-api'
                console_error("caught exception:", error);
                if (error.stack) {
                    console_error("stack trace:", error.stack);
                }
                console_error("resetting VM next tick.");
                // reset the VM since we don't know if everything was cleaned up and don't
                // want an inconsistent state.
                module.exports.loop = function() {
                    __initialize(new WebAssembly.Module(require("compiled")), false);
                    module.exports.loop();
                }
            }
        }
    }
}
