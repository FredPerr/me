//! Pluggable adapters for AI CLI tools.
//!
//! Each supported AI CLI (Kiro today, others later) is described by a
//! [`CliAdapter`]. An adapter is a thin, declarative description of *how* to
//! invoke a given CLI in a non-interactive/headless way: which binary to look
//! for, which subcommand and flags turn it into a one-shot streaming run, and
//! which environment variables to forward.
//!
//! The orchestration machinery (process registry, streaming, kill) is adapter
//! agnostic. To support a new CLI you add one adapter and register it in
//! [`AdapterRegistry::builtin`] — no changes to the spawning/streaming code.

use crate::binary_resolver;
use std::collections::HashMap;
use std::path::PathBuf;

/// A fully resolved plan for spawning a single AI CLI run. Produced by an
/// adapter from a [`SpawnRequest`] and consumed by the process runner.
#[derive(Debug, Clone)]
pub struct SpawnSpec {
    /// Absolute path to the resolved binary.
    pub binary: PathBuf,
    /// Arguments passed to the binary, in order.
    pub arguments: Vec<String>,
    /// Working directory for the process.
    pub working_directory: String,
    /// Extra environment variables to set for the child process.
    pub environment: Vec<(String, String)>,
    /// When set, this string is written to the child's stdin and stdin is then
    /// closed. Used by adapters that prefer passing the prompt over stdin.
    pub stdin_payload: Option<String>,
}

/// The caller's intent for a run, independent of any particular CLI.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    /// The natural-language prompt / instruction to run headlessly.
    pub prompt: String,
    /// Directory the CLI should operate in (typically a worktree path).
    pub working_directory: String,
    /// Optional named agent/profile to run, when the CLI supports it.
    pub agent: Option<String>,
    /// Optional id of a prior conversation to resume, so the run continues with
    /// the earlier context instead of starting fresh. Honored by adapters whose
    /// CLI supports resuming (Kiro's `--resume-id`).
    pub resume_id: Option<String>,
    /// Optional login shell used to resolve the binary via the user's PATH.
    pub shell: Option<String>,
    /// Additional environment variables supplied by the caller (e.g. an API
    /// key for headless auth). Merged after the adapter's own defaults.
    pub environment: Vec<(String, String)>,
    /// When true, ask the CLI to run tools without per-action confirmation.
    /// Headless runs cannot answer interactive prompts, so this is normally on;
    /// a workspace permission policy still provides the safety floor.
    pub trust_all_tools: bool,
}

/// How an adapter prefers to deliver the prompt to its CLI.
///
/// `Stdin` is a supported delivery mode for adapters that read the prompt from
/// standard input; the built-in Kiro adapter uses `Argument`, so the variant is
/// exercised only by other/future adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PromptDelivery {
    /// Prompt is appended as a positional argument.
    Argument,
    /// Prompt is written to stdin, which is then closed.
    Stdin,
}

/// Describes a single AI CLI and how to drive it in headless mode.
pub trait CliAdapter: Send + Sync {
    /// Stable identifier used on the wire (e.g. `"kiro"`). Must be unique.
    fn id(&self) -> &'static str;

    /// Human-readable label for the UI (e.g. `"Kiro CLI"`).
    fn display_name(&self) -> &'static str;

    /// The default command name to resolve on PATH when the user has not
    /// configured an explicit binary (e.g. `"kiro-cli"`).
    fn default_command(&self) -> &'static str;

    /// Subcommand + flags that precede the prompt to run headlessly
    /// (e.g. `["chat", "--no-interactive"]`).
    fn headless_arguments(&self) -> Vec<String>;

    /// How this CLI receives the prompt.
    fn prompt_delivery(&self) -> PromptDelivery;

    /// Whether to emit an end-of-options separator (`--`) before the positional
    /// prompt argument. CLIs built on clap-style parsers otherwise treat a
    /// prompt that starts with `-` (e.g. a Markdown bullet list) as an unknown
    /// option and reject the run. Only meaningful for [`PromptDelivery::Argument`].
    fn prompt_needs_options_terminator(&self) -> bool {
        false
    }

