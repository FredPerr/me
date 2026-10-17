use super::app_settings::AppSettings;

/// The settings store could not be used. The reason is logged by the adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsStoreUnavailable;

/// The one app-settings record. `load` falls back to defaults when the record
/// is missing or unreadable, so only a storage failure is an error.
pub trait SettingsStore: Send + Sync {
    /// The stored settings, or [`AppSettings::default`] when none are stored.
    fn load(&self) -> Result<AppSettings, SettingsStoreUnavailable>;
    /// Replaces the stored settings.
    fn save(&self, settings: &AppSettings) -> Result<(), SettingsStoreUnavailable>;
}
