pub use super::spawn_info::*;
use super::{conqueror, defender, gofer, harvester, lrh, lrw, scout, upgrader, worker};
use crate::prelude::*;
use arrayvec::ArrayVec;
use screeps::objects::Room;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq, Ord, PartialOrd)]
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
    Scout = 8,
    Defender = 9,
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
            8 => Role::Scout,
            9 => Role::Defender,
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
            Role::Scout => "Scout",
            Role::Defender => "Defender",
        };
        write!(f, "{}", name)
    }
}

type RoleArray = [Role; 9];
impl Role {
    pub fn all_roles() -> ArrayVec<RoleArray> {
        use self::Role::*;
        const ROLES: RoleArray = [
            Upgrader, Harvester, Worker, Gofer, Lrh, Conqueror, Lrw, Scout, Defender,
        ];
        ROLES
            .iter()
            // Trigger compilation error on a new role if it's missing
            .filter_map(|r| match r {
                Scout | Upgrader | Harvester | Worker | Gofer | Lrh | Conqueror | Lrw
                | Defender => Some(*r),
                Unknown => None,
            })
            .collect()
    }
}

// TODO: return an array of all roles to spawn in order of priority
/// Get the next target role in the given room
pub fn next_role<'a>(state: &'a mut GameState, room: &'a Room) -> Option<Role> {
    let creeps = { state.count_creeps_in_room(room).clone() };
    creeps
        .into_iter()
        .fold(None, |result: Option<Role>, (role, actual)| {
            let expected = target_number_of_role_in_room(role, room, state);
            if expected <= actual {
                return result;
            }
            result
                .map(|result| {
                    let result_prio = role_priority(room, result);
                    let role_prio = role_priority(room, role);
                    if role_prio > result_prio {
                        role
                    } else {
                        result
                    }
                })
                .or_else(|| Some(role))
        })
}

/// Run the creep according to the given role
pub fn run_role<'a>(state: &mut CreepState, role: Role) -> ExecutionResult {
    let result = match role {
        Role::Upgrader => upgrader::run(state),
        Role::Harvester => harvester::run(state),
        Role::Worker => worker::run(state),
        Role::Gofer => gofer::run(state),
        Role::Conqueror => conqueror::run(state),
        Role::Lrh => lrh::run(state),
        Role::Lrw => lrw::run(state),
        Role::Scout => scout::run(state),
        Role::Defender => defender::run(state),
        _ => unimplemented!(),
    };

    result.map_err(|e| {
        warn!("Creep {} is idle: {}", state.creep_name().0, e);
        ExecutionError::from("Idle")
    })
}

