//! Move resources
//!
use super::super::bt::*;
use super::{get_energy, harvest, move_to, repairer, upgrader};
use screeps::{
    constants::ResourceType,
    game::get_object_erased,
    objects::{
        Creep, StructureExtension, StructureSpawn, StructureStorage, StructureTower, Transferable,
    },
    prelude::*,
    ReturnCode,
};
use stdweb::{unstable::TryFrom, Reference};

pub fn run<'a>(creep: &'a Creep) -> Task<'a> {
    trace!("Running gofer {}", creep.name());
    let tasks = vec![
        Task::new(move |_| attempt_unload(creep)),
        Task::new(move |_| get_energy(creep)),
        Task::new(move |_| harvest(creep)),
        Task::new(move |_| attempt_unload(creep)),
        // Fallback
        Task::new(move |_| repairer::attempt_repair(creep)),
        Task::new(move |_| upgrader::attempt_upgrade(creep)),
    ];

    let tree = Control::Sequence(tasks);
    Task::new(move |_| tree.tick())
}

fn attempt_unload<'a>(creep: &'a Creep) -> ExecutionResult {
    trace!("Unloading");
    let loading: bool = creep.memory().bool("loading");
    if loading {
        return Err("loading".into());
    }

    let carry_total = creep.carry_total();

    if carry_total == 0 {
        trace!("Empty");
        creep.memory().set("loading", true);
        return Err("empty".into());
    }

    let target = find_unload_target(creep).ok_or_else(|| String::new())?;

    let tasks = vec![
        Task::new(|_| try_transfer::<StructureSpawn>(creep, &target)),
        Task::new(|_| try_transfer::<StructureExtension>(creep, &target)),
        Task::new(|_| try_transfer::<StructureTower>(creep, &target)),
        Task::new(|_| try_transfer::<StructureStorage>(creep, &target)),
    ]
    .into_iter()
    .collect();

    let tree = Control::Sequence(tasks);
    tree.tick().map_err(|e| {
        creep.memory().del("target");
        e
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
            Task::new(|_| find_unload_target_by_type(creep, "storage")),
        ]
        .into_iter()
        .collect();
        let tree = Control::Sequence(tasks);
        tree.tick().unwrap_or_else(|e| {
            debug!("Failed to find unload target {:?}", e);
            creep.memory().del("target");
        });
        None
    }
}

fn try_transfer<'a, T>(creep: &'a Creep, target: &'a Reference) -> ExecutionResult
where
    T: Transferable + screeps::traits::TryFrom<&'a Reference>,
{
    let target = T::try_from(target.as_ref())
        .map_err(|_| String::from("failed to convert transfer target"))?;
    transfer(creep, &target)
}

fn find_unload_target_by_type<'a>(creep: &'a Creep, struct_type: &'a str) -> ExecutionResult {
    let res = js! {
        const creep = @{creep};
        const exts = creep.room.find(FIND_STRUCTURES, {
            filter: function (s) {
                return s.structureType == @{struct_type} && s.energy < s.energyCapacity;
            }
        });
        return exts[0] && exts[0].id;
    };
    let target = String::try_from(res).map_err(|_| String::from("expected string"))?;
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

