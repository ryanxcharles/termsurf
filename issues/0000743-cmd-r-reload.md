# Issue 743: Cmd+R intercepted by Wezboard

## Goal

Pressing Cmd+R in a browser pane should reload the web page, not reload the
Wezboard configuration.

## Background

WezTerm binds Cmd+R to `ReloadConfiguration` — reloading the terminal's config
file. Wezboard inherits this binding. When a user presses Cmd+R while viewing a
web page, Wezboard intercepts the key and reloads its config instead of
forwarding it to the browser.

Cmd+[ and Cmd+] (back/forward) work correctly because they are _not_ in
Wezboard's default key bindings. Unbound keys propagate through
`try_forward_key()` in `termsurf/input.rs` and reach Chromium. Cmd+R never
reaches `try_forward_key()` because `process_key()` matches it to
`ReloadConfiguration` first.

The relevant code path:

1. Key event enters `process_key()` in `keyevent.rs`
2. `try_forward_key()` is called — but for raw key events,
   `only_key_bindings=true` causes it to return `None` immediately
3. Key binding lookup finds `ReloadConfiguration` for Cmd+R
4. Config is reloaded; browser never sees the key

### Why removing the binding is safe

Wezboard already monitors its config file for changes and reloads automatically.
The `ReloadConfiguration` menu item and Cmd+R shortcut are redundant. Removing
Cmd+R from the binding lets it propagate to the browser like Cmd+[/] do. Users
who want manual config reload can still trigger it from the menu (without a
keyboard shortcut) or by saving their config file.

### Where Cmd+R is defined

`wezboard/wezboard-gui/src/commands.rs` (lines 1264–1271):

```rust
ReloadConfiguration => CommandDef {
    brief: "Reload configuration".into(),
    doc: "Reloads the configuration file".into(),
    keys: vec![(Modifiers::SUPER, "r".into())],
    args: &[],
    menubar: &["TermSurf Wezboard"],
    icon: Some("md_reload"),
},
```

The `keys` field binds Cmd+R. The `menubar` field places it in the application
menu. These are independent — removing the key binding doesn't remove the menu
item.
