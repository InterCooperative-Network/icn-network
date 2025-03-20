use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Dependency container for managing component dependencies
pub struct DependencyContainer {
    providers: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl DependencyContainer {
    /// Create a new empty dependency container
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
    
    /// Register a component implementation under its interface type
    pub fn register<I: ?Sized + 'static, T: 'static + Send + Sync + AsRef<I>>(&mut self, instance: Arc<T>) {
        let type_id = TypeId::of::<I>();
        let boxed = Box::new(instance);
        self.providers.insert(type_id, boxed);
    }
    
    /// Register a component implementation directly with its concrete type
    pub fn register_concrete<T: 'static + Send + Sync>(&mut self, instance: Arc<T>) {
        let type_id = TypeId::of::<T>();
        let boxed = Box::new(instance);
        self.providers.insert(type_id, boxed);
    }
    
    /// Resolve a component by its interface type
    pub fn resolve<T: 'static + ?Sized>(&self) -> Option<Arc<T>> {
        self.providers
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<Arc<T>>())
            .cloned()
    }
    
    /// Check if a component is registered for a given interface type
    pub fn contains<T: 'static + ?Sized>(&self) -> bool {
        self.providers.contains_key(&TypeId::of::<T>())
    }
} 