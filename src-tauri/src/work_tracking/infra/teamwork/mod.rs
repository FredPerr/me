//! Read-only Teamwork API v3 adapter (anti-corruption layer).
mod adapter;
mod client;
mod dto;
mod mapper;
mod web_urls;

use crate::work_tracking::domain::api_key::ApiKey;
use crate::work_tracking::domain::error::WorkTrackingError;
use crate::work_tracking::domain::ports::WorkTracker;
use crate::work_tracking::domain::provider_connection::ProviderConnection;

pub fn create(
    connection: &ProviderConnection,
    api_key: ApiKey,
) -> Result<Box<dyn WorkTracker>, WorkTrackingError> {
    let client = client::TeamworkClient::new(connection.base_url(), api_key)?;
    Ok(Box::new(adapter::TeamworkWorkTracker::new(
        client,
        connection.id().clone(),
        connection.base_url().clone(),
    )))
}
