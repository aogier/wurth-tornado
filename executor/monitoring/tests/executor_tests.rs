use httpmock::Method::POST;
use httpmock::{MockServer, Regex};
use maplit::*;
use std::collections::HashMap;
use tornado_common_api::{Action, Value};
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tornado_executor_director::config::DirectorClientConfig;
use tornado_executor_icinga2::config::Icinga2ClientConfig;
use tornado_executor_monitoring::MonitoringExecutor;

#[tokio::test]
async fn should_return_error_if_process_check_result_fails_with_error_different_than_non_existing_object(
) {
    // Arrange
    let server = MockServer::start();

    let icinga_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/actions/process-check-result");
        then.status(500);
    });

    let executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        },
    )
    .unwrap();

    let mut action = Action::new("", "");
    action.payload.insert(
        "action_name".to_owned(),
        Value::String("create_and_or_process_host_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Object(hashmap!(
            "host".to_owned() => Value::String("myhost".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Object(HashMap::new()));

    // Act
    let result = executor.execute(action.into()).await;

    // Assert
    assert!(result.is_err());
    assert_eq!(icinga_mock.hits(), 1);

    match result {
        Err(ExecutorError::ActionExecutionError { message, .. }) => {
            assert!(
                message.contains(&format!("{}/v1/actions/process-check-result", server.url("")))
            );
        }
        _ => assert!(false),
    };
}

#[tokio::test]
async fn should_return_ok_if_process_check_result_is_successful() {
    // Arrange
    let icinga_server = MockServer::start();

    let icinga_mock = icinga_server.mock(|when, then| {
        when.method(POST).path("/v1/actions/process-check-result");
        then.status(201);
    });

    let executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: "".to_owned(),
        },
    )
    .unwrap();

    let mut action = Action::new("", "");
    action.payload.insert(
        "action_name".to_owned(),
        Value::String("create_and_or_process_host_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Object(hashmap!(
            "host".to_owned() => Value::String("myhost".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Object(HashMap::new()));

    // Act
    let result = executor.execute(action.into()).await;

    // Assert
    assert!(result.is_ok());
    assert_eq!(icinga_mock.hits(), 1);
}

#[tokio::test]
async fn should_return_call_process_check_result_twice_on_non_existing_object() {
    // Arrange
    let icinga_server = MockServer::start();
    let icinga_server_response = "{\"error\":404.0,\"status\":\"No objects found.\"}";

    let icinga_mock = icinga_server.mock(|when, then| {
        when.method(POST).path("/v1/actions/process-check-result");
        then.body(icinga_server_response).status(404);
    });

    let director_server = MockServer::start();

    let director_mock = director_server.mock(|when, then| {
        when.method(POST).path_matches(Regex::new("/(host)|(service)").unwrap());
        then.status(201);
    });

    let executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: director_server.url(""),
        },
    )
    .unwrap();

    let mut action = Action::new("", "");
    action.payload.insert(
        "action_name".to_owned(),
        Value::String("create_and_or_process_service_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Object(hashmap!(
            "service".to_owned() => Value::String("myhost:myservice".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Object(HashMap::new()));
    action.payload.insert("service_creation_payload".to_owned(), Value::Object(HashMap::new()));

    // Act
    let result = executor.execute(action.clone().into()).await;

    // Assert
    assert!(result.is_err());
    // one time when object is not existing, one time after the creation of the object
    assert_eq!(icinga_mock.hits(), 2);
    // director server should be called once to create the host, and once to create the service
    assert_eq!(director_mock.hits(), 2);

    match result {
        Err(ExecutorError::ActionExecutionError { message, .. }) => {
            assert!(message
                .contains(&format!("{}/v1/actions/process-check-result", icinga_server.url(""))));
        }
        _ => assert!(false),
    };
}

#[tokio::test]
async fn should_return_return_error_on_object_creation_failure() {
    // Arrange
    let icinga_server = MockServer::start();
    let icinga_server_response = "{\"error\":404.0,\"status\":\"No objects found.\"}";

    let icinga_mock = icinga_server.mock(|when, then| {
        when.method(POST).path("/v1/actions/process-check-result");
        then.body(icinga_server_response).status(404);
    });

    let director_server = MockServer::start();
    let director_server_response = "{\"error\":500.0,\"status\":\"Internal Server Error.\"}";

    let director_mock = director_server.mock(|when, then| {
        when.method(POST).path_matches(Regex::new("/(host)|(service)").unwrap());
        then.body(director_server_response).status(500);
    });

    let executor = MonitoringExecutor::new(
        Icinga2ClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: icinga_server.url(""),
        },
        DirectorClientConfig {
            timeout_secs: None,
            username: "".to_owned(),
            password: "".to_owned(),
            disable_ssl_verification: true,
            server_api_url: director_server.url(""),
        },
    )
    .unwrap();

    let mut action = Action::new("", "");
    action.payload.insert(
        "action_name".to_owned(),
        Value::String("create_and_or_process_service_passive_check_result".to_owned()),
    );
    action.payload.insert(
        "process_check_result_payload".to_owned(),
        Value::Object(hashmap!(
            "service".to_owned() => Value::String("myhost:myservice".to_owned()),
        )),
    );
    action.payload.insert("host_creation_payload".to_owned(), Value::Object(HashMap::new()));
    action.payload.insert("service_creation_payload".to_owned(), Value::Object(HashMap::new()));

    // Act
    let result = executor.execute(action.into()).await;

    // Assert
    assert!(result.is_err());
    // one time when object is not existing, one time after the creation of the object
    assert_eq!(icinga_mock.hits(), 1);
    // director server should be called once to create the host, and once to create the service
    assert_eq!(director_mock.hits(), 1);

    match result {
        Err(ExecutorError::ActionExecutionError { message, .. }) => {
            assert!(message.contains("Response status: 500 Internal Server Error"));
            assert!(
                message.contains(&format!("{}/host?live-creation=true", director_server.url("")))
            );
        }
        _ => assert!(false),
    };
}
