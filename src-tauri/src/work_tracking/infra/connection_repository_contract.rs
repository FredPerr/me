//! Behaviour every `ConnectionRepository` adapter shares. An adapter's tests
//! run it with `connection_repository_contract_tests!(with_repository)`, where
//! `with_repository` runs a test against a fresh, empty store.

use crate::work_tracking::domain::base_url::BaseUrl;
use crate::work_tracking::domain::error::InputField;
use crate::work_tracking::domain::identifiers::ConnectionId;
use crate::work_tracking::domain::ports::ConnectionRepository;
use crate::work_tracking::domain::provider_connection::{DisplayName, ProviderConnection};
use crate::work_tracking::domain::provider_kind::ProviderKind;

const ACME_HOST: &str = "acme.teamwork.com";
const OTHER_HOST: &str = "other.teamwork.com";

fn connection(host: &str, display_name: Option<&str>) -> ProviderConnection {
    let base_url = BaseUrl::parse(&format!("https://{host}")).unwrap();
    let display = DisplayName::parse(display_name, &base_url).unwrap();
    ProviderConnection::new(ProviderKind::Teamwork, base_url, display)
}

fn id_of(host: &str) -> ConnectionId {
    ConnectionId::parse(&format!("teamwork:{host}"), InputField::ConnectionId).unwrap()
}

pub fn lists_nothing_when_empty(repository: &dyn ConnectionRepository) {
    assert_eq!(repository.list().unwrap(), vec![]);
}

pub fn saves_a_connection(repository: &dyn ConnectionRepository) {
    let acme = connection(ACME_HOST, None);

    repository.save(&acme).unwrap();

    assert_eq!(repository.list().unwrap(), vec![acme]);
}

pub fn save_upserts_by_id(repository: &dyn ConnectionRepository) {
    repository
        .save(&connection(ACME_HOST, Some("Acme")))
        .unwrap();
    let renamed = connection(ACME_HOST, Some("Renamed"));

    repository.save(&renamed).unwrap();

    assert_eq!(repository.list().unwrap(), vec![renamed]);
}

pub fn lists_every_saved_connection(repository: &dyn ConnectionRepository) {
    let acme = connection(ACME_HOST, None);
    let other = connection(OTHER_HOST, None);

    repository.save(&acme).unwrap();
    repository.save(&other).unwrap();

    let mut listed = repository.list().unwrap();
    listed.sort_by_key(|connection| connection.id().as_str().to_string());
    assert_eq!(listed, vec![acme, other]);
}

pub fn finds_a_connection_by_id(repository: &dyn ConnectionRepository) {
    let acme = connection(ACME_HOST, None);
    repository.save(&acme).unwrap();
    repository.save(&connection(OTHER_HOST, None)).unwrap();

    assert_eq!(repository.find(&id_of(ACME_HOST)).unwrap(), Some(acme));
}

pub fn finds_nothing_for_an_unknown_id(repository: &dyn ConnectionRepository) {
    repository.save(&connection(ACME_HOST, None)).unwrap();

    assert_eq!(repository.find(&id_of(OTHER_HOST)).unwrap(), None);
}

pub fn removes_a_connection(repository: &dyn ConnectionRepository) {
    let other = connection(OTHER_HOST, None);
    repository.save(&connection(ACME_HOST, None)).unwrap();
    repository.save(&other).unwrap();

    repository.remove(&id_of(ACME_HOST)).unwrap();

    assert_eq!(repository.list().unwrap(), vec![other]);
}

pub fn removing_a_missing_connection_succeeds(repository: &dyn ConnectionRepository) {
    repository.remove(&id_of(ACME_HOST)).unwrap();
    repository.remove(&id_of(ACME_HOST)).unwrap();

    assert_eq!(repository.find(&id_of(ACME_HOST)).unwrap(), None);
}

/// Generates one `#[test]` per contract behaviour, each run through
/// `$with_repository: fn(fn(&dyn ConnectionRepository))`.
macro_rules! connection_repository_contract_tests {
    ($with_repository:ident) => {
        $crate::work_tracking::infra::connection_repository_contract::connection_repository_contract_tests!(
            @tests $with_repository;
            lists_nothing_when_empty,
            saves_a_connection,
            save_upserts_by_id,
            lists_every_saved_connection,
            finds_a_connection_by_id,
            finds_nothing_for_an_unknown_id,
            removes_a_connection,
            removing_a_missing_connection_succeeds,
        );
    };
    (@tests $with_repository:ident; $($behaviour:ident),* $(,)?) => {
        mod contract {
            $(
                #[test]
                fn $behaviour() {
                    super::$with_repository(
                        $crate::work_tracking::infra::connection_repository_contract::$behaviour,
                    );
                }
            )*
        }
    };
}

pub(crate) use connection_repository_contract_tests;