    /// Builds the flag that selects a named agent/profile, if supported.
    fn agent_arguments(&self, _agent: &str) -> Vec<String> {
        Vec::new()
    }

    /// Builds the flag that resumes a prior conversation by id, if the CLI
    /// supports it. Empty by default, so an unsupported CLI ignores the request.
    fn resume_arguments(&self, _resume_id: &str) -> Vec<String> {
        Vec::new()
    }

    /// Flags that grant the model permission to run tools without per-action
    /// confirmation. Returned only when the caller requests trust; empty by
    /// default so a CLI without such a flag simply ignores the request.
    fn trust_all_arguments(&self) -> Vec<String> {
        Vec::new()
    }

    /// Resolves a concrete [`SpawnSpec`] for the request, locating the binary.
    ///
    /// `command_override` lets the caller point at a specific binary/path
    /// (mirroring how the IDE command is configurable); when `None` the
    /// adapter's [`default_command`](Self::default_command) is used.
    fn build_spec(
        &self,
        request: &SpawnRequest,
        command_override: Option<&str>,
    ) -> Result<SpawnSpec, String> {
        let command = command_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| self.default_command());

        let binary =
            binary_resolver::resolve(command, request.shell.as_deref()).ok_or_else(|| {
                format!(
                    "Could not locate the {} binary ('{}') on this system",
                    self.display_name(),
                    command
                )
            })?;

        let mut arguments = self.headless_arguments();
        if let Some(agent) = request.agent.as_deref().filter(|value| !value.is_empty()) {
            arguments.extend(self.agent_arguments(agent));
        }
        if let Some(resume_id) = request.resume_id.as_deref().filter(|value| !value.is_empty()) {
            arguments.extend(self.resume_arguments(resume_id));
        }
        if request.trust_all_tools {
            arguments.extend(self.trust_all_arguments());
        }

        let stdin_payload = match self.prompt_delivery() {
            PromptDelivery::Argument => {
                // Terminate option parsing so a prompt beginning with `-` is
                // treated as the positional prompt rather than a flag.
                if self.prompt_needs_options_terminator() {
                    arguments.push("--".to_string());
                }
                arguments.push(request.prompt.clone());
                None
            }
            PromptDelivery::Stdin => Some(request.prompt.clone()),
        };

        Ok(SpawnSpec {
            binary,
            arguments,
            working_directory: request.working_directory.clone(),
            environment: request.environment.clone(),
            stdin_payload,
        })
    }
}

/// Metadata describing an available adapter, surfaced to the frontend so the UI
/// can offer the list of supported CLIs without hardcoding it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterInfo {
    pub id: String,
    pub display_name: String,
    pub default_command: String,
}

/// Holds the set of known adapters, keyed by their stable id.
pub struct AdapterRegistry {
    adapters: HashMap<&'static str, Box<dyn CliAdapter>>,
}

impl AdapterRegistry {
    /// Builds the registry with every adapter the app ships with.
    pub fn builtin() -> Self {
        let mut registry = Self {
            adapters: HashMap::new(),
        };
        registry.register(Box::new(super::kiro::KiroAdapter));
        registry
    }

    fn register(&mut self, adapter: Box<dyn CliAdapter>) {
        self.adapters.insert(adapter.id(), adapter);
    }

    /// Looks up an adapter by id.
    pub fn get(&self, id: &str) -> Option<&dyn CliAdapter> {
        self.adapters.get(id).map(|boxed| boxed.as_ref())
    }

    /// Lists metadata for every registered adapter, sorted by display name for
    /// a stable UI ordering.
    pub fn list(&self) -> Vec<AdapterInfo> {
        let mut infos: Vec<AdapterInfo> = self
            .adapters
            .values()
            .map(|adapter| AdapterInfo {
                id: adapter.id().to_string(),
                display_name: adapter.display_name().to_string(),
                default_command: adapter.default_command().to_string(),
            })
            .collect();
        infos.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        infos
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}
