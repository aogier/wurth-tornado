use log::*;
use tornado_engine_matcher::model::InternalEvent;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tornado_common_api::Value;
use tornado_common::actors::message::TornadoCommonActorError;


#[derive(Serialize, Deserialize)]
struct Timestamps {
    #[serde(with = "serde_regex")]
    pattern: Regex,
}

pub enum NatsExtractor{
    /// Uses a regular expression to extract the tenant_id from the subject name
    TenantIdFromSubject {
        regex: Regex
    }
}

impl NatsExtractor {

    fn process(&self, subject: &str, mut event: InternalEvent) -> Result<InternalEvent, TornadoCommonActorError> {
        match self {
            NatsExtractor::TenantIdFromSubject { regex } => {
                match regex.captures(subject).and_then(|captures| captures.get(1)) {
                    Some(tenant_id_match) => {
                        let tenant_id = tenant_id_match.as_str();
                        trace!("tenant_id [{}] extracted from Nats subject [{}]", tenant_id, subject);
                        event.add_to_metadata("tenant_id".to_owned(), Value::Text(tenant_id.to_owned())).map_err(|err| TornadoCommonActorError::GenericError { message: format! {"{}", err} })?;
                        Ok(event)
                    },
                    None => {
                        debug!("Cannot extract tenant_id from Nats subject [{}]", subject);
                        Ok(event)
                    }
                }
            }
        }
    }

}

#[cfg(test)]
mod test {
    use tornado_engine_matcher::model::InternalEvent;
    use crate::enrich::nats::NatsExtractor;
    use regex::Regex;

    #[test]
    fn should_extract_the_tenant_id() {
        // Arrange
        let original_event = InternalEvent::new(Default::default());

        let extractor = NatsExtractor::TenantIdFromSubject {
            regex: Regex::new("(.*)\\.tornado\\.events").unwrap()
        };

        // Act
        let event = extractor.process("MY.TENANT_ID.tornado.events", original_event).unwrap();

        // Assert
        let tenant_id = event.metadata.get_from_map("tenant_id").and_then(|val| val.get_text());
        assert_eq!(Some("MY.TENANT_ID"), tenant_id);
    }

    #[test]
    fn should_ignore_missing_tenant_id() {
        // Arrange
        let original_event = InternalEvent::new(Default::default());

        let extractor = NatsExtractor::TenantIdFromSubject {
            regex: Regex::new("(.*)\\.tornado\\.events").unwrap()
        };

        // Act
        let event = extractor.process("hello.world", original_event.clone()).unwrap();

        // Assert
        assert_eq!(original_event, event);

        let tenant_id = event.metadata.get_from_map("tenant_id").and_then(|val| val.get_text());
        assert!(tenant_id.is_none());

    }

}