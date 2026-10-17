//! Behaviour every `SettingsStore` adapter shares. An adapter's tests run it
//! with `settings_store_contract_tests!(with_store)`, where `with_store` runs a
//! test against a fresh, empty store.

use crate::settings::domain::app_settings::AppSettings;
use crate::settings::domain::ports::SettingsStore;

fn settings_with_assign_me(assign_me_to_tasks: bool) -> AppSettings {
    AppSettings { assign_me_to_tasks }
}

pub fn loads_the_defaults_when_empty(store: &dyn SettingsStore) {
    assert_eq!(store.load().unwrap(), AppSettings::default());
}

pub fn saves_the_settings(store: &dyn SettingsStore) {
    let settings = settings_with_assign_me(false);

    store.save(&settings).unwrap();

    assert_eq!(store.load().unwrap(), settings);
}

pub fn save_replaces_the_stored_settings(store: &dyn SettingsStore) {
    store.save(&settings_with_assign_me(false)).unwrap();
    let replacement = settings_with_assign_me(true);

    store.save(&replacement).unwrap();

    assert_eq!(store.load().unwrap(), replacement);
}

/// Generates one `#[test]` per contract behaviour, each run through
/// `$with_store: fn(fn(&dyn SettingsStore))`.
macro_rules! settings_store_contract_tests {
    ($with_store:ident) => {
        $crate::settings::infra::settings_store_contract::settings_store_contract_tests!(
            @tests $with_store;
            loads_the_defaults_when_empty,
            saves_the_settings,
            save_replaces_the_stored_settings,
        );
    };
    (@tests $with_store:ident; $($behaviour:ident),* $(,)?) => {
        mod contract {
            $(
                #[test]
                fn $behaviour() {
                    super::$with_store(
                        $crate::settings::infra::settings_store_contract::$behaviour,
                    );
                }
            )*
        }
    };
}

pub(crate) use settings_store_contract_tests;
