//! The best-effort "assign me to these tasks" use case's model: the tasks to
//! assign the current user to and the tasks that could not be assigned.
//!
//! Assignment is tied to the create-contexts-from-tasks flow but kept separate
//! from context creation: contexts are always created, and this reports which
//! tasks the user could not be added to so they can assign themselves by hand.

use super::identifiers::{ConnectionId, TaskId};

/// One task to assign the current user to, on its connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskToAssign {
    pub connection_id: ConnectionId,
    pub task_id: TaskId,
}

/// A task the current user could not be assigned to, kept so the frontend can
/// list it. The connection and task ids are enough for the frontend to match
/// the failure back to its card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedAssignment {
    pub connection_id: ConnectionId,
    pub task_id: TaskId,
}

/// The outcome of a best-effort assign-me batch: the tasks that could not be
/// assigned. An empty list means every task was assigned.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AssignMeResult {
    pub failed: Vec<FailedAssignment>,
}
