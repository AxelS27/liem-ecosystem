use crate::manifest::Plugin;
use std::panic::{catch_unwind, AssertUnwindSafe};
use libloading::{Library, Symbol};

/// Constructor symbol signature exported by plugin DLLs
#[allow(improper_ctypes_definitions)]
pub type CreatePluginFn = unsafe extern "C" fn() -> *mut dyn Plugin;

/// Represents a loaded dynamic plugin.
/// Keeps the Library handle in memory so that the dynamically resolved symbols remain valid.
pub struct DynamicPlugin {
    _library: Library,
    plugin: Box<dyn Plugin>,
}

impl Plugin for DynamicPlugin {
    fn on_enable(&self) -> Result<(), String> {
        self.plugin.on_enable()
    }
    
    fn on_disable(&self) -> Result<(), String> {
        self.plugin.on_disable()
    }
}

/// Load a plugin dynamically from a DLL on disk.
/// This method resolves the standard entry point constructor `liem_create_plugin`.
///
/// # Safety
/// The dynamically loaded library must conform to the Plugin trait ABI layout.
pub unsafe fn load_dynamic_plugin<P: AsRef<std::ffi::OsStr>>(path: P) -> Result<Box<dyn Plugin>, String> {
    let lib = Library::new(path).map_err(|e| format!("Failed to load dynamic library: {}", e))?;
    
    let constructor: Symbol<CreatePluginFn> = lib
        .get(b"liem_create_plugin")
        .map_err(|e| format!("Constructor symbol 'liem_create_plugin' not found: {}", e))?;
        
    let raw_ptr = constructor();
    if raw_ptr.is_null() {
        return Err("Plugin constructor returned a null pointer".to_string());
    }
    
    let plugin = Box::from_raw(raw_ptr);
    
    Ok(Box::new(DynamicPlugin {
        _library: lib,
        plugin,
    }))
}

/// Unwind-safe wrapper wrapping a Plugin.
/// Intercepts panics inside plugin calls, isolating and containing runtime failures.
pub struct UnwindSafePlugin {
    inner: Box<dyn Plugin>,
}

impl UnwindSafePlugin {
    /// Wrap any Plugin inside the unwind safety wrapper
    pub fn new(inner: Box<dyn Plugin>) -> Self {
        Self { inner }
    }
}

impl Plugin for UnwindSafePlugin {
    fn on_enable(&self) -> Result<(), String> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.inner.on_enable()
        }));
        
        match result {
            Ok(res) => res,
            Err(e) => {
                let err_msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                Err(format!("Plugin panicked during on_enable: {}", err_msg))
            }
        }
    }

    fn on_disable(&self) -> Result<(), String> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.inner.on_disable()
        }));
        
        match result {
            Ok(res) => res,
            Err(e) => {
                let err_msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                Err(format!("Plugin panicked during on_disable: {}", err_msg))
            }
        }
    }
}
