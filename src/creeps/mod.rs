pub mod roles;

mod builder;
mod conqueror;
mod gofer;
mod harvester;
mod lrh;
mod repairer;
mod upgrader;

pub use self::roles::Role;
use crate::prelude::*;
use screeps::{
    constants::ResourceType,
    game::get_object_erased,
    objects::{
        Creep, RoomObject, RoomObjectProperties, Structure, StructureContainer, StructureStorage,
        Tombstone, Withdrawable,
    },
    prelude::*,
    ReturnCode, Room,
};
use stdweb::{unstable::TryInto, Reference};

pub const HOME_ROOM: &'static str = "home";
pub const TARGET: &'static str = "target";
pub const CREEP_ROLE: &'static str = "role";

pub fn task<'a>() -> Task<'a> {
    Task::new(move |state| {
        screeps::game::creeps::values()
            .into_iter()
            .for_each(|creep| run_creep(state, creep).unwrap_or(()));
        Ok(())
    })
}

fn run_creep<'a>(state: &mut GameState, creep: Creep) -> ExecutionResult {
    debug!("Running creep {}", creep.name());
    if creep.spawning() {
        return Ok(());
    }
    let tasks = vec![
        Task::new(|state| run_role(state, &creep)),
        Task::new(|state| initialize_creep(state, &creep)),
    ];
    let tree = Control::Sequence(tasks);
    tree.tick(state)
}

pub fn initialize_creep<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    assign_role(state, &creep)
        .map(|_| {})
        .ok_or_else(|| "Failed to find a role for creep")?;
    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(HOME_ROOM.into(), creep.room().name().into());
    Ok(())
}

fn assign_role<'a>(state: &'a mut GameState, creep: &'a Creep) -> Option<Role> {
    trace!("Assigning role to {}", creep.name());

    if state
        .creep_memory_role(CreepName(&creep.name()), CREEP_ROLE)
        .is_some()
    {
        trace!("Already has a role");
        None?;
    }

    let result = roles::next_role(state, &creep.room()).or_else(|| {
        warn!("Room is full");
        None
    })?;

    let memory = state.creep_memory_entry(CreepName(&creep.name()));
    memory.insert(CREEP_ROLE.to_string(), (result as i64).into());
    Some(result)
}

fn run_role<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    let task = {
        let role = state
            .creep_memory_role(CreepName(&creep.name()), CREEP_ROLE)
            .ok_or_else(|| {
                let error = "failed to read creep role";
                error!("{}", error);
                error
            })?;

        roles::run_role(role, creep)
    };
    task.tick(state)
}

pub fn move_to<'a>(
    creep: &'a Creep,
    target: &'a impl screeps::RoomObjectProperties,
) -> ExecutionResult {
    use screeps::traits::TryFrom;
    let res = js! {
        const creep = @{creep};
        const target = @{target.pos()};
        return creep.moveTo(target, {reusePath: 10});
    };
    let res =
        ReturnCode::try_from(res).map_err(|e| format!("Failed to convert move result {:?}", e))?;
    match res {
        ReturnCode::Ok | ReturnCode::Tired => Ok(()),
        _ => {
            let error = format!("Move failed {:?}", res);
            debug!("{}", error);
            Err(error)
        }
    }
}

/// Retreive energy from a Container
/// # Contracts & Side effects
/// Required the `loading` flag to be set to true
/// If the creep is full sets the `loading` flag to false
pub fn get_energy<'a>(state: &'a mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Getting energy");
    let target = {
        if !state.creep_memory_bool(CreepName(&creep.name()), "loading") {
            Err("not loading")?;
        }

        let memory = state.creep_memory_entry(CreepName(&creep.name()));

        if creep.carry_total() == creep.carry_capacity() {
            memory.insert("loading".into(), false.into());
            memory.remove(TARGET);
            Err("full")?;
        }

        memory
            .get(TARGET)
            .map(|x| x.as_str())
            .iter()
            .filter_map(|id| *id)
            .find_map(|id| get_object_erased(id))
            .or_else(|| {
                find_available_energy(creep).map(|target| {
                    let id = js! {
                        return @{&target}.id;
                    };

                    let id: String = id.try_into().unwrap();

                    memory.insert(TARGET.into(), id.into());
                    target
                })
            })
            .ok_or_else(|| {
                memory.remove(TARGET);
                "Can't find energy source"
            })?
    };

    let tasks = vec![
        Task::new(|_| try_withdraw::<Tombstone>(creep, &target)),
        Task::new(|_| try_withdraw::<StructureStorage>(creep, &target)),
        Task::new(|_| try_withdraw::<StructureContainer>(creep, &target)),
        Task::new(|state| {
            let memory = state.creep_memory_entry(CreepName(&creep.name()));
            memory.remove(TARGET);
            Ok(())
        }),
    ];

    let tree = Control::Sequence(tasks);

    tree.tick(state).map_err(|_| {
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        memory.remove(TARGET);
        "can't withdraw".into()
    })
}

fn try_withdraw<'a, T>(creep: &'a Creep, target: &'a RoomObject) -> ExecutionResult
where
    T: Withdrawable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| String::new())?;
    withdraw(creep, &target)
}

fn withdraw<'a, T>(creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Withdrawable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.withdraw_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            debug!("couldn't withdraw: {:?}", r);
            Err("couldn't withdraw")?;
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

fn find_available_energy<'a>(creep: &'a Creep) -> Option<RoomObject> {
    trace!("Finding new withdraw target");
    let result = js! {
        const creep = @{creep};
        const energy = creep.pos.findClosestByRange(FIND_TOMBSTONES, {
            filter: (ts) => ts.creep.my && ts.store[RESOURCE_ENERGY]
        });
        if (energy) {
            return energy;
        }
        const container = creep.pos.findClosestByRange(FIND_STRUCTURES, {
            filter: (i) => (i.structureType == STRUCTURE_CONTAINER || i.structureType == STRUCTURE_STORAGE)
                && i.store[RESOURCE_ENERGY] > 0
        });
        return container;
    };
    result.try_into().unwrap_or_else(|_| None)
}

/// Fallback harvest, method for a worker to harvest energy temporary
/// ## Contracts:
/// - Should not interfere with the harvester::harvest functionality
pub fn harvest<'a>(state: &mut GameState, creep: &'a Creep) -> ExecutionResult {
    trace!("Worker harvesting");

    {
        let loading = state.creep_memory_bool(CreepName(&creep.name()), "loading");
        if !loading {
            Err("not loading")?;
        }
        let memory = state.creep_memory_entry(CreepName(&creep.name()));
        if creep.carry_total() == creep.carry_capacity() {
            memory.insert("loading".into(), false.into());
            memory.remove(TARGET);
            return Ok(());
        }
    }

    harvester::attempt_harvest(state, creep, Some(TARGET))
}

pub fn find_repair_target<'a>(room: &'a Room) -> Option<Structure> {
    trace!("Finding repair target in room {:?}", room.name());

    let result = js! {
        const room = @{room};
        return room.find(FIND_STRUCTURES, {
            filter: s => s.hits < s.hitsMax
        })[0];
    };
    result.try_into().ok()
}

