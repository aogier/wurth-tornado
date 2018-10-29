use std::collections::HashMap;
use tornado_common_api::{Action, Event};

/// The ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent<'o> {
    pub event: Event,
    pub rules: HashMap<&'o str, ProcessedRule>,
    pub extracted_vars: HashMap<&'o str, String>,
}

impl<'o> ProcessedEvent<'o> {
    pub fn new(event: Event) -> ProcessedEvent<'o> {
        ProcessedEvent { event, rules: HashMap::new(), extracted_vars: HashMap::new() }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedRule {
    pub status: ProcessedRuleStatus,
    pub actions: Vec<Action>,
    pub message: Option<String>,
}

impl Default for ProcessedRule {
    fn default() -> Self {
            ProcessedRule { status: ProcessedRuleStatus::NotProcessed, actions: vec![], message: None }
    }
}

impl ProcessedRule {
    pub fn new() -> ProcessedRule {
        Default::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedRuleStatus {
    Matched,
    PartiallyMatched,
    NotMatched,
    NotProcessed,
}
