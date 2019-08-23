use super::task::*;
use super::*;

/// Run an iterator of tasks as a Selector
pub fn selector<'a, T: 'a + TaskInput, It: Iterator<Item = &'a Task<'a, T>>>(
    state: &'a mut T,
    tasks: It,
) -> ExecutionResult {
    let found = tasks
        .map(|node| (node, node.tick(state)))
        .find(|(_node, result)| result.is_err());
    if let Some(found) = found {
        Err(format!("A task failed in Selector {:?}", found.1))?;
    }
    Ok(())
}

/// Run an iterator of tasks as a Sequence
pub fn sequence<'a, T: 'a + TaskInput, It: Iterator<Item = &'a Task<'a, T>>>(
    state: &'a mut T,
    mut tasks: It,
) -> ExecutionResult {
    let found = tasks.any(|node| {
        let result = node.tick(state);
        debug!(
            "Task result in sequence node: {:?} result: {:?}",
            node, result
        );
        result.is_ok()
    });
    if found {
        Ok(())
    } else {
        Err("All tasks failed in Sequence!")?
    }
}

