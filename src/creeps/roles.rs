pub use super::spawn_info::*;
use super::{conqueror, gofer, harvester, lrh, upgrader, worker, lrw};
use crate::prelude::*;
use screeps::objects::{Creep, Room};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[repr(u8)]
pub enum Role {
    Unknown = 0,
    Upgrader = 1,
    Harvester = 2,
    Worker = 3,
    Gofer = 4,
    Lrh = 5,
    Conqueror = 6,
    Lrw = 7,
}

impl From<u8> for Role {
    fn from(item: u8) -> Self {
        match item {
            0 => Role::Unknown,
            1 => Role::Upgrader,
            2 => Role::Harvester,
            3 => Role::Worker,
            4 => Role::Gofer,
            5 => Role::Lrh,
            6 => Role::Conqueror,
            7 => Role::Lrw,
            _ => unimplemented!("Role {} is not unimplemented!", item),
        }
    }
}

impl Display for Role {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let name = match self {
            Role::Unknown => "Unknown",
            Role::Upgrader => "Upgrader",
            Role::Harvester => "Harvester",
            Role::Worker => "Worker",
            Role::Gofer => "Gofer",
            Role::Lrh => "Lrh",
            Role::Lrw => "Lrw",
            Role::Conqueror => "Conqueror",
        };
        write!(f, "{}", name)?;
        Ok(())
    }
}

impl Role {
    pub fn all_roles() -> &'static [Self] {
        use self::Role::*;
        &[Upgrader, Harvester, Worker, Gofer, Lrh, Conqueror]
    }
}

// TODO: return an array of all roles to spawn in order of priority
/// Get the next target role in the given room
pub fn next_role<'a>(state: &'a mut GameState, room: &'a Room) -> Option<Role> {
    let conqueror_count = state.global_conqueror_count();
    let counts = state.count_creeps_in_room(room);
    counts.insert(Role::Conqueror, conqueror_count);

    counts
        .into_iter()
        .fold(None, |result: Option<Role>, (role, actual)| {
            let expected = target_number_of_role_in_room(*role, room);
            if expected <= *actual {
                return result;
            }
            result
                .map(|result| {
                    let result_prio = role_priority(room, result);
                    let role_prio = role_priority(room, *role);
                    if role_prio > result_prio {
                        *role
                    } else {
                        result
                    }
                })
                .or_else(|| Some(*role))
        })
}

/// Run the creep according to the given role
pub fn run_role<'a>(role: Role, creep: &'a Creep) -> Task<'a, GameState> {
    trace!("Running creep {} by role {}", creep.name(), role);

    let task = match role {
        Role::Upgrader => upgrader::run(creep),
        Role::Harvester => harvester::run(creep),
        Role::Worker => worker::run(creep),
        Role::Gofer => gofer::run(creep),
        Role::Conqueror => conqueror::run(creep),
        Role::Lrh => lrh::run(creep),
        Role::Lrw => lrw::run(creep),
        _ => unimplemented!(),
    };

    Task::new(move |state| {
        task.tick(state).map_err(|e| {
            let error = format!("Creep {} is idle: {}", creep.name(), e);
            warn!("{}", error);
            error
        })
    })
    .with_name("Run role")
}

