use liem_sdk::Plugin;

struct TemplatePlugin;

impl Plugin for TemplatePlugin {
    fn on_enable(&self) -> Result<(), String> {
        println!("Template Plugin Enabled!");
        // Initialize your resources or registers here
        Ok(())
    }

    fn on_disable(&self) -> Result<(), String> {
        println!("Template Plugin Disabled!");
        // Release your resources here
        Ok(())
    }
}

/// Constructor entry point exported by the dynamic library.
/// The host SDK loader will call this to instantiate your plugin.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn liem_create_plugin() -> *mut dyn Plugin {
    let plugin = TemplatePlugin;
    let boxed = Box::new(plugin);
    Box::into_raw(boxed)
}
