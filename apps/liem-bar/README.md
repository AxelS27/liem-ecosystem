# Liem Bar

Liem Bar is a modern, modular, and capability-aware desktop bar designed specifically for Windows.

---

## Features
- **Tree-Based Layout Engine**: Renders dynamic layouts using Rows, Columns, Spacers, and Groups.
- **Dynamic Configuration & Hot-Reload**: Watches configuration modifications and updates on-screen styling/widgets instantly.
- **Ecosystem theme sync**: Integrates with Liem Wallpaper over Named Pipes to automatically sync accent colors.
- **Windows Taskbar Management**: Auto-hides native taskbars on startup and restores them on exit.
- **Community Extensions**: Supports dynamic plugin DLL loading under the `community` feature gate with SemVer checks.
- **Throttling & Power Budgeting**: Skips ticks for resource-intensive widgets when battery saver is active.

---

## CLI Tools
Route subcommands by passing arguments to the binary:

```bash
# Validate config profile layouts
liem-bar validate

# Run system monitor diagnostics
liem-bar diagnostics

# Switch active profile interactively
liem-bar edit
```

---

## Configuration Settings
Configuration is saved in `%APPDATA%/Liem/liem-bar/config.json`:

```json
{
  "schema_version": 1,
  "active_profile": "default",
  "manage_windows_taskbar": true,
  "profiles": {
    "default": {
      "bars": [
        {
          "monitor_id": "primary",
          "position": "Top",
          "layout_name": "standard"
        }
      ]
    }
  }
}
```
