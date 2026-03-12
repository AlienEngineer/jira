pub mod interface;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::ioc::interface::Interface;

type Service = Box<dyn Any + Send + Sync>;
type Factory = Arc<dyn Fn(&Ioc) -> Service + Send + Sync>;

#[macro_export]
macro_rules! register_service {
    ($ioc:expr, $interface:path, $implementation:path) => {
        $ioc.register::<dyn $interface, _>(|_| -> std::sync::Arc<dyn $interface> {
            std::sync::Arc::new($implementation)
        })
    };
    ($ioc:expr, $interface:path, $implementation:block) => {
        $ioc.register::<dyn $interface, _>(|container| -> std::sync::Arc<dyn $interface> {
            macro_rules! get_instance {
                ($dependency:path) => {
                    container
                        .get::<dyn $dependency>()
                        .expect(concat!("service not registered: ", stringify!($dependency)))
                };
            }

            std::sync::Arc::new($implementation)
        })
    };
    ($ioc:expr, $interface:path, |_| $implementation:expr) => {
        $ioc.register::<dyn $interface, _>(|_| -> std::sync::Arc<dyn $interface> {
            $implementation
        })
    };
    ($ioc:expr, $interface:path, |$container:ident| $implementation:expr) => {
        $ioc.register::<dyn $interface, _>(|$container| -> std::sync::Arc<dyn $interface> {
            macro_rules! get_instance {
                ($dependency:path) => {
                    $container
                        .get::<dyn $dependency>()
                        .expect(concat!("service not registered: ", stringify!($dependency)))
                };
            }

            $implementation
        })
    };
}

#[macro_export]
macro_rules! get_instance {
    ($ioc:expr, $interface:path) => {
        $ioc.get::<dyn $interface>()
            .expect(concat!("service not registered: ", stringify!($interface)))
    };
}

struct Ioc {
    services: RwLock<HashMap<TypeId, Service>>,
    factories: HashMap<TypeId, Factory>,
}

impl Ioc {
    fn new() -> Self {
        Self {
            services: RwLock::new(HashMap::new()),
            factories: HashMap::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.factories.is_empty() && self.services.read().unwrap().is_empty()
    }

    fn register<I, F>(&mut self, factory: F)
    where
        I: ?Sized + Interface + 'static,
        F: Fn(&Ioc) -> Arc<I> + Send + Sync + 'static,
    {
        self.factories.insert(
            TypeId::of::<I>(),
            Arc::new(move |container| Box::new(factory(container))),
        );
    }

    fn get<I>(&self) -> Option<Arc<I>>
    where
        I: ?Sized + Interface + 'static,
    {
        let type_id = TypeId::of::<I>();

        if let Some(service) = self
            .services
            .read()
            .unwrap()
            .get(&type_id)
            .and_then(|service| service.downcast_ref::<Arc<I>>())
            .cloned()
        {
            return Some(service);
        }

        let factory = self.factories.get(&type_id)?.clone();
        let service = factory(self);

        let mut services = self.services.write().unwrap();
        let service = services.entry(type_id).or_insert(service);

        service.downcast_ref::<Arc<I>>().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait MyServiceInterface: Interface {
        fn hello(&self) -> &'static str;
    }

    struct MyRealService;

    impl MyServiceInterface for MyRealService {
        fn hello(&self) -> &'static str {
            "hello"
        }
    }

    trait MyServiceWithDependencyInterface: Interface {
        fn hello(&self) -> &'static str;
    }

    struct MyRealServiceWithDependency {
        dependency: Arc<dyn MyServiceInterface>,
    }

    impl MyServiceWithDependencyInterface for MyRealServiceWithDependency {
        fn hello(&self) -> &'static str {
            self.dependency.hello()
        }
    }

    // ── basic resolution ─────────────────────────────────────────────────────

    #[test]
    fn a_new_ioc_doesnt_contain_any_service() {
        let ioc = Ioc::new();

        assert!(ioc.is_empty());
    }

    #[test]
    fn registering_a_service_factory_resolves_service() {
        let mut ioc = Ioc::new();

        register_service!(ioc, MyServiceInterface, MyRealService);

        assert!(
            !ioc.is_empty(),
            "registered factories should make the container non-empty"
        );

        let service = get_instance!(ioc, MyServiceInterface);

        assert_eq!(service.hello(), "hello");
    }

    #[test]
    fn resolving_a_service_multiple_times_reuses_the_cached_instance() {
        let mut ioc = Ioc::new();

        register_service!(ioc, MyServiceInterface, MyRealService);

        let first = get_instance!(ioc, MyServiceInterface);
        let second = get_instance!(ioc, MyServiceInterface);

        assert!(
            Arc::ptr_eq(&first, &second),
            "lazy resolution should cache the constructed service instance"
        );
    }

    // ── dependency resolution ────────────────────────────────────────────────

    #[test]
    fn resolving_a_service_with_dependencies_works_even_when_registered_first() {
        let mut ioc = Ioc::new();

        register_service!(ioc, MyServiceWithDependencyInterface, {
            MyRealServiceWithDependency {
                dependency: get_instance!(MyServiceInterface),
            }
        });
        register_service!(ioc, MyServiceInterface, MyRealService);

        let service = get_instance!(ioc, MyServiceWithDependencyInterface);

        assert_eq!(service.hello(), "hello");
    }
}
