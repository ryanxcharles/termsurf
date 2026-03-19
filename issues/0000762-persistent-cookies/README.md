+++
status = "open"
opened = "2026-03-19"
+++

# Issue 762: Cookies and network state not persisted across restarts

## Goal

Cookies, HTTP cache, and other network state should persist to disk so that
users stay logged in across Roamium restarts.

## Background

### The problem

When Roamium exits and restarts, all cookies are gone. Users are logged out of
every site. localStorage may also be lost. The profile directory at
`~/.local/share/termsurf/chromium-profiles/{profile}/` exists and contains some
data (GPU cache, etc.), but cookies and network state are not among it.

### Root cause

Content shell's `ConfigureNetworkContextParamsForShell()` in
`shell_content_browser_client.cc` does not set `file_paths` on the
`NetworkContextParams`. Without `file_paths`, the network service uses
**in-memory storage** for everything — cookies, HTTP cache, HTTP server
properties, and all other network state. The data lives only in memory and is
lost when the process exits.

Chrome solves this in
`ProfileNetworkContextService::ConfigureNetworkContextParams()` by setting:

```cpp
network_context_params->file_paths =
    ::network::mojom::NetworkContextFilePaths::New();
network_context_params->file_paths->data_directory = path.Append("Network");
network_context_params->file_paths->unsandboxed_data_path = path;
network_context_params->file_paths->cookie_database_name =
    base::FilePath("Cookies");
network_context_params->file_paths->http_cache_directory =
    cache_path.Append("Cache");
network_context_params->file_paths->http_server_properties_file_name =
    base::FilePath("Network Persistent State");
```

Content shell never does this because it's a test harness, not a real browser.

### History

This was never solved in the Content API generation. Earlier generations used
CEF, which handles cookie persistence internally when given a `cache_path`. When
TermSurf switched from CEF to the Content API (ts5/ts6), nobody wired up the
network service file paths. The `--user-data-dir` flag correctly sets
`ShellBrowserContext::GetPath()`, but that path is only used by some subsystems
(GPU cache, etc.) — not the network service.

### Fix

Override `ConfigureNetworkContextParamsForShell()` in our content browser client
(or patch content shell's implementation) to set `file_paths` using the browser
context's path. The minimum viable change:

```cpp
auto* context_impl = static_cast<ShellBrowserContext*>(context);
base::FilePath path = context_impl->GetPath();

context_params->file_paths =
    network::mojom::NetworkContextFilePaths::New();
context_params->file_paths->data_directory = path.Append("Network");
context_params->file_paths->unsandboxed_data_path = path;
context_params->file_paths->cookie_database_name =
    base::FilePath("Cookies");
context_params->file_paths->http_cache_directory =
    path.Append("Cache");
context_params->file_paths->http_server_properties_file_name =
    base::FilePath("Network Persistent State");
```

### Scope

Chromium-only change. One function in `shell_content_browser_client.cc` (or a
TermSurf override). No protocol, Roamium, or TUI changes needed.
