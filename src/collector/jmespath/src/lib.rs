use std::collections::HashMap;
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::Event;
use tornado_common_api::Value;
use tornado_common_api::Payload;
use jmespath::Rcvar;

pub mod config;

///A collector that receives an input in JSON format and allows the creation of Events using the JMESPath JSON query language.
pub struct JMESPathEventCollector {
    processor: EventProcessor,
}

impl JMESPathEventCollector {
    pub fn build(
        config: config::JMESPathEventCollectorConfig,
    ) -> Result<JMESPathEventCollector, CollectorError> {
        let processor = EventProcessor::build(config)?;
        Ok(JMESPathEventCollector { processor })
    }
}

impl<'a> Collector<&'a str> for JMESPathEventCollector {
    fn to_event(&self, input: &'a str) -> Result<Event, CollectorError> {
        let data = jmespath::Variable::from_json(input).map_err(|err| {
            CollectorError::EventCreationError {
                message: format!("Cannot parse received json. Err: {} - Json: {}.", err, input),
            }
        })?;
        self.processor.process(data)
    }
}

struct EventProcessor {
    event_type: ValueProcessor,
    payload: EventProcessorPayload,
}

type EventProcessorPayload = HashMap<String, ValueProcessor>;

const EXPRESSION_START_DELIMITER: &str = "${";
const EXPRESSION_END_DELIMITER: &str = "}";

impl EventProcessor {
    pub fn build(
        config: config::JMESPathEventCollectorConfig,
    ) -> Result<EventProcessor, CollectorError> {
        let mut processor = EventProcessor {
            event_type: EventProcessor::build_value(Value::Text(config.event_type))?,
            payload: EventProcessorPayload::new(),
        };

        for (key, value) in config.payload {
            processor.payload.insert(key, EventProcessor::build_value(value)?);
        }

        Ok(processor)
    }

    fn build_value(value: Value) -> Result<ValueProcessor, CollectorError> {
        match value {
            // ToDo: implement Map
            Value::Map(payload) => Err(CollectorError::EventCreationError{message: "MAP not implemented yet".to_owned()}),
            // ToDo: implement Array
            Value::Array(_) => Err(CollectorError::EventCreationError{message: "ARRAY not implemented yet".to_owned()}),
            Value::Text(text) => EventProcessor::build_value_from_str(&text),
            Value::Bool(boolean) => Ok(ValueProcessor::Bool(boolean)),
            Value::Number(number) => Ok(ValueProcessor::Number(number)),
        }
    }

    fn build_value_from_str(text: &str) -> Result<ValueProcessor, CollectorError> {
        if text.starts_with(EXPRESSION_START_DELIMITER)
            && text.ends_with(EXPRESSION_END_DELIMITER)
        {
            let expression = &text
                [EXPRESSION_START_DELIMITER.len()..(text.len() - EXPRESSION_END_DELIMITER.len())];
            let jmespath_exp = jmespath::compile(expression)
                .map_err(|err| CollectorError::EventCreationError{message: format!("Not valid jmespath expression: [{}]. Err: {}", expression, err)})?;
            Ok(ValueProcessor::Expression { exp: jmespath_exp })
        } else {
            Ok(ValueProcessor::Text(text.to_owned()))
        }
    }

    pub fn process(&self, var: jmespath::Variable) -> Result<Event, CollectorError> {
        let event_type = self.event_type.process(&var)?.get_text()
            .ok_or(CollectorError::EventCreationError{message: "Event type must be a string".to_owned()})?;
        let mut event = Event::new(event_type);

        for (key, value_processor) in &self.payload {
            event.payload.insert(key.clone(), value_processor.process(&var)?);
        }

        Ok(event)
    }
}

#[derive(Debug, PartialEq)]
enum ValueProcessor {
    Expression { exp: jmespath::Expression<'static> },
    Bool(bool),
    Number(f64),
    Text(String),
    Array(Vec<ValueProcessor>),
    Map(HashMap<String, ValueProcessor>)
}

impl ValueProcessor {
    pub fn process(&self, var: &jmespath::Variable) -> Result<Value, CollectorError>
    {
        match self {
            ValueProcessor::Expression { exp } => {
                let search_result: Rcvar =
                    exp.search(var).map_err(|e| CollectorError::EventCreationError {
                        message: format!(
                            "Expression failed to execute. Exp: {}. Error: {}",
                            exp, e
                        ),
                    })?;
                variable_to_value(search_result)
            }
            ValueProcessor::Text(text) => Ok(Value::Text(text.to_owned())),
            ValueProcessor::Number(number) => Ok(Value::Number(number.clone())),
            ValueProcessor::Bool(boolean) => Ok(Value::Bool(boolean.clone())),
            // ToDo implement Map
            ValueProcessor::Map(map) => Err(CollectorError::EventCreationError{message: "ARRAY not implemented yet".to_owned()}),
            // ToDo implement Array
            ValueProcessor::Array(array) => Err(CollectorError::EventCreationError{message: "ARRAY not implemented yet".to_owned()})
        }
    }
}

fn variable_to_value(var: Rcvar) -> Result<Value, CollectorError> {
    match *var {
        jmespath::Variable::String(s) => Ok(Value::Text(s)),
        jmespath::Variable::Bool(b) => Ok(Value::Bool(b)),
        jmespath::Variable::Number(n) => Ok(Value::Number(n)),
        jmespath::Variable::Object(values) => {
            let mut payload = Payload::new();
            for (key, value) in values {
                payload.insert(key, variable_to_value(value)?);
            }
            Ok(Value::Map(payload))
        },
        jmespath::Variable::Array(values) => {
            let mut payload = vec![];
            for value in values {
                payload.push(variable_to_value(value)?);
            }
            Ok(Value::Array(payload))
        },
        // ToDo how to map null?
        jmespath::Variable::Null => Err(CollectorError::EventCreationError{message: "Cannot map jmespath::Variable::Null to the Event payload".to_owned()}),
        // ToDo how to map Expref?
        jmespath::Variable::Expref(_) => Err(CollectorError::EventCreationError{message: "Cannot map jmespath::Variable::Expref to the Event payload".to_owned()}),
    }
}


#[cfg(test)]
mod test {

    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::rc::Rc;
    use std::sync::Mutex;

