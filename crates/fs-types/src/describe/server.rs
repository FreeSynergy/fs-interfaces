//! `CatalogServer`: answers `TypeCatalog.describe` from a registry.
//!
//! Pattern: Adapter — the generated `type_catalog::Server` trait over a [`TypeRegistry`]. A program adds it to its
//! own RPC bootstrap (or serves it on its own capability); it holds no logic of its own.

use std::rc::Rc;

use fs_interfaces::types_capnp::type_catalog;

use super::TypeRegistry;

/// Serves the descriptions of a registry.
pub struct CatalogServer(Rc<TypeRegistry>);

impl CatalogServer {
    /// Serve `registry`.
    #[must_use]
    pub fn new(registry: TypeRegistry) -> Self {
        Self(Rc::new(registry))
    }
}

impl type_catalog::Server for CatalogServer {
    fn describe(
        self: Rc<Self>,
        _: type_catalog::DescribeParams,
        mut results: type_catalog::DescribeResults,
    ) -> impl std::future::Future<Output = Result<(), capnp::Error>> + 'static {
        // Nothing to wait for: the answer is written before the future exists.
        let n = u32::try_from(self.0.len()).unwrap_or(u32::MAX);
        self.0.write(results.get().init_types(n));
        std::future::ready(Ok(()))
    }
}
