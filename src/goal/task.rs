use anyhow::{bail, Result};

/// Valid task status transitions
const VALID_TRANSITIONS: &[(&str, &str)] = &[
    ("pending", "assigned"),
    ("pending", "blocked"),
    ("pending", "running"), // direct dispatch
    ("assigned", "running"),
    ("assigned", "pending"), // unassign
    ("running", "done"),
    ("running", "failed"),
    ("running", "stalled"),
    ("stalled", "running"), // resumed
    ("stalled", "failed"),
    ("stalled", "killed"),
    ("failed", "pending"), // retry
    ("blocked", "pending"), // unblocked
];

/// Check if a status transition is valid
pub fn validate_transition(from: &str, to: &str) -> Result<()> {
    if from == to {
        return Ok(());
    }

    for (valid_from, valid_to) in VALID_TRANSITIONS {
        if *valid_from == from && *valid_to == to {
            return Ok(());
        }
    }

    bail!(
        "Invalid task status transition: {} -> {}",
        from,
        to
    );
}

/// Check for dependency cycles in a task graph
pub fn has_cycle(
    task_id: &str,
    depends_on: &[String],
    all_tasks: &[(String, Vec<String>)],
) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut stack = depends_on.to_vec();

    while let Some(dep_id) = stack.pop() {
        if dep_id == task_id {
            return true;
        }
        if visited.contains(&dep_id) {
            continue;
        }
        visited.insert(dep_id.clone());

        // Find this dep's dependencies
        if let Some((_, deps)) = all_tasks.iter().find(|(id, _)| id == &dep_id) {
            stack.extend(deps.iter().cloned());
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Transition validation tests ──

    #[test]
    fn test_valid_transitions() {
        assert!(validate_transition("pending", "running").is_ok());
        assert!(validate_transition("running", "done").is_ok());
        assert!(validate_transition("failed", "pending").is_ok());
    }

    #[test]
    fn test_all_valid_transitions() {
        let valid = vec![
            ("pending", "assigned"),
            ("pending", "blocked"),
            ("pending", "running"),
            ("assigned", "running"),
            ("assigned", "pending"),
            ("running", "done"),
            ("running", "failed"),
            ("running", "stalled"),
            ("stalled", "running"),
            ("stalled", "failed"),
            ("stalled", "killed"),
            ("failed", "pending"),
            ("blocked", "pending"),
        ];
        for (from, to) in valid {
            assert!(
                validate_transition(from, to).is_ok(),
                "Expected {} -> {} to be valid",
                from,
                to
            );
        }
    }

    #[test]
    fn test_invalid_transitions() {
        assert!(validate_transition("done", "running").is_err());
        assert!(validate_transition("pending", "done").is_err());
    }

    #[test]
    fn test_more_invalid_transitions() {
        let invalid = vec![
            ("done", "pending"),
            ("done", "failed"),
            ("done", "running"),
            ("killed", "running"),
            ("killed", "pending"),
            ("blocked", "running"),
            ("blocked", "done"),
            ("pending", "failed"),
            ("pending", "stalled"),
            ("pending", "killed"),
        ];
        for (from, to) in invalid {
            assert!(
                validate_transition(from, to).is_err(),
                "Expected {} -> {} to be invalid",
                from,
                to
            );
        }
    }

    #[test]
    fn test_same_status() {
        assert!(validate_transition("running", "running").is_ok());
        assert!(validate_transition("pending", "pending").is_ok());
        assert!(validate_transition("done", "done").is_ok());
        assert!(validate_transition("failed", "failed").is_ok());
    }

    #[test]
    fn test_invalid_transition_error_message() {
        let err = validate_transition("done", "running").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("done"));
        assert!(msg.contains("running"));
        assert!(msg.contains("Invalid"));
    }

    // ── Cycle detection tests ──

    #[test]
    fn test_no_cycle() {
        let tasks = vec![
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["c".to_string()]),
            ("c".to_string(), vec![]),
        ];
        assert!(!has_cycle("a", &["b".to_string()], &tasks));
    }

    #[test]
    fn test_cycle() {
        let tasks = vec![
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["c".to_string()]),
            ("c".to_string(), vec!["a".to_string()]),
        ];
        assert!(has_cycle("a", &["b".to_string()], &tasks));
    }

    #[test]
    fn test_self_cycle() {
        let tasks = vec![("a".to_string(), vec!["a".to_string()])];
        assert!(has_cycle("a", &["a".to_string()], &tasks));
    }

    #[test]
    fn test_no_dependencies_no_cycle() {
        let tasks = vec![
            ("a".to_string(), vec![]),
            ("b".to_string(), vec![]),
        ];
        assert!(!has_cycle("a", &[], &tasks));
    }

    #[test]
    fn test_diamond_dependency_no_cycle() {
        // a depends on b and c, both depend on d
        let tasks = vec![
            ("a".to_string(), vec!["b".to_string(), "c".to_string()]),
            ("b".to_string(), vec!["d".to_string()]),
            ("c".to_string(), vec!["d".to_string()]),
            ("d".to_string(), vec![]),
        ];
        assert!(!has_cycle(
            "a",
            &["b".to_string(), "c".to_string()],
            &tasks
        ));
    }

    #[test]
    fn test_indirect_cycle() {
        // a -> b -> c -> d -> b (cycle not involving a directly)
        let tasks = vec![
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["c".to_string()]),
            ("c".to_string(), vec!["d".to_string()]),
            ("d".to_string(), vec!["b".to_string()]),
        ];
        // This doesn't cycle back to "a", so has_cycle returns false
        assert!(!has_cycle("a", &["b".to_string()], &tasks));
    }

    #[test]
    fn test_long_chain_cycle_back_to_start() {
        let tasks = vec![
            ("a".to_string(), vec!["b".to_string()]),
            ("b".to_string(), vec!["c".to_string()]),
            ("c".to_string(), vec!["d".to_string()]),
            ("d".to_string(), vec!["e".to_string()]),
            ("e".to_string(), vec!["a".to_string()]),
        ];
        assert!(has_cycle("a", &["b".to_string()], &tasks));
    }

    #[test]
    fn test_missing_dependency_no_crash() {
        // Task depends on something not in the graph
        let tasks = vec![("a".to_string(), vec!["nonexistent".to_string()])];
        assert!(!has_cycle("a", &["nonexistent".to_string()], &tasks));
    }

    #[test]
    fn test_empty_graph() {
        let tasks: Vec<(String, Vec<String>)> = vec![];
        assert!(!has_cycle("a", &[], &tasks));
    }
}
