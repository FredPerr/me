//! WorkbenchService tests on the in-memory Workbench and project repositories
//! and the recording events fake (seam A). They check what callers see: the
//! reconciled order and placements, the last-open tag, and that a save emits
//! the change.
use std::sync::Arc;

use super::*;
use crate::workspace::domain::branch_naming::BranchNamingPolicy;
use crate::workspace::domain::ports::{ProjectRepository, WorkbenchEvents, WorkbenchRepository};
use crate::workspace::domain::project::{Project, ProjectSettings};
use crate::workspace::domain::workbench::{Placement, Workbench, WorkbenchEntry};
use crate::workspace::infra::in_memory_project_repository::InMemoryProjectRepository;
use crate::workspace::infra::in_memory_workbench_repository::InMemoryWorkbenchRepository;
use crate::workspace::infra::recording_fakes::RecordingWorkbenchEvents;

const ALPHA: &str = "alpha";
const BETA: &str = "beta";
const GAMMA: &str = "gamma";
const PROJECT_PATH: &str = "/work/project";

fn tag(raw: &str) -> ProjectTag {
    ProjectTag::parse(raw).unwrap()
}

fn project(tag_raw: &str) -> Project {
    let settings = ProjectSettings {
        name: tag_raw.to_string(),
        path: PROJECT_PATH.to_string(),
        icon: None,
        repositories: vec![],
        symlinks: vec![],
        remote_project_links: vec![],
        branch_naming: BranchNamingPolicy::default(),
    };
    Project::new(tag(tag_raw), settings)
}

fn entry(tag_raw: &str, placement: Placement) -> WorkbenchEntry {
    WorkbenchEntry {
        project_tag: tag(tag_raw),
        placement,
    }
}

fn placements(workbench: &Workbench) -> Vec<(&str, Placement)> {
    workbench
        .entries
        .iter()
        .map(|entry| (entry.project_tag.as_str(), entry.placement))
        .collect()
}

struct Fixture {
    workbench_repository: Arc<InMemoryWorkbenchRepository>,
    project_repository: Arc<InMemoryProjectRepository>,
    events: Arc<RecordingWorkbenchEvents>,
    service: WorkbenchService,
}

impl Fixture {
    /// A service over live projects `live_tags`, with no stored arrangement yet.
    fn with_projects(live_tags: &[&str]) -> Self {
        let workbench_repository = Arc::new(InMemoryWorkbenchRepository::default());
        let project_repository = Arc::new(InMemoryProjectRepository::default());
        for tag_raw in live_tags {
            project_repository.save(&project(tag_raw)).unwrap();
        }
        let events = Arc::new(RecordingWorkbenchEvents::default());
        let service = WorkbenchService::new(
            Arc::clone(&workbench_repository) as Arc<dyn WorkbenchRepository>,
            Arc::clone(&project_repository) as Arc<dyn ProjectRepository>,
            Arc::clone(&events) as Arc<dyn WorkbenchEvents>,
        );
        Self {
            workbench_repository,
            project_repository,
            events,
            service,
        }
    }

    fn seed(&self, workbench: Workbench) {
        self.workbench_repository.save(&workbench).unwrap();
    }

    fn stored(&self) -> Workbench {
        self.workbench_repository.load().unwrap()
    }
}

mod get {
    use super::*;

    #[test]
    fn reconciles_against_the_live_project_tags() {
        // Given a live project with no stored entry
        let fixture = Fixture::with_projects(&[ALPHA]);

        // When the arrangement is read
        let workbench = fixture.service.get().unwrap();

        // Then the live project is folded in as a shown tab
        assert_eq!(placements(&workbench), vec![(ALPHA, Placement::Shown)]);
    }

    #[test]
    fn does_not_save_on_read() {
        let fixture = Fixture::with_projects(&[ALPHA]);

        fixture.service.get().unwrap();

        // The read reconciled in memory only; nothing was written or emitted.
        assert_eq!(fixture.stored(), Workbench::default());
        assert_eq!(fixture.events.change_count(), 0);
    }

    #[test]
    fn drops_a_project_removed_from_the_project_repository() {
        let fixture = Fixture::with_projects(&[ALPHA, BETA]);
        fixture.seed(Workbench {
            entries: vec![
                entry(ALPHA, Placement::Shown),
                entry(BETA, Placement::Shown),
            ],
            last_project_tag: None,
        });

        fixture.project_repository.delete(&tag(BETA)).unwrap();
        let workbench = fixture.service.get().unwrap();

        assert_eq!(placements(&workbench), vec![(ALPHA, Placement::Shown)]);
    }
}

