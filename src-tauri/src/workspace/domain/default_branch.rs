use crate::git::domain::branch_name::BranchName;

const PREFERRED_DEFAULT_BRANCHES: [&str; 2] = ["main", "master"];

/// The branch the default context uses in a repository it has none for:
/// "main", else "master", else the first branch. Only used to create or
/// repair the default context; afterwards the default context's branch is the
/// repository's default branch.
pub fn choose_default_branch(branches: &[BranchName]) -> Option<&BranchName> {
    PREFERRED_DEFAULT_BRANCHES
        .iter()
        .find_map(|preferred| branches.iter().find(|branch| branch.as_str() == *preferred))
        .or_else(|| branches.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branches(names: &[&str]) -> Vec<BranchName> {
        names
            .iter()
            .map(|name| BranchName::parse(name).unwrap())
            .collect()
    }

    fn chosen(names: &[&str]) -> Option<String> {
        choose_default_branch(&branches(names)).map(ToString::to_string)
    }

    #[test]
    fn prefers_main() {
        assert_eq!(
            chosen(&["develop", "master", "main"]).as_deref(),
            Some("main")
        );
    }

    #[test]
    fn falls_back_to_master() {
        assert_eq!(chosen(&["develop", "master"]).as_deref(), Some("master"));
    }

    #[test]
    fn falls_back_to_the_first_branch() {
        assert_eq!(chosen(&["develop", "trunk"]).as_deref(), Some("develop"));
    }

    #[test]
    fn chooses_nothing_without_branches() {
        assert_eq!(chosen(&[]), None);
    }
}
