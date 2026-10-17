//! The Workbench: one per app, the user's arrangement of Projects on the tab
//! bar. It holds an ordered list of entries, each a project tag with a
//! placement, and the tag of the last-open project. It is a pure value: the
//! redb store persists it and the service (ticket #94) drives it; reconcile and
//! the placement/reorder mutators live here so the arrangement rules are tested
//! without a command or the UI.

use serde::{Deserialize, Serialize};

use super::project_tag::ProjectTag;

/// Where a project sits on the tab bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    /// An icon-only tab at the left.
    Pinned,
    /// A normal tab.
    Shown,
    /// Not a tab; reachable through the "+N" button.
    Hidden,
}

/// One project's place in the arrangement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkbenchEntry {
    pub project_tag: ProjectTag,
    pub placement: Placement,
}

/// The whole arrangement. `#[serde(default)]` on each field so a record written
/// before a field existed still loads as the empty default for that field.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workbench {
    #[serde(default)]
    pub entries: Vec<WorkbenchEntry>,
    /// The project the app reopens on; `None` falls back to the first shown tab.
    #[serde(default)]
    pub last_project_tag: Option<ProjectTag>,
}

impl Workbench {
    /// Folds the arrangement against the live project list so add, delete and
    /// "unreadable" all stay consistent without the Project use cases writing to
    /// the Workbench. `live_project_tags` is the readable project list, in its
    /// tag-alphabetical order; a deleted or unreadable project is simply absent
    /// from it.
    ///
    /// - Keeps existing entries whose tag is still live, in their stored order.
    /// - Appends any live tag with no entry as `Shown`, in project-list order,
    ///   so a new project lands at the end deterministically.
    /// - Drops entries whose tag is not live (covers deleted and unreadable).
    /// - Clears `last_project_tag` when it is not live or its entry is `Hidden`,
    ///   so the frontend falls back to the first shown tab.
    pub fn reconcile(self, live_project_tags: &[ProjectTag]) -> Self {
        let kept: Vec<WorkbenchEntry> = self
            .entries
            .into_iter()
            .filter(|entry| live_project_tags.contains(&entry.project_tag))
            .collect();
        let appended: Vec<WorkbenchEntry> = live_project_tags
            .iter()
            .filter(|tag| !kept.iter().any(|entry| &entry.project_tag == *tag))
            .map(|tag| WorkbenchEntry {
                project_tag: tag.clone(),
                placement: Placement::Shown,
            })
            .collect();
        let entries: Vec<WorkbenchEntry> = kept.into_iter().chain(appended).collect();

        let last_project_tag = self.last_project_tag.filter(|tag| {
            entries
                .iter()
                .any(|entry| &entry.project_tag == tag && entry.placement != Placement::Hidden)
        });

        Self {
            entries,
            last_project_tag,
        }
    }

    /// Sets a project's placement. Pin and hide set it in place. "Show" and
    /// "unpin" both resolve to `Shown`; when they actually change the placement
    /// the entry moves to the end so it lands as a normal tab at the end, as the
    /// spec says ("Show returns a project as a normal tab at the end"). Setting
    /// a placement an entry already has leaves the order untouched. Does nothing
    /// for a tag with no entry.
    pub fn set_placement(mut self, project_tag: &ProjectTag, placement: Placement) -> Self {
        let Some(position) = self
            .entries
            .iter()
            .position(|entry| &entry.project_tag == project_tag)
        else {
            return self;
        };
        if self.entries[position].placement == placement {
            return self;
        }
        if placement == Placement::Shown {
            let mut entry = self.entries.remove(position);
            entry.placement = placement;
            self.entries.push(entry);
        } else {
            self.entries[position].placement = placement;
        }
        self
    }

    /// Reorders the entries of one placement group to match `ordered_tags`,
    /// leaving the other groups where they are. Tags in `ordered_tags` that are
    /// not in the group are ignored; group entries not named in `ordered_tags`
    /// keep their relative order after the named ones.
    pub fn reorder_within(mut self, placement: Placement, ordered_tags: &[ProjectTag]) -> Self {
        let mut group: Vec<WorkbenchEntry> = self
            .entries
            .iter()
            .filter(|entry| entry.placement == placement)
            .cloned()
            .collect();
        group.sort_by_key(|entry| {
            ordered_tags
                .iter()
                .position(|tag| tag == &entry.project_tag)
                .unwrap_or(usize::MAX)
        });

        let mut reordered = group.into_iter();
        self.entries = self
            .entries
            .into_iter()
            .map(|entry| {
                if entry.placement == placement {
                    reordered.next().unwrap_or(entry)
                } else {
                    entry
                }
            })
            .collect();
        self
    }

