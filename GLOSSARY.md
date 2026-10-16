# me

A desktop app for working on several git branches of a multi-repository project at once, each set of branches checked out side by side and handed to AI coding agents.

## Language

### Projects

**Project**:
A named, local codebase made of one or more repositories that are worked on together.
_Avoid_: Workspace, app

**Repository**:
One git repository that belongs to a project. A project with a single repository at its root still has exactly one repository.
_Avoid_: Repo (in prose), sub-project

### Contexts

**Context**:
A named set of checkouts, one per repository of a project, used to work on one change. Always means this; the DDD sense is written "bounded context".
_Avoid_: Workspace, environment, sandbox

**Default context**:
The context that is the project's own main checkouts, on each repository's default branch. Every project has exactly one.
_Avoid_: Main context, root context

**Base context**:
The context a new context branches from: each of the new context's branches starts from the base context's branch in the same repository.
_Avoid_: Parent context, source context

**Context branch**:
The branch a context uses in one repository.

**Linked repository**:
A repository in a context that reuses the base context's checkout instead of getting its own branch.
_Avoid_: Shared repository

**Static context**:
A context that stays in place on the board and is not moved through statuses. The default context is always static.

**Context status**:
Where a context stands in the flow of work: idle, ready, in progress, needs human, review, or done.
_Avoid_: State, column
