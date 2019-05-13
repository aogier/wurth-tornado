use crate::error::ApiError;
use futures::Future;
use tornado_common_api::Event;
use tornado_engine_matcher::config::MatcherConfig;
use tornado_engine_matcher::error::MatcherError;
use tornado_engine_matcher::model::ProcessedEvent;

/// The ApiHandler trait defines the contract that a struct has to respect to
/// be used by the backend.
/// It permits to decouple the backend from a specific implementation.
pub trait ApiHandler {
    fn read(&self) -> Result<MatcherConfig, MatcherError>;
    fn send_event(&self, event: Event) -> Box<Future<Item = ProcessedEvent, Error = ApiError>>;
}
