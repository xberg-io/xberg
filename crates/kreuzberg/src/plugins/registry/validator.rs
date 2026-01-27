//! Validator registry implementation.

use crate::Result;
use crate::plugins::Validator;
use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Registry for validator plugins.
///
/// Manages validators with priority-based execution order.
pub struct ValidatorRegistry {
    validators: BTreeMap<i32, IndexMap<String, Arc<dyn Validator>>>,
}

impl ValidatorRegistry {
    /// Create a new empty validator registry.
    pub fn new() -> Self {
        Self {
            validators: BTreeMap::new(),
        }
    }

    /// Register a validator.
    ///
    /// # Arguments
    ///
    /// * `validator` - The validator to register
    pub fn register(&mut self, validator: Arc<dyn Validator>) -> Result<()> {
        let name = validator.name().to_string();
        let priority = validator.priority();

        if let Err(e) = super::validate_plugin_name(&name) {
            tracing::warn!(
                "Failed to validate validator name '{}': {}. \
                 Registration aborted. Plugin names must be non-empty and contain only alphanumeric characters, hyphens, and underscores.",
                name,
                e
            );
            return Err(e);
        }

        if let Err(e) = validator.initialize() {
            tracing::error!(
                "Failed to initialize validator '{}' with priority {}: {}. \
                 Validation step will not be executed.",
                name,
                priority,
                e
            );
            return Err(e);
        }

        self.validators
            .entry(priority)
            .or_default()
            .insert(name.clone(), validator);
        tracing::debug!("Registered validator '{}' with priority {}", name, priority);

        Ok(())
    }

    /// Get all validators in priority order.
    ///
    /// # Returns
    ///
    /// Vector of validators in priority order (highest first).
    pub fn get_all(&self) -> Vec<Arc<dyn Validator>> {
        let mut result = Vec::new();

        for (_priority, validators) in self.validators.iter().rev() {
            for validator in validators.values() {
                result.push(Arc::clone(validator));
            }
        }

        result
    }

    /// List all registered validator names.
    pub fn list(&self) -> Vec<String> {
        let mut names = std::collections::HashSet::new();
        for validators in self.validators.values() {
            names.extend(validators.keys().cloned());
        }
        names.into_iter().collect()
    }

    /// Remove a validator from the registry.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        let mut validator_to_shutdown: Option<Arc<dyn Validator>> = None;
        let mut found = false;

        for validators in self.validators.values_mut() {
            if let Some(validator) = validators.shift_remove(name)
                && validator_to_shutdown.is_none()
            {
                validator_to_shutdown = Some(validator);
                found = true;
            }
        }

        if !found {
            tracing::debug!(
                "Validator '{}' not found in registry (already removed or never registered)",
                name
            );
        }

        if let Some(validator) = validator_to_shutdown {
            if let Err(e) = validator.shutdown() {
                tracing::warn!(
                    "Failed to shutdown validator '{}': {}. \
                     Resources may not have been properly released.",
                    name,
                    e
                );
                return Err(e);
            }
            tracing::debug!("Successfully removed and shut down validator '{}'", name);
        }

        self.validators.retain(|_, validators| !validators.is_empty());