mod set_placement {
    use super::*;

    #[test]
    fn pins_a_project_and_persists_and_emits() {
        let fixture = Fixture::with_projects(&[ALPHA, BETA]);

        let workbench = fixture
            .service
            .set_placement(&tag(ALPHA), Placement::Pinned)
            .unwrap();

        assert_eq!(
            placements(&workbench),
            vec![(ALPHA, Placement::Pinned), (BETA, Placement::Shown)]
        );
        assert_eq!(fixture.stored(), workbench);
        assert_eq!(fixture.events.change_count(), 1);
    }

    #[test]
    fn unpinning_shows_the_project_at_the_end() {
        let fixture = Fixture::with_projects(&[ALPHA, BETA]);
        fixture.seed(Workbench {
            entries: vec![
                entry(ALPHA, Placement::Pinned),
                entry(BETA, Placement::Shown),
            ],
            last_project_tag: None,
        });

        let workbench = fixture
            .service
            .set_placement(&tag(ALPHA), Placement::Shown)
            .unwrap();

        assert_eq!(
            placements(&workbench),
            vec![(BETA, Placement::Shown), (ALPHA, Placement::Shown)]
        );
    }

    #[test]
    fn hides_a_project() {
        let fixture = Fixture::with_projects(&[ALPHA, BETA]);

        let workbench = fixture
            .service
            .set_placement(&tag(ALPHA), Placement::Hidden)
            .unwrap();

        assert_eq!(
            placements(&workbench),
            vec![(ALPHA, Placement::Hidden), (BETA, Placement::Shown)]
        );
    }

    #[test]
    fn showing_a_hidden_project_returns_it_as_a_tab_at_the_end() {
        let fixture = Fixture::with_projects(&[ALPHA, BETA]);
        fixture.seed(Workbench {
            entries: vec![
                entry(ALPHA, Placement::Hidden),
                entry(BETA, Placement::Shown),
            ],
            last_project_tag: None,
        });

        let workbench = fixture
            .service
            .set_placement(&tag(ALPHA), Placement::Shown)
            .unwrap();

        assert_eq!(
            placements(&workbench),
            vec![(BETA, Placement::Shown), (ALPHA, Placement::Shown)]
        );
    }
}

mod reorder_within {
    use super::*;

    #[test]
    fn reorders_a_group_and_persists() {
        let fixture = Fixture::with_projects(&[ALPHA, BETA, GAMMA]);
        fixture.seed(Workbench {
            entries: vec![
                entry(ALPHA, Placement::Shown),
                entry(BETA, Placement::Shown),
                entry(GAMMA, Placement::Shown),
            ],
            last_project_tag: None,
        });

        let workbench = fixture
            .service
            .reorder_within(Placement::Shown, &[tag(GAMMA), tag(ALPHA), tag(BETA)])
            .unwrap();

        assert_eq!(
            placements(&workbench),
            vec![
                (GAMMA, Placement::Shown),
                (ALPHA, Placement::Shown),
                (BETA, Placement::Shown),
            ]
        );
        assert_eq!(fixture.stored(), workbench);
        assert_eq!(fixture.events.change_count(), 1);
    }
}

mod set_last_project {
    use super::*;

    #[test]
    fn persists_the_last_open_project() {
        let fixture = Fixture::with_projects(&[ALPHA]);

        let workbench = fixture.service.set_last_project(Some(tag(ALPHA))).unwrap();

        assert_eq!(workbench.last_project_tag, Some(tag(ALPHA)));
        assert_eq!(fixture.stored().last_project_tag, Some(tag(ALPHA)));
        assert_eq!(fixture.events.change_count(), 1);
    }

    #[test]
    fn clears_a_dangling_last_open_tag_on_reconcile() {
        // Setting a last-open tag whose project is not live is cleared by the
        // reconcile that every mutation runs.
        let fixture = Fixture::with_projects(&[ALPHA]);

        let workbench = fixture.service.set_last_project(Some(tag(BETA))).unwrap();

        assert_eq!(workbench.last_project_tag, None);
    }
}
