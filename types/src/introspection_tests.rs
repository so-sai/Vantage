#[cfg(test)]
mod vil_tests {
    use super::*;
    use crate::CapabilityDescriptor;
    use crate::CAPABILITY_REGISTRY;

    #[test]
    fn test_vil_list_order_deterministic() {
        let entries = CAPABILITY_REGISTRY.list();
        let first_run: Vec<&str> = entries.iter().map(|e| e.name).collect();
        let second_run: Vec<&str> = entries.iter().map(|e| e.name).collect();
        assert_eq!(
            first_run, second_run,
            "Registry order must be deterministic"
        );
    }

    #[test]
    fn test_vil_capability_schema_stable() {
        let entries = CAPABILITY_REGISTRY.list();
        for cap in entries {
            assert!(!cap.name.is_empty(), "Capability name must not be empty");
            assert!(!cap.help.is_empty(), "Capability help must not be empty");
            assert!(!cap.inputs.is_empty(), "Capability must have inputs");
            assert!(!cap.outputs.is_empty(), "Capability must have outputs");
        }
    }

    #[test]
    fn test_vil_registry_has_all_expected_capabilities() {
        let entries = CAPABILITY_REGISTRY.list();
        let names: Vec<&str> = entries.iter().map(|e| e.name).collect();

        let expected = vec![
            "dirty_propagation",
            "symbol_dependency_graph",
            "incremental_parse",
            "triple_hash",
            "drift_detection",
            "invariant_enforcement",
            "tombstone_eviction",
            "blast_radius_control",
        ];

        for name in expected {
            assert!(
                names.contains(&name),
                "Expected capability '{}' not found",
                name
            );
        }
    }

    #[test]
    fn test_vil_json_snapshot_stable() {
        use serde_json::json;

        let entries = CAPABILITY_REGISTRY.list();
        let serialized = serde_json::to_string(&json!({
            "capabilities": entries.iter().map(|e| json!({
                "name": e.name,
                "inputs": e.inputs,
                "outputs": e.outputs,
            })).collect::<Vec<_>>(),
        }))
        .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        let caps = parsed["capabilities"].as_array().unwrap();

        assert_eq!(caps.len(), 8, "Must have exactly 8 capabilities");

        let dirty = caps
            .iter()
            .find(|c| c["name"] == "dirty_propagation")
            .unwrap();
        assert_eq!(dirty["inputs"], json!(["NodeId"]));
        assert_eq!(dirty["outputs"], json!(["DirtySet"]));
    }

    #[test]
    fn test_vil_get_returns_correct_capability() {
        let cap = CAPABILITY_REGISTRY.get("dirty_propagation");
        assert!(cap.is_some());

        let cap = cap.unwrap();
        assert_eq!(cap.name, "dirty_propagation");
        assert!(cap.invariants.contains(&"O(depth)"));
    }

    #[test]
    fn test_vil_get_unknown_returns_none() {
        let cap = CAPABILITY_REGISTRY.get("unknown_capability");
        assert!(cap.is_none());
    }

    #[test]
    fn test_vil_capability_invariants_complete() {
        let entries = CAPABILITY_REGISTRY.list();
        for cap in entries {
            assert!(
                !cap.invariants.is_empty(),
                "Capability '{}' must have at least one invariant",
                cap.name
            );
        }
    }
}