    /// Sets the last-open project. The caller passes a tag that is present and
    /// not hidden; reconcile clears a tag that later becomes gone or hidden.
    pub fn set_last_project(mut self, project_tag: Option<ProjectTag>) -> Self {
        self.last_project_tag = project_tag;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALPHA: &str = "alpha";
    const BETA: &str = "beta";
    const GAMMA: &str = "gamma";
    const DELTA: &str = "delta";

    fn tag(raw: &str) -> ProjectTag {
        ProjectTag::parse(raw).unwrap()
    }

    fn entry(raw: &str, placement: Placement) -> WorkbenchEntry {
        WorkbenchEntry {
            project_tag: tag(raw),
            placement,
        }
    }

    fn workbench(entries: Vec<WorkbenchEntry>, last: Option<&str>) -> Workbench {
        Workbench {
            entries,
            last_project_tag: last.map(tag),
        }
    }

    fn placements(workbench: &Workbench) -> Vec<(&str, Placement)> {
        workbench
            .entries
            .iter()
            .map(|entry| (entry.project_tag.as_str(), entry.placement))
            .collect()
    }

    mod reconcile {
        use super::*;

        #[test]
        fn folds_a_new_project_in_as_shown_at_the_end() {
            // Given an arrangement missing a live project
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Pinned),
                    entry(BETA, Placement::Hidden),
                ],
                None,
            );

            // When reconciled against the live list that adds gamma
            let after = before.reconcile(&[tag(ALPHA), tag(BETA), tag(GAMMA)]);