        Ok(())
    }

    /// Shutdown all validators and clear the registry.
    pub fn shutdown_all(&mut self) -> Result<()> {
        let names = self.list();
        let count = names.len();

        if count > 0 {
            tracing::debug!("Shutting down {} validators", count);
        }

        for name in names {
            self.remove(&name)?;
        }

        if count > 0 {
            tracing::debug!("Successfully shut down all {} validators", count);
        }
        Ok(())
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KreuzbergError;
    use crate::core::config::ExtractionConfig;
    use crate::plugins::Plugin;
    use crate::types::ExtractionResult;
    use async_trait::async_trait;

    struct MockValidator {
        name: String,
        priority: i32,
    }

    impl Plugin for MockValidator {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            Ok(())
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for MockValidator {
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[test]
    fn test_validator_registry() {
        let mut registry = ValidatorRegistry::new();

        let high_priority = Arc::new(MockValidator {
            name: "high-priority".to_string(),
            priority: 100,
        });

        let low_priority = Arc::new(MockValidator {
            name: "low-priority".to_string(),
            priority: 10,
        });

        registry.register(high_priority).unwrap();
        registry.register(low_priority).unwrap();

        let validators = registry.get_all();
        assert_eq!(validators.len(), 2);
        assert_eq!(validators[0].name(), "high-priority");
        assert_eq!(validators[1].name(), "low-priority");
    }

    #[test]
    fn test_validator_registry_remove() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "test-validator".to_string(),
            priority: 50,
        });

        registry.register(validator).unwrap();
        assert_eq!(registry.get_all().len(), 1);

        registry.remove("test-validator").unwrap();
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_validator_registry_default() {
        let registry = ValidatorRegistry::default();
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_validator_registry_invalid_name_empty() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "".to_string(),
            priority: 50,
        });

        let result = registry.register(validator);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_validator_registry_invalid_name_whitespace() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "my validator".to_string(),
            priority: 50,
        });

        let result = registry.register(validator);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_validator_registry_shutdown_all() {
        let mut registry = ValidatorRegistry::new();

        let validator1 = Arc::new(MockValidator {
            name: "validator1".to_string(),
            priority: 100,
        });

        let validator2 = Arc::new(MockValidator {
            name: "validator2".to_string(),
            priority: 50,
        });

        registry.register(validator1).unwrap();
        registry.register(validator2).unwrap();

        assert_eq!(registry.get_all().len(), 2);

        registry.shutdown_all().unwrap();
        assert_eq!(registry.get_all().len(), 0);
    }

    struct FailingValidator {
        name: String,
        priority: i32,
        fail_on_init: bool,
    }

    impl Plugin for FailingValidator {
        fn name(&self) -> &str {
            &self.name
        }
        fn version(&self) -> String {
            "1.0.0".to_string()
        }
        fn initialize(&self) -> Result<()> {
            if self.fail_on_init {
                Err(KreuzbergError::Plugin {
                    message: "Validator initialization failed".to_string(),
                    plugin_name: self.name.clone(),
                })
            } else {
                Ok(())
            }
        }
        fn shutdown(&self) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl Validator for FailingValidator {
        async fn validate(&self, _: &ExtractionResult, _: &ExtractionConfig) -> Result<()> {
            Ok(())
        }

        fn priority(&self) -> i32 {
            self.priority
        }
    }

    #[test]
    fn test_validator_initialization_failure_logs_error() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(FailingValidator {
            name: "failing-validator".to_string(),
            priority: 50,
            fail_on_init: true,
        });

        let result = registry.register(validator);
        assert!(result.is_err());
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_validator_invalid_name_empty_logs_warning() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "".to_string(),
            priority: 50,
        });

        let result = registry.register(validator);
        assert!(matches!(result, Err(KreuzbergError::Validation { .. })));
    }

    #[test]
    fn test_validator_successful_registration_logs_debug() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "valid-validator".to_string(),
            priority: 50,
        });

        let result = registry.register(validator);
        assert!(result.is_ok());
        assert_eq!(registry.get_all().len(), 1);
    }

    #[test]
    fn test_validator_remove_nonexistent_logs_debug() {
        let mut registry = ValidatorRegistry::new();

        let result = registry.remove("nonexistent-validator");
        assert!(result.is_ok());
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_validator_priority_ordering_reversed() {
        let mut registry = ValidatorRegistry::new();

        let high = Arc::new(MockValidator {
            name: "high-priority".to_string(),
            priority: 100,
        });

        let low = Arc::new(MockValidator {
            name: "low-priority".to_string(),
            priority: 10,
        });

        registry.register(low).unwrap();
        registry.register(high).unwrap();

        let validators = registry.get_all();
        assert_eq!(validators.len(), 2);
        assert_eq!(validators[0].name(), "high-priority");
        assert_eq!(validators[1].name(), "low-priority");
    }

    #[test]
    fn test_validator_multiple_same_priority() {
        let mut registry = ValidatorRegistry::new();

        let validator1 = Arc::new(MockValidator {
            name: "validator-a".to_string(),
            priority: 50,
        });

        let validator2 = Arc::new(MockValidator {
            name: "validator-b".to_string(),
            priority: 50,
        });

        let validator3 = Arc::new(MockValidator {
            name: "validator-c".to_string(),
            priority: 50,
        });

        registry.register(validator1).unwrap();
        registry.register(validator2).unwrap();
        registry.register(validator3).unwrap();

        let validators = registry.get_all();
        assert_eq!(validators.len(), 3);
    }

    #[test]
    fn test_validator_shutdown_empty_registry() {
        let mut registry = ValidatorRegistry::new();
        let result = registry.shutdown_all();
        assert!(result.is_ok());
        assert_eq!(registry.get_all().len(), 0);
    }

    #[test]
    fn test_validator_remove_and_readd() {
        let mut registry = ValidatorRegistry::new();

        let validator = Arc::new(MockValidator {
            name: "test-validator".to_string(),
            priority: 50,
        });

        registry.register(validator.clone()).unwrap();
        assert_eq!(registry.get_all().len(), 1);

        registry.remove("test-validator").unwrap();
        assert_eq!(registry.get_all().len(), 0);

        registry.register(validator).unwrap();
        assert_eq!(registry.get_all().len(), 1);
    }
}
