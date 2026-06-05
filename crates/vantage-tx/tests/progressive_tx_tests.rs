use std::time::SystemTime;
use vantage_core::{KnowledgeMutation, MutationId, AgentId, ResourceId, MutationOp};
use vantage_tx::TransactionDAG;

fn mid(id: &str) -> MutationId { MutationId(id.to_string()) }
fn rid(resource: &str) -> ResourceId { ResourceId(resource.to_string()) }

fn make_mutation(id: &str, _resource: &str, op: MutationOp) -> KnowledgeMutation {
    KnowledgeMutation {
        mutation_id: mid(id),
        actor: AgentId("test-agent".into()),
        op,
        timestamp: SystemTime::now(),
    }
}

fn insert_op(resource: &str) -> MutationOp {
    MutationOp::Insert {
        resource_id: rid(resource),
        payload: "data".into(),
    }
}

fn delete_op(resource: &str) -> MutationOp {
    MutationOp::Delete {
        resource_id: rid(resource),
    }
}

#[test]
fn test_empty_mutations_compiles() {
    let dag = TransactionDAG::compile(vec![]).unwrap();
    assert!(dag.topological_sort().is_empty());
}

#[test]
fn test_independent_mutations_no_deps() {
    let mutations = vec![
        make_mutation("m1", "res_a", insert_op("res_a")),
        make_mutation("m2", "res_b", insert_op("res_b")),
    ];
    let dag = TransactionDAG::compile(mutations).unwrap();
    let order = dag.topological_sort();
    assert_eq!(order.len(), 2);
    let m1 = mid("m1");
    let m2 = mid("m2");
    assert!(dag.adjacency[&m1].is_empty());
    assert!(dag.adjacency[&m2].is_empty());
}

#[test]
fn test_raw_dependency_detected() {
    let mutations = vec![
        make_mutation("m1", "res_x", insert_op("res_x")),
        make_mutation("m2", "res_x", delete_op("res_x")),
    ];
    let dag = TransactionDAG::compile(mutations).unwrap();
    let order = dag.topological_sort();
    assert_eq!(order.len(), 2);
    let m1 = mid("m1");
    assert!(!dag.adjacency[&m1].is_empty());
}

#[test]
fn test_waw_dependency_no_cycle() {
    // WAW creates a forward edge — no cycle in sequential ordering
    let mutations = vec![
        make_mutation("m1", "res_a", delete_op("res_a")),
        make_mutation("m2", "res_a", delete_op("res_a")),
    ];
    let result = TransactionDAG::compile(mutations);
    assert!(result.is_ok());
}

#[test]
fn test_waw_dependency_detected() {
    let mutations = vec![
        make_mutation("m1", "res_z", insert_op("res_z")),
        make_mutation("m2", "res_z", insert_op("res_z")),
    ];
    let dag = TransactionDAG::compile(mutations).unwrap();
    let order = dag.topological_sort();
    assert_eq!(order.len(), 2);
    let m1 = mid("m1");
    assert!(!dag.adjacency[&m1].is_empty());
}