    #[test]
    fn value_processor_text_should_return_static_text() {
        // Arrange
        let value_proc = ValueProcessor::Text { text: "hello world".to_owned() };
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_ok());
        assert_eq!("hello world", *atomic.lock().unwrap());
    }

    #[test]
    fn value_processor_expression_should_return_from_json() {
        // Arrange
        let exp = jmespath::compile("level_one.level_two").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_ok());
        assert_eq!("level_two_value", *atomic.lock().unwrap());
    }

    #[test]
    fn value_processor_expression_should_return_error_if_not_present() {
        // Arrange
        let exp = jmespath::compile("level_one.level_three").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "level_one": {
                "level_two": "level_two_value"
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_err());
        assert_eq!("", *atomic.lock().unwrap());
    }

    #[test]
    fn value_processor_expression_should_return_error_if_not_present_in_array() {
        // Arrange
        let exp = jmespath::compile("level_one.level_two[2]").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "level_one": {
                "level_two": ["level_two_0", "level_two_1"]
            }
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_err());
        assert_eq!("", *atomic.lock().unwrap());
    }

    /*
    #[test]
    fn value_processor_expression_should_handle_non_string_values() {
        // Arrange
        let exp = jmespath::compile("key").unwrap();
        let value_proc = ValueProcessor::Expression { exp };
        let json = r#"
        {
            "key": true
        }
        "#;
        let data = jmespath::Variable::from_json(&json).unwrap();
        let atomic = Rc::new(Mutex::new("".to_owned()));
        let atomic_clone = atomic.clone();

        // Act
        let result = value_proc.process(&data, move |value| {
            let mut lock = atomic_clone.lock().unwrap();
            *lock = value.to_owned();
            Ok(())
        });

        // Assert
        assert!(result.is_ok());
        assert_eq!("true", *atomic.lock().unwrap());
    }
    */

    #[test]
    fn event_processor_should_build_from_config_with_static_type() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "hello world".to_owned(),
            payload: HashMap::new(),
        };
        config.payload.insert("one".to_owned(), "value_one".to_owned());
        config.payload.insert("two".to_owned(), "value_two".to_owned());

        // Act
        let event_processor = EventProcessor::build(&config).unwrap();

        // Assert
        assert_eq!(
            ValueProcessor::Text { text: "hello world".to_owned() },
            event_processor.event_type
        );
        assert_eq!(
            &ValueProcessor::Text { text: "value_one".to_owned() },
            event_processor.payload.get("one").unwrap()
        );
        assert_eq!(
            &ValueProcessor::Text { text: "value_two".to_owned() },
            event_processor.payload.get("two").unwrap()
        );
    }

    #[test]
    fn event_processor_should_build_from_config_with_expression() {
        // Arrange
        let mut config = config::JMESPathEventCollectorConfig {
            event_type: "${first.second[0]}".to_owned(),
            payload: HashMap::new(),
        };
        config.payload.insert("one".to_owned(), "${first.third}".to_owned());
        let expected_event_expression = jmespath::compile("first.second[0]").unwrap();
        let expected_payload_expression = jmespath::compile("first.third").unwrap();

        // Act
        let event_processor = EventProcessor::build(&config).unwrap();

        // Assert
        assert_eq!(
            ValueProcessor::Expression { exp: expected_event_expression },
            event_processor.event_type
        );
        assert_eq!(
            &ValueProcessor::Expression { exp: expected_payload_expression },
            event_processor.payload.get("one").unwrap()
        );
    }

    #[test]
    fn verify_expected_io() {
        verify_io(
            "./test_resources/01_config.json",
            "./test_resources/01_input.json",
            "./test_resources/01_output.json",
        );
        verify_io(
            "./test_resources/02_config.json",
            "./test_resources/02_input.json",
            "./test_resources/02_output.json",
        );
        verify_io(
            "./test_resources/github_webhook_01_config.json",
            "./test_resources/github_webhook_01_input.json",
            "./test_resources/github_webhook_01_output.json",
        );
    }

    fn verify_io(config_path: &str, input_path: &str, output_path: &str) {
        // Arrange
        let config_json = fs::read_to_string(config_path)
            .expect(&format!("Unable to open the file [{}]", config_path));
        let config: config::JMESPathEventCollectorConfig = serde_json::from_str(&config_json)
            .map_err(|e| panic!("Cannot parse config json. Err: {}", e))
            .unwrap();

        let collector = JMESPathEventCollector::build(&config).unwrap();

        let input_json = fs::read_to_string(input_path)
            .expect(&format!("Unable to open the file [{}]", input_path));

        let output_json = fs::read_to_string(output_path)
            .expect(&format!("Unable to open the file [{}]", output_path));
        let mut expected_event: Event = serde_json::from_str(&output_json)
            .map_err(|e| panic!("Cannot parse output json. Err: {}", e))
            .unwrap();;

        // Act
        let result = collector.to_event(&input_json);

        // Assert
        assert!(result.is_ok());

        let result_event = result.unwrap();
        expected_event.created_ts = result_event.created_ts.clone();

        assert_eq!(expected_event, result_event);
    }

}
