//! Where a Project's checkouts live on disk, computed from its settings only:
//! nothing here touches the filesystem. Paths starting with `~` are kept as
//! they are; expanding them belongs to the adapters.

use super::context::Context;
use super::context_branch::ContextBranch;
use super::project::Project;
use super::repository::Repository;

const WORKTREES_FOLDER: &str = ".worktrees";

impl Project {
    /// `<project>/.worktrees/<context folder>`.
    pub fn context_folder_path(&self, context: &Context) -> String {
        format!(
            "{}/{WORKTREES_FOLDER}/{}",
            self.folder_path(),
            context.folder.as_str()
        )
    }

    /// The default context uses the repository's own folder; any other
    /// context has one checkout per repository in its context folder.
    pub fn worktree_path(&self, context: &Context, repository: &Repository) -> String {
        if context.is_default {
            self.repository_path(repository)
        } else {
            format!("{}/{}", self.context_folder_path(context), repository.name)
        }
    }

    /// `None` when the branch is on a repository the project no longer has.
    pub fn context_branch_worktree_path(
        &self,
        context: &Context,
        branch: &ContextBranch,
    ) -> Option<String> {
        self.find_effective_repository(&branch.repository)
            .map(|repository| self.worktree_path(context, &repository))
    }

    /// The repository's folder in the project.
    pub fn repository_path(&self, repository: &Repository) -> String {
        let project_folder = self.folder_path();
        let relative_path = repository.relative_path.as_str();
        if relative_path.starts_with('/') || relative_path.starts_with('~') {
            return relative_path.to_string();
        }
        match relative_path.strip_prefix("./").unwrap_or(relative_path) {
            "" | "." => project_folder.to_string(),
            inside => format!("{project_folder}/{inside}"),
        }
    }

    /// The folders to open the context in, best first: the repository's
    /// checkout when the context has a single known repository, then the
    /// context folder, then the project folder.
    pub fn ide_path_candidates(&self, context: &Context) -> Vec<String> {
        let known_branches: Vec<String> = context
            .branches
            .iter()
            .filter_map(|branch| self.context_branch_worktree_path(context, branch))
            .collect();
        let mut candidates = Vec::new();
        if let [single_checkout] = known_branches.as_slice() {
            candidates.push(single_checkout.clone());
        }
        candidates.push(self.context_folder_path(context));
        candidates.push(self.folder_path().to_string());
        without_duplicates(candidates)
    }

    pub fn ide_path(&self, context: &Context) -> String {
        self.ide_path_candidates(context)
            .into_iter()
            .next()
            .expect("there is always at least the project folder")
    }

    /// The folders to open the project itself in, best first.
    pub fn root_ide_path_candidates(&self) -> Vec<String> {
        let project_folder = self.folder_path().to_string();
        match self.effective_repositories().as_ref() {
            [single_repository] => without_duplicates(vec![
                self.repository_path(single_repository),
                project_folder,
            ]),
            _ => vec![project_folder],
        }
    }

    fn folder_path(&self) -> &str {
        let path = self.path();
        if path.len() > 1 {
            path.strip_suffix('/').unwrap_or(path)
        } else {
            path
        }
    }
}

