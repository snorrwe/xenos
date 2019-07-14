use super::{ExecutionResult, Task, TaskGraph};

pub fn run_selector<T>(state: &mut TaskGraph<T>, nodes: &[Task<T>]) -> ExecutionResult {
    let found = nodes
        .iter()
        .map(|node| node.tick(state))
        .find(|result| result.is_err());
    if let Some(found) = found {
        Err(format!("Task failure running selector {:?}", found))?;
    }
    Ok(())
}

pub fn run_sequence<T>(state: &mut TaskGraph<T>, nodes: &[Task<T>]) -> ExecutionResult {
    let found = nodes.iter().any(|node| {
        let result = node.tick(state);
        debug!("Task result in sequence {:?}", result);
        result.is_ok()
    });
    if found {
        Ok(())
    } else {
        Err(format!("All tasks failed in sequence"))
    }
}

