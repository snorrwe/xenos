#![recursion_limit = "256"]
extern crate fern;
#[macro_use]
extern crate log;
extern crate screeps;
#[macro_use]
extern crate stdweb;

mod bt;
mod constructions;
mod creeps;
mod game_loop;
mod logging;
mod structures;

use game_loop::game_loop;

fn main() {
    stdweb::initialize();
    logging::setup_logging(logging::Info);

    js! {
        const game_loop = @{game_loop};

        function sendStats() {
            let cpu = Game.cpu.getUsed();
            let bucket = Game.cpu.bucket;
            let gcl = Game.gcl;
            let population = Object.keys(Game.creeps).length;
            let time = Game.time;
            let stats = {
                time,
                cpu,
                bucket,
                gcl,
                population
            };
            stats = JSON.stringify(stats);
            Game.notify(stats, 0);
        }

        module.exports.loop = function() {
            // Provide actual error traces.
            try {
                game_loop();
                sendStats();
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

