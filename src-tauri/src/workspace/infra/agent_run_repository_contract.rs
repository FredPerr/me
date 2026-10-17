//! Behaviour every `AgentRunRepository` adapter shares. An adapter's tests run
//! it with `agent_run_repository_contract_tests!(with_repository)`, where
//! `with_repository` runs a test against a fresh, empty repository.

use crate::workspace::domain::agent_run::{
    AgentRun, AgentRunState, MAX_RUNS_PER_CONTEXT, MAX_TRANSCRIPT_BYTES,
};
use crate::workspace::domain::identifiers::{AgentRunId, ContextId};
use crate::workspace::domain::ports::AgentRunRepository;

const CONTEXT: &str = "context-1";
const OTHER_CONTEXT: &str = "context-2";
const PROMPT: &str = "Build the login page";
const STARTED_AT: &str = "2024-01-15T10:00:00.000Z";
const FIRST_LINE: &str = "compiling";
const SECOND_LINE: &str = "done";

fn context_id(raw: &str) -> ContextId {
    ContextId::parse(raw).unwrap()
}

fn run_in(context: &ContextId) -> AgentRun {
    AgentRun::started(
        AgentRunId::generate(),
        context.clone(),
        PROMPT.to_string(),
        STARTED_AT.to_string(),
    )
}

fn line_of(bytes: usize) -> String {
    "x".repeat(bytes)
}

pub fn records_a_run_and_lists_it(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let run = run_in(&context);

    repository.start_run(&run).unwrap();

    assert_eq!(repository.list_runs(&context).unwrap(), vec![run]);
}

pub fn lists_nothing_for_a_context_without_runs(repository: &dyn AgentRunRepository) {
    assert_eq!(repository.list_runs(&context_id(CONTEXT)).unwrap(), vec![]);
}

pub fn appends_lines_to_a_run(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let run = run_in(&context);
    repository.start_run(&run).unwrap();

    repository
        .append_lines(
            &context,
            &run.id,
            &[FIRST_LINE.to_string(), SECOND_LINE.to_string()],
        )
        .unwrap();

    let stored = repository.list_runs(&context).unwrap();
    assert_eq!(stored[0].output_lines, vec![FIRST_LINE, SECOND_LINE]);
}

pub fn the_size_cap_drops_the_oldest_lines_and_sets_trimmed(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let run = run_in(&context);
    repository.start_run(&run).unwrap();
    let half = line_of(MAX_TRANSCRIPT_BYTES / 2);

    repository
        .append_lines(&context, &run.id, &["oldest".to_string(), half.clone()])
        .unwrap();
    repository
        .append_lines(&context, &run.id, std::slice::from_ref(&half))
        .unwrap();

    let stored = &repository.list_runs(&context).unwrap()[0];
    assert!(stored.trimmed);
    assert_eq!(stored.output_lines, vec![half.clone(), half]);
    assert!(stored.transcript_bytes() <= MAX_TRANSCRIPT_BYTES);
}

pub fn the_last_ten_cap_drops_the_oldest_run(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let mut ids = Vec::new();
    for _ in 0..MAX_RUNS_PER_CONTEXT + 1 {
        let run = run_in(&context);
        ids.push(run.id.clone());
        repository.start_run(&run).unwrap();
    }

    let stored = repository.list_runs(&context).unwrap();

    assert_eq!(stored.len(), MAX_RUNS_PER_CONTEXT);
    assert_eq!(stored.first().unwrap().id, ids[1]);
    assert_eq!(stored.last().unwrap().id, *ids.last().unwrap());
}

pub fn lists_a_contexts_runs_oldest_first(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let first = run_in(&context);
    let second = run_in(&context);
    repository.start_run(&first).unwrap();
    repository.start_run(&second).unwrap();

    let stored = repository.list_runs(&context).unwrap();

    assert_eq!(
        stored.iter().map(|run| &run.id).collect::<Vec<_>>(),
        vec![&first.id, &second.id]
    );
}

pub fn finishing_a_run_records_its_state_and_exit_code(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let run = run_in(&context);
    repository.start_run(&run).unwrap();

    repository
        .finish_run(&context, &run.id, AgentRunState::Finished, Some(0))
        .unwrap();

    let stored = &repository.list_runs(&context).unwrap()[0];
    assert_eq!(stored.state, AgentRunState::Finished);
    assert_eq!(stored.exit_code, Some(0));
}

pub fn keeps_each_contexts_runs_apart(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let other = context_id(OTHER_CONTEXT);
    let run = run_in(&context);
    let other_run = run_in(&other);
    repository.start_run(&run).unwrap();
    repository.start_run(&other_run).unwrap();

    assert_eq!(repository.list_runs(&context).unwrap(), vec![run]);
    assert_eq!(repository.list_runs(&other).unwrap(), vec![other_run]);
}

pub fn deletes_all_runs_of_a_context(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let other = context_id(OTHER_CONTEXT);
    repository.start_run(&run_in(&context)).unwrap();
    let other_run = run_in(&other);
    repository.start_run(&other_run).unwrap();

    repository.delete_runs(&context).unwrap();

    assert_eq!(repository.list_runs(&context).unwrap(), vec![]);
    assert_eq!(repository.list_runs(&other).unwrap(), vec![other_run]);
}

pub fn deleting_a_context_without_runs_succeeds(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);

    repository.delete_runs(&context).unwrap();
    repository.delete_runs(&context).unwrap();

    assert_eq!(repository.list_runs(&context).unwrap(), vec![]);
}

pub fn finds_runs_left_running(repository: &dyn AgentRunRepository) {
    let context = context_id(CONTEXT);
    let other = context_id(OTHER_CONTEXT);
    let running = run_in(&context);
    let ended = run_in(&context);
    let other_running = run_in(&other);
    repository.start_run(&running).unwrap();
    repository.start_run(&ended).unwrap();
    repository.start_run(&other_running).unwrap();
    repository
        .finish_run(&context, &ended.id, AgentRunState::Finished, Some(0))
        .unwrap();

    let running_ids: Vec<AgentRunId> = repository
        .running_runs()
        .unwrap()
        .into_iter()
        .map(|run| run.id)
        .collect();

    assert!(running_ids.contains(&running.id));
    assert!(running_ids.contains(&other_running.id));
    assert!(!running_ids.contains(&ended.id));
}

/// Generates one `#[test]` per contract behaviour, each run through
/// `$with_repository: fn(fn(&dyn AgentRunRepository))`.
macro_rules! agent_run_repository_contract_tests {
    ($with_repository:ident) => {
        $crate::workspace::infra::agent_run_repository_contract::agent_run_repository_contract_tests!(
            @tests $with_repository;
            records_a_run_and_lists_it,
            lists_nothing_for_a_context_without_runs,
            appends_lines_to_a_run,
            the_size_cap_drops_the_oldest_lines_and_sets_trimmed,
            the_last_ten_cap_drops_the_oldest_run,
            lists_a_contexts_runs_oldest_first,
            finishing_a_run_records_its_state_and_exit_code,
            keeps_each_contexts_runs_apart,
            deletes_all_runs_of_a_context,
            deleting_a_context_without_runs_succeeds,
            finds_runs_left_running,
        );
    };
    (@tests $with_repository:ident; $($behaviour:ident),* $(,)?) => {
        mod agent_run_contract {
            $(
                #[test]
                fn $behaviour() {
                    super::$with_repository(
                        $crate::workspace::infra::agent_run_repository_contract::$behaviour,
                    );
                }
            )*
        }
    };
}

pub(crate) use agent_run_repository_contract_tests;