fn without_duplicates(paths: Vec<String>) -> Vec<String> {
    let mut unique: Vec<String> = Vec::with_capacity(paths.len());
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::domain::context_folder::ContextFolder;
    use crate::workspace::domain::context_name::ContextName;
    use crate::workspace::domain::project::ProjectSettings;
    use crate::workspace::domain::project_tag::ProjectTag;
    use crate::workspace::domain::test_support::*;

    const API_ID: &str = "repo-1";
    const WEB_ID: &str = "repo-2";
    const BRANCH: &str = "feature";
    const CONTEXT_NAME: &str = "feature-x";

    fn context_folder() -> String {
        format!("{PROJECT_PATH}/.worktrees/{CONTEXT_NAME}")
    }

    fn api() -> Repository {
        repository(API_ID, "api", "./api")
    }

    fn web() -> Repository {
        repository(WEB_ID, "web", "./web")
    }

    fn context_on(name: &str, repository_ids: &[&str]) -> Context {
        Context {
            branches: repository_ids
                .iter()
                .map(|repository| branch_on(repository, BRANCH))
                .collect(),
            ..context_named(name)
        }
    }

    fn default_context_on(repository_ids: &[&str]) -> Context {
        Context {
            branches: context_on(CONTEXT_NAME, repository_ids).branches,
            ..Context::new_default(vec![])
        }
    }

    fn legacy_context(name: &str) -> Context {
        let name = ContextName::parse(name).unwrap();
        Context {
            folder: ContextFolder::legacy(&name),
            name,
            ..context_named(CONTEXT_NAME)
        }
    }

    mod context_folder_path {
        use super::*;

        #[test]
        fn uses_the_raw_name_of_a_legacy_context() {
            let project = project_with(vec![]);
            assert_eq!(
                project.context_folder_path(&legacy_context("My Feature")),
                format!("{PROJECT_PATH}/.worktrees/My Feature")
            );
        }

        #[test]
        fn uses_the_slug_of_a_new_context() {
            let project = project_with(vec![]);
            assert_eq!(
                project.context_folder_path(&context_named("My Feature")),
                format!("{PROJECT_PATH}/.worktrees/my-feature")
            );
        }
    }

    mod worktree_path {
        use super::*;

        #[test]
        fn is_in_the_context_folder_under_the_repository_name() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.worktree_path(&context_named("My Feature"), &api()),
                format!("{PROJECT_PATH}/.worktrees/my-feature/api")
            );
        }

        #[test]
        fn is_the_project_folder_for_the_root_repository_of_the_default_context() {
            let project = project_with(vec![]);
            let root = project.effective_repositories()[0].clone();
            assert_eq!(
                project.worktree_path(&default_context_on(&[PROJECT_TAG]), &root),
                PROJECT_PATH
            );
        }

        #[test]
        fn is_the_repository_folder_for_the_default_context() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.worktree_path(&default_context_on(&[API_ID]), &api()),
                format!("{PROJECT_PATH}/api")
            );
        }

        #[test]
        fn is_unknown_for_a_branch_on_an_unknown_repository() {
            let project = project_with(vec![api()]);
            let context = context_on(CONTEXT_NAME, &["missing"]);
            assert_eq!(
                project.context_branch_worktree_path(&context, &context.branches[0]),
                None
            );
        }
    }

    mod repository_path {
        use super::*;

        #[test]
        fn resolves_relative_paths_in_the_project_folder() {
            let project = project_with(vec![]);
            let cases = [
                ("./api", format!("{PROJECT_PATH}/api")),
                ("api", format!("{PROJECT_PATH}/api")),
                (".", PROJECT_PATH.to_string()),
                ("./", PROJECT_PATH.to_string()),
                ("", PROJECT_PATH.to_string()),
            ];
            for (relative_path, expected) in cases {
                let repository = repository(API_ID, "api", relative_path);
                assert_eq!(
                    project.repository_path(&repository),
                    expected,
                    "{relative_path:?}"
                );
            }
        }

        #[test]
        fn keeps_an_absolute_path() {
            let project = project_with(vec![]);
            let repository = repository(API_ID, "api", "/elsewhere/api");
            assert_eq!(project.repository_path(&repository), "/elsewhere/api");
        }
    }

    mod ide_path_candidates {
        use super::*;

        #[test]
        fn start_with_the_worktree_when_the_context_has_a_single_repository() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.ide_path_candidates(&context_on(CONTEXT_NAME, &[API_ID])),
                [
                    format!("{}/api", context_folder()),
                    context_folder(),
                    PROJECT_PATH.to_string()
                ]
            );
        }

        #[test]
        fn start_with_the_context_folder_when_the_context_has_several_repositories() {
            let project = project_with(vec![api(), web()]);
            assert_eq!(
                project.ide_path_candidates(&context_on(CONTEXT_NAME, &[API_ID, WEB_ID])),
                [context_folder(), PROJECT_PATH.to_string()]
            );
        }

        #[test]
        fn start_with_the_context_folder_when_the_context_has_no_repositories() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.ide_path_candidates(&context_on(CONTEXT_NAME, &[])),
                [context_folder(), PROJECT_PATH.to_string()]
            );
        }

        #[test]
        fn ignore_branches_on_unknown_repositories() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.ide_path_candidates(&context_on(CONTEXT_NAME, &["missing"])),
                [context_folder(), PROJECT_PATH.to_string()]
            );
        }

        #[test]
        fn use_the_project_name_for_the_worktree_of_a_single_repository_project() {
            let project = project_with(vec![]);
            assert_eq!(
                project.ide_path_candidates(&context_on(CONTEXT_NAME, &[PROJECT_TAG])),
                [
                    format!("{}/{PROJECT_NAME}", context_folder()),
                    context_folder(),
                    PROJECT_PATH.to_string()
                ]
            );
        }

        #[test]
        fn start_with_the_repository_folder_for_the_default_context() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.ide_path(&default_context_on(&[API_ID])),
                format!("{PROJECT_PATH}/api")
            );
        }

        #[test]
        fn start_with_the_project_folder_for_the_default_context_of_a_root_repository() {
            let project = project_with(vec![]);
            let default_context = Context {
                folder: ContextFolder::legacy(&ContextName::parse("main").unwrap()),
                ..default_context_on(&[PROJECT_TAG])
            };
            assert_eq!(
                project.ide_path_candidates(&default_context),
                [
                    PROJECT_PATH.to_string(),
                    format!("{PROJECT_PATH}/.worktrees/main")
                ]
            );
        }

        #[test]
        fn ignore_a_trailing_slash_in_the_project_path() {
            let settings = ProjectSettings {
                path: format!("{PROJECT_PATH}/"),
                ..settings(vec![api()])
            };
            let project = Project::new(ProjectTag::parse(PROJECT_TAG).unwrap(), settings);
            assert_eq!(
                project.ide_path_candidates(&context_on(CONTEXT_NAME, &[API_ID])),
                [
                    format!("{}/api", context_folder()),
                    context_folder(),
                    PROJECT_PATH.to_string()
                ]
            );
        }

        #[test]
        fn the_ide_path_is_the_first_candidate() {
            let project = project_with(vec![api(), web()]);
            let context = context_on(CONTEXT_NAME, &[API_ID, WEB_ID]);
            assert_eq!(project.ide_path(&context), context_folder());
        }
    }

    mod root_ide_path_candidates {
        use super::*;

        #[test]
        fn start_with_the_repository_folder_when_the_project_has_a_single_repository() {
            let project = project_with(vec![api()]);
            assert_eq!(
                project.root_ide_path_candidates(),
                [format!("{PROJECT_PATH}/api"), PROJECT_PATH.to_string()]
            );
        }

        #[test]
        fn are_the_project_folder_when_the_project_has_several_repositories() {
            let project = project_with(vec![api(), web()]);
            assert_eq!(project.root_ide_path_candidates(), [PROJECT_PATH]);
        }

        #[test]
        fn are_the_project_folder_when_the_project_has_no_configured_repositories() {
            let project = project_with(vec![]);
            assert_eq!(project.root_ide_path_candidates(), [PROJECT_PATH]);
        }
    }
}
