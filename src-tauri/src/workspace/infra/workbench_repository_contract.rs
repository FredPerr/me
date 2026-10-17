//! Behaviour every `WorkbenchRepository` adapter shares. An adapter's tests run
//! it with `workbench_repository_contract_tests!(with_repository)`, where
//! `with_repository` runs a test against a fresh, empty repository.

use crate::workspace::domain::ports::WorkbenchRepository;
use crate::workspace::domain::project_tag::ProjectTag;
use crate::workspace::domain::workbench::{Placement, Workbench, WorkbenchEntry};

const PINNED_TAG: &str = "alpha";
const SHOWN_TAG: &str = "beta";
const REPLACEMENT_TAG: &str = "gamma";

fn tag(raw: &str) -> ProjectTag {
    ProjectTag::parse(raw).unwrap()
}

fn entry(raw: &str, placement: Placement) -> WorkbenchEntry {
    WorkbenchEntry {
        project_tag: tag(raw),
        placement,
    }
}

fn arranged_workbench() -> Workbench {
    Workbench {
        entries: vec![
            entry(PINNED_TAG, Placement::Pinned),
            entry(SHOWN_TAG, Placement::Shown),
        ],
        last_project_tag: Some(tag(SHOWN_TAG)),
    }
}

pub fn load_returns_the_default_when_there_is_no_record(repository: &dyn WorkbenchRepository) {
    let loaded = repository.load().unwrap();

    assert_eq!(loaded, Workbench::default());
}

pub fn save_then_load_round_trips_the_workbench(repository: &dyn WorkbenchRepository) {
    let workbench = arranged_workbench();

    repository.save(&workbench).unwrap();

    assert_eq!(repository.load().unwrap(), workbench);
}

pub fn save_replaces_the_previous_workbench(repository: &dyn WorkbenchRepository) {
    repository.save(&arranged_workbench()).unwrap();
    let replacement = Workbench {
        entries: vec![entry(REPLACEMENT_TAG, Placement::Shown)],
        last_project_tag: None,
    };

    repository.save(&replacement).unwrap();

    assert_eq!(repository.load().unwrap(), replacement);
}

/// Generates one `#[test]` per contract behaviour, each run through
/// `$with_repository: fn(fn(&dyn WorkbenchRepository))`.
macro_rules! workbench_repository_contract_tests {
    ($with_repository:ident) => {
        $crate::workspace::infra::workbench_repository_contract::workbench_repository_contract_tests!(
            @tests $with_repository;
            load_returns_the_default_when_there_is_no_record,
            save_then_load_round_trips_the_workbench,
            save_replaces_the_previous_workbench,
        );
    };
    (@tests $with_repository:ident; $($behaviour:ident),* $(,)?) => {
        mod contract {
            $(
                #[test]
                fn $behaviour() {
                    super::$with_repository(
                        $crate::workspace::infra::workbench_repository_contract::$behaviour,
                    );
                }
            )*
        }
    };
}

pub(crate) use workbench_repository_contract_tests;
