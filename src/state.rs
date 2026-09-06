//! Shared application state handed to every route handler.

use std::sync::Arc;

use minijinja_autoreload::AutoReloader;

use crate::store::Store;
use crate::templates;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub struct Inner {
    pub store: Store,
    pub templates: AutoReloader,
}

impl AppState {
    pub fn new(store: Store) -> Self {
        AppState(Arc::new(Inner {
            store,
            templates: templates::build_reloader(),
        }))
    }

    pub fn store(&self) -> &Store {
        &self.0.store
    }

    pub fn templates(&self) -> &AutoReloader {
        &self.0.templates
    }
}
