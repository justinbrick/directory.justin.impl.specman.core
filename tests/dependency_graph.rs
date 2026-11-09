//! Integration tests for dependency mapping services, conflict-resolution primitives,
//! and the Part 3 key entities (`DataModelAdapter`, `DependencyTree`, `TemplateDescriptor`
//! schema, `WorkspaceStatusConfig`, and friends).

mod common;

use specman::{
    ArtifactId, ArtifactKind, ArtifactSummary, DataModelAdapter, DependencyEdge,
    DependencyRelation, DependencyTree, InMemoryAdapter, StatusResult, ValidationType,
    WorkspaceStatusConfig,
};

fn artifact(kind: ArtifactKind, name: &str) -> ArtifactId {
    ArtifactId {
        kind,
        name: name.to_string(),
    }
}

#[test]
fn artifact_id_display_reports_kind_and_name() {
    let id = artifact(ArtifactKind::Specification, "core");
    assert_eq!(id.to_string(), "Specification://core");
    let impl_id = artifact(ArtifactKind::Implementation, "lib");
    assert_eq!(impl_id.to_string(), "Implementation://lib");
}

#[test]
fn dependency_tree_has_upstream_downstream_aggregate_views() {
    let root = ArtifactSummary {
        id: artifact(ArtifactKind::Specification, "root"),
        metadata: Default::default(),
        resolved_path: None,
    };
    let up = ArtifactSummary {
        id: artifact(ArtifactKind::Specification, "up"),
        metadata: Default::default(),
        resolved_path: None,
    };
    let down = ArtifactSummary {
        id: artifact(ArtifactKind::Implementation, "down"),
        metadata: Default::default(),
        resolved_path: None,
    };

    let tree = DependencyTree {
        root: root.clone(),
        upstream: vec![DependencyEdge {
            from: root.clone(),
            to: up,
            relation: DependencyRelation::Upstream,
            optional: false,
        }],
        downstream: vec![DependencyEdge {
            from: down.clone(),
            to: root.clone(),
            relation: DependencyRelation::Downstream,
            optional: false,
        }],
        aggregate: vec![],
    };

    assert_eq!(tree.upstream.len(), 1);
    assert_eq!(tree.downstream.len(), 1);
    // Downstream entry is non-optional -> blocks deletion.
    assert!(tree.has_blocking_dependents());
}

#[test]
fn dependency_tree_optional_dependents_do_not_block() {
    let root = ArtifactSummary {
        id: artifact(ArtifactKind::Specification, "root"),
        metadata: Default::default(),
        resolved_path: None,
    };
    let tree = DependencyTree {
        root: root.clone(),
        upstream: vec![],
        downstream: vec![DependencyEdge {
            from: ArtifactSummary {
                id: artifact(ArtifactKind::Implementation, "opt"),
                metadata: Default::default(),
                resolved_path: None,
            },
            to: root.clone(),
            relation: DependencyRelation::Downstream,
            optional: true,
        }],
        aggregate: vec![],
    };
    assert!(!tree.has_blocking_dependents());
}

#[test]
fn in_memory_adapter_saves_loads_invalidates() {
    let adapter = InMemoryAdapter::new();
    let root = ArtifactSummary {
        id: artifact(ArtifactKind::Specification, "root"),
        metadata: Default::default(),
        resolved_path: None,
    };
    let tree = DependencyTree::empty(root.clone());

    let key = root.id.clone();
    assert!(adapter.load_dependency_tree(&key).unwrap().is_none());
    adapter.save_dependency_tree(tree).unwrap();
    assert!(adapter.load_dependency_tree(&key).unwrap().is_some());
    adapter.invalidate_dependency_tree(&key).unwrap();
    assert!(adapter.load_dependency_tree(&key).unwrap().is_none());
}

#[test]
fn workspace_status_config_defaults_to_all_enabled() {
    let cfg = WorkspaceStatusConfig::default();
    assert!(cfg.structure);
    assert!(cfg.references);
    assert!(cfg.cycles);
    assert!(cfg.compliance);
}

#[test]
fn status_result_is_pass_fail_enum() {
    assert_ne!(StatusResult::Pass, StatusResult::Fail);
    // Default is Pass.
    assert_eq!(StatusResult::default(), StatusResult::Pass);
}

#[test]
fn validation_types_are_distinct() {
    assert_ne!(ValidationType::Test, ValidationType::Check);
    assert_ne!(ValidationType::Test, ValidationType::Manual);
    assert_ne!(ValidationType::Check, ValidationType::Manual);
}
