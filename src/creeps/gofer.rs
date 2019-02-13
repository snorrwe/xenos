//! Move resources
//!
use super::super::bt::*;
use super::{get_energy, harvester, move_to, repairer, upgrader};
use screeps::{
    constants::ResourceType,
    game::get_object_erased,
    objects::{Creep, StructureExtension, StructureSpawn, StructureTower, Transferable},
    prelude::*,
    ReturnCode,
};
use stdweb::{unstable::TryFrom, Reference};

pub fn run<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Running builder {}", creep.name());
    let tasks = vec![
        Task::new(|_| unload(creep)),
        Task::new(|_| get_energy(creep)),
        Task::new(|_| harvest(creep)),
        Task::new(|_| unload(creep)),
        // Fallback
        Task::new(|_| repairer::attempt_repair(creep)),
        Task::new(|_| upgrader::attempt_upgrade(creep)),
    ]
    .into_iter()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick()
}

fn harvest<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Harvesting");

    let loading: bool = creep.memory().bool("loading");
    if !loading {
        return Err(());
    }
    if creep.carry_total() == creep.carry_capacity() {
        creep.memory().set("loading", false);
        creep.memory().del("target");
        Ok(())
    } else {
        harvester::attempt_harvest(creep)
    }
}

fn unload<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err(());
    }

    let carry_total = creep.carry_total();

    if carry_total == 0 {
        trace!("Empty");
        creep.memory().set("loading", true);
        return Err(());
    }

    let target = find_unload_target(creep).ok_or_else(|| {})?;

    let tasks = vec![
        Task::new(|_| try_transfer::<StructureSpawn>(creep, &target)),
        Task::new(|_| try_transfer::<StructureExtension>(creep, &target)),
        Task::new(|_| try_transfer::<StructureTower>(creep, &target)),
    ]
    .into_iter()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick().map_err(|_| {
        creep.memory().del("target");
    })
}

fn find_unload_target<'a>(creep: &'a Creep) -> Option<Reference> {
    trace!("Setting unload target");

    let target = creep
        .memory()
        .string("target")
        .map_err(|e| {
            error!("failed to read creep target {:?}", e);
        })
        .ok()?;

    if let Some(target) = target {
        trace!("Validating existing target");
        let target = get_object_erased(target.as_str())?;
        Some(target.as_ref().clone())
    } else {
        let tasks = vec![
            Task::new(|_| find_unload_target_by_type(creep, "tower")),
            Task::new(|_| find_unload_target_by_type(creep, "spawn")),
            Task::new(|_| find_unload_target_by_type(creep, "extension")),
        ]
        .into_iter()
        .collect();
        let tree = Control::Sequence(tasks);
        tree.tick().unwrap_or_else(|()| {
            warn!("Failed to find unload target");
            creep.memory().del("target");
        });
        None
    }
}

fn try_transfer<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref()).map_err(|_| {})?;
    transfer(creep, &target)
}

fn find_unload_target_by_type<'a>(creep: &'a Creep, struct_type: &'a str) -> ExecutionResult {
    let res = js!{
        const creep = @{creep};
        const exts = creep.room.find(FIND_STRUCTURES, {
            filter: function (s) {
                return s.structureType == @{struct_type} && s.energy < s.energyCapacity;
            }
        });
        return exts[0] && exts[0].id;
    };
    let target = String::try_from(res).map_err(|_| {})?;
    creep.memory().set("target", target);
    Ok(())
}

fn transfer<'a, T>(creep: &'a Creep, target: &'a T) -> ExecutionResult
where
    T: Transferable,
{
    if creep.pos().is_near_to(target) {
        let r = creep.transfer_all(target, ResourceType::Energy);
        if r != ReturnCode::Ok {
            trace!("couldn't unload: {:?}", r);
            creep.memory().del("target");
        }
    } else {
        move_to(creep, target)?;
    }
    Ok(())
}

