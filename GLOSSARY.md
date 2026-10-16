# me

A desktop app for working on several git branches of a multi-repository project at once, each set of branches checked out side by side and handed to AI coding agents.

## Language

### Projects

**Project**:
A named, local codebase made of one or more repositories that are worked on together.
_Avoid_: Workspace, app

**Workbench**:
Your arrangement of projects in the app: which ones are shown, in what order, and which are pinned as small icons or hidden. One per app; it is not part of any project.
_Avoid_: Workspace, layout, project bar

**Repository**:
One git repository that belongs to a project. A project with a single repository at its root still has exactly one repository.
_Avoid_: Repo (in prose), sub-project

### Contexts

**Context**:
A named set of checkouts, one per repository of a project, each on its own branch, used to work on one change. Always means this; the DDD sense is written "bounded context".
_Avoid_: Workspace, environment, sandbox

**Default context**:
The context that is the project's own main checkouts, on each repository's default branch. Every project has exactly one.
_Avoid_: Main context, root context

**Base context**:
The context a new context branches from: each of the new context's branches starts from the base context's branch in the same repository.
_Avoid_: Parent context, source context

**Context branch**:
The branch a context uses in one repository. Every repository of a context has one, even when nothing is changed there.
_Avoid_: Linked repository

**Static context**:
A context that stays in place on the board and is not moved through statuses. The default context is always static.

**Context status**:
Where a context stands in the flow of work: idle, ready, in progress, needs human, review, or done.
_Avoid_: State, column

**Context origin**:
Where a context came from: made by hand, from another context, from existing branches, from pull requests, or from a task.
_Avoid_: Source, template

**Agent work**:
What an AI coding agent is doing in a context: the prompt to start from and the agent's own conversation to resume.
_Avoid_: Kiro session (Kiro is one agent among others)

**Agent run**:
One run of an AI coding agent in a context, from a prompt to its end: finished, failed, stopped or interrupted. A context keeps its latest runs and their transcripts.
_Avoid_: Session, job

**AI assist**:
A short AI answer you review before using, such as suggested contexts or a suggested pull request title and description. Unlike an agent run, it changes no files and is not kept.
_Avoid_: Generated draft, suggestion

### Integrations

**Git host**:
A service where a repository's remote lives and pull requests are opened, such as GitHub or Bitbucket. You sign in to each with one account.
_Avoid_: Git provider, forge

**Work tracker**:
A service where tasks are tracked, such as Teamwork or Jira.
_Avoid_: Provider, PM tool

**Work tracker connection**:
Your sign-in to one work tracker site.

**Remote project**:
A project on a work tracker. It holds tasks, optionally sorted into task groups.
_Avoid_: Work project, tracker project

**Remote project link**:
A remote project attached to a Project, so its tasks can be shown on the board and turned into contexts.

**Task**:
One unit of work on a work tracker: a Teamwork task, a Jira or GitHub issue.
_Avoid_: Work item, issue, ticket

**Task group**:
An optional grouping of tasks within a remote project: a Teamwork task list, a Jira epic or sprint, a GitHub milestone.
_Avoid_: Task list

**Subtask**:
A task whose parent is another task.

**Task status**:
Where a task stands on its work tracker: to do, in progress, done, or cancelled. Unrelated to Context status.