            // Then the new project is appended as a normal tab
            assert_eq!(
                placements(&after),
                vec![
                    (ALPHA, Placement::Pinned),
                    (BETA, Placement::Hidden),
                    (GAMMA, Placement::Shown),
                ]
            );
        }

        #[test]
        fn drops_an_entry_whose_project_is_gone() {
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Shown),
                    entry(BETA, Placement::Pinned),
                ],
                None,
            );

            let after = before.reconcile(&[tag(ALPHA)]);

            assert_eq!(placements(&after), vec![(ALPHA, Placement::Shown)]);
        }

        #[test]
        fn drops_an_unreadable_project_like_a_gone_one() {
            // An unreadable project is simply absent from the live list, so it
            // is dropped the same way a deleted one is.
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Shown),
                    entry(BETA, Placement::Shown),
                ],
                None,
            );

            let after = before.reconcile(&[tag(ALPHA)]);

            assert_eq!(placements(&after), vec![(ALPHA, Placement::Shown)]);
        }

        #[test]
        fn clears_the_last_open_tag_when_its_project_is_gone() {
            let before = workbench(vec![entry(ALPHA, Placement::Shown)], Some(BETA));

            let after = before.reconcile(&[tag(ALPHA)]);

            assert_eq!(after.last_project_tag, None);
        }

        #[test]
        fn clears_the_last_open_tag_when_its_project_is_hidden() {
            let before = workbench(vec![entry(ALPHA, Placement::Hidden)], Some(ALPHA));

            let after = before.reconcile(&[tag(ALPHA)]);

            assert_eq!(after.last_project_tag, None);
        }

        #[test]
        fn keeps_a_last_open_tag_that_is_still_shown() {
            let before = workbench(vec![entry(ALPHA, Placement::Shown)], Some(ALPHA));

            let after = before.reconcile(&[tag(ALPHA)]);

            assert_eq!(after.last_project_tag, Some(tag(ALPHA)));
        }

        #[test]
        fn preserves_the_stored_order_of_kept_entries() {
            // The stored order (gamma, alpha) differs from the live list's
            // alphabetical order; kept entries stay in their stored order and
            // the new one (beta) is appended last.
            let before = workbench(
                vec![
                    entry(GAMMA, Placement::Shown),
                    entry(ALPHA, Placement::Pinned),
                ],
                None,
            );

            let after = before.reconcile(&[tag(ALPHA), tag(BETA), tag(GAMMA)]);

            assert_eq!(
                placements(&after),
                vec![
                    (GAMMA, Placement::Shown),
                    (ALPHA, Placement::Pinned),
                    (BETA, Placement::Shown),
                ]
            );
        }
    }

    mod set_placement {
        use super::*;

        #[test]
        fn pinning_sets_the_placement_in_place() {
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Shown),
                    entry(BETA, Placement::Shown),
                ],
                None,
            );

            let after = before.set_placement(&tag(ALPHA), Placement::Pinned);

            assert_eq!(
                placements(&after),
                vec![(ALPHA, Placement::Pinned), (BETA, Placement::Shown)]
            );
        }

        #[test]
        fn hiding_sets_the_placement_in_place() {
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Shown),
                    entry(BETA, Placement::Shown),
                ],
                None,
            );

            let after = before.set_placement(&tag(ALPHA), Placement::Hidden);

            assert_eq!(
                placements(&after),
                vec![(ALPHA, Placement::Hidden), (BETA, Placement::Shown)]
            );
        }

        #[test]
        fn showing_moves_the_entry_to_the_end() {
            // Unpinning or showing lands the project as a normal tab at the end.
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Pinned),
                    entry(BETA, Placement::Shown),
                    entry(GAMMA, Placement::Shown),
                ],
                None,
            );

            let after = before.set_placement(&tag(ALPHA), Placement::Shown);

            assert_eq!(
                placements(&after),
                vec![
                    (BETA, Placement::Shown),
                    (GAMMA, Placement::Shown),
                    (ALPHA, Placement::Shown),
                ]
            );
        }

        #[test]
        fn setting_the_same_placement_leaves_the_order_untouched() {
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Shown),
                    entry(BETA, Placement::Shown),
                ],
                None,
            );

            let after = before.set_placement(&tag(ALPHA), Placement::Shown);

            assert_eq!(
                placements(&after),
                vec![(ALPHA, Placement::Shown), (BETA, Placement::Shown)]
            );
        }

        #[test]
        fn does_nothing_for_a_tag_with_no_entry() {
            let before = workbench(vec![entry(ALPHA, Placement::Shown)], None);

            let after = before.set_placement(&tag(BETA), Placement::Pinned);

            assert_eq!(placements(&after), vec![(ALPHA, Placement::Shown)]);
        }
    }

    mod reorder_within {
        use super::*;

        #[test]
        fn reorders_only_the_named_group() {
            // Pinned reorder among pinned; the shown tab stays put.
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Pinned),
                    entry(BETA, Placement::Shown),
                    entry(GAMMA, Placement::Pinned),
                ],
                None,
            );

            let after = before.reorder_within(Placement::Pinned, &[tag(GAMMA), tag(ALPHA)]);

            assert_eq!(
                placements(&after),
                vec![
                    (GAMMA, Placement::Pinned),
                    (BETA, Placement::Shown),
                    (ALPHA, Placement::Pinned),
                ]
            );
        }

        #[test]
        fn keeps_unnamed_group_entries_after_the_named_ones() {
            let before = workbench(
                vec![
                    entry(ALPHA, Placement::Shown),
                    entry(BETA, Placement::Shown),
                    entry(GAMMA, Placement::Shown),
                ],
                None,
            );

            let after = before.reorder_within(Placement::Shown, &[tag(GAMMA)]);

            assert_eq!(
                placements(&after),
                vec![
                    (GAMMA, Placement::Shown),
                    (ALPHA, Placement::Shown),
                    (BETA, Placement::Shown),
                ]
            );
        }
    }

    mod set_last_project {
        use super::*;

        #[test]
        fn sets_the_last_open_tag() {
            let before = workbench(vec![entry(ALPHA, Placement::Shown)], None);

            let after = before.set_last_project(Some(tag(ALPHA)));

            assert_eq!(after.last_project_tag, Some(tag(ALPHA)));
        }

        #[test]
        fn clears_the_last_open_tag() {
            let before = workbench(vec![entry(ALPHA, Placement::Shown)], Some(ALPHA));

            let after = before.set_last_project(None);

            assert_eq!(after.last_project_tag, None);
        }
    }

    #[test]
    fn a_record_written_before_the_last_project_field_existed_still_loads() {
        let older_record = serde_json::json!({
            "entries": [{ "projectTag": DELTA, "placement": "shown" }]
        });
        let bytes = rmp_serde::to_vec_named(&older_record).unwrap();

        let workbench: Workbench = rmp_serde::from_slice(&bytes).unwrap();

        assert_eq!(placements(&workbench), vec![(DELTA, Placement::Shown)]);
        assert_eq!(workbench.last_project_tag, None);
    }
}
