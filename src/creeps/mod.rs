pub mod roles;

mod builder;
mod harvester;
mod repairer;
mod upgrader;

use super::bt::*;
use screeps::{
    constants::ResourceType,
    objects::{Creep, StructureContainer, Withdrawable},
    prelude::*,
    ReturnCode,
};
use stdweb::{unstable::TryInto, Reference};

pub fn task<'a>() -> Node<'a> {
    let tasks = vec![run_creeps()];
    Node::Control(Control::Selector(tasks))
}

fn run_creeps<'a>() -> Node<'a> {
    let tasks = screeps::game::creeps::values()
        .into_iter()
        .map(|creep| run_creep(creep))
        .collect();

    Node::Control(Control::All(tasks))
}

fn run_creep<'a>(creep: Creep) -> Node<'a> {
    let task = move || {
        debug!("Running creep {}", creep.name());
        if creep.spawning() {
            return Ok(());
        }
        let tasks = vec![
            Task::new("run_role", || run_role(&creep)),
            Task::new("assing_role", || {
                assign_role(&creep);
                Ok(())
            }),
        ]
        .into_iter()
        .map(|task| Node::Task(task))
        .collect();
        let tree = BehaviourTree::new(Control::Sequence(tasks));
        tree.tick()
    };
    let task = Task::new("run_creep", task);
    Node::Task(task)
}

fn assign_role<'a>(creep: &'a Creep) -> Option<String> {
    trace!("Assigning role to {}", creep.name());

    let result = roles::next_role(&creep.room()).or_else(|| {
        warn!("Room is full");
        None
    })?;

    creep.memory().set("role", &result);

    trace!("Assigned role {} to {}", result, creep.name());
    Some(result)
}

fn run_role<'a>(creep: &'a Creep) -> ExecutionResult {
    let role = creep
        .memory()
        .string("role")
        .map_err(|e| {
            error!("failed to read creep role {:?}", e);
        })?
        .ok_or_else(|| {
            trace!("creep role is null");
        })?;

    roles::run_role(role.as_str(), creep)
}

pub fn move_to<'a>(
    creep: &'a Creep,
    target: &'a impl screeps::RoomObjectProperties,
) -> ExecutionResult {
    let res = creep.move_to(target);
    match res {
        ReturnCode::Ok => Ok(()),
        _ => {
            warn!("Move failed {:?}", res);
            Err(())
        }
    }
}

/// Retreive energy from a Container
/// # Contracts & Side effects
/// Required the `loading` flag to be set to true
/// If the creep is full sets the `loading` flag to false
pub fn get_energy<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Getting energy");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err(());
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        Err(())
    } else {
        let target = find_container(creep).ok_or_else(|| {})?;

        let tasks = vec![Task::new("transfer container", || {
            try_withdraw::<StructureContainer>(creep, &target)
        })]
        .into_iter()
        .map(|task| Node::Task(task))
        .collect();

        let tree = BehaviourTree::new(Control::Sequence(tasks));
        tree.tick().map_err(|_| {
            creep.memory().del("target");
        })
    }
}

fn try_withdraw<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Withdrawable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| {})?;
    withdraw(creep, &target)
}

fn withdraw<'a, T>(creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Withdrawable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.withdraw_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            warn!("couldn't unload: {:?}", r);
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

fn find_container<'a>(creep: &'a Creep) -> Option<Reference> {
    trace!("Finding new withdraw target");
    // screeps api is bugged at the moment and FIND_STRUCTURES panics
    let result = js!{
        let creep = @{creep};
        const containers = creep.room.find(FIND_STRUCTURES, {
            filter: (i) => i.structureType == STRUCTURE_CONTAINER &&
                           i.store[RESOURCE_ENERGY] > 0
        });
        return containers[0];
    };
    let result = result.try_into().unwrap_or_else(|_| None);
    result
}

