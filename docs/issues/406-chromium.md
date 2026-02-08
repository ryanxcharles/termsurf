# Issue 406-chromium: Profile Isolation — CEF Limitation vs Chromium Limitation

## Goal

Determine whether the "one profile per process" constraint that shaped ts3's
architecture is a CEF limitation or a Chromium limitation. If it is CEF-only,
using the Chromium Content API directly would eliminate the need for one process
per browser profile.

## Background

ts2 and ts3 were built around a hard constraint: CEF allows only one
`root_cache_path` per process. A second CEF initialization with a different
`root_cache_path` crashes due to `SingletonLock`. CEF's Chrome runtime
(post-M128) ignores custom `cache_path` — the `root_cache_path` IS the profile.
This forced ts3 to use one CEF process per browser profile, communicating with
the GUI via XPC Mach port transfer.

ts4 plans to drop CEF and potentially use Chromium directly (Content API). This
raises the question: was the one-profile-per-process constraint a CEF limitation
or a Chromium limitation?

## Finding: It Is a CEF Limitation

**The one-profile-per-process constraint is a CEF limitation, not a Chromium
limitation.** Chromium's content layer (`content::BrowserContext`) fully
supports multiple profiles with different storage paths in the same process.
Chrome itself does this routinely.

### How Chromium handles profiles

Chrome manages multiple profiles in a single browser process. When you switch
profiles in Chrome, it opens a new window — but within the same browser process.

The directory hierarchy:

```
User Data Directory (e.g., ~/.config/google-chrome/)
├── SingletonLock          ← ProcessSingleton lock (one per user data dir)
├── Local State            ← Global state
├── Default/               ← Profile directory (BrowserContext)
│   ├── Cookies
│   ├── History
│   └── ...
├── Profile 1/             ← Another profile (another BrowserContext)
├── Profile 2/             ← Yet another profile
└── ...
```

Key relationships:

- **`Profile`** is Chrome's class for a user profile (in the `chrome/` layer)
- **`BrowserContext`** is the content-layer abstraction that `Profile`
  implements
- **`KeyedService`** pattern creates per-profile service instances via factory
  singletons
- Multiple `Profile`/`BrowserContext` instances coexist in one process, each
  with isolated cookies, history, and storage

### Where the locks live

Chromium has a `ProcessSingleton` mechanism that locks the **user data
directory** (the parent of all profiles). This prevents two Chrome processes
from using the same user data directory. The lock is a symlink file called
`SingletonLock` placed in the user data directory, encoding `hostname-PID`.

Crucially:

- `ProcessSingleton` locks the **user data directory**, not individual profile
  directories
- Multiple profiles coexist as subdirectories under the same user data
  directory, managed by the same process
- `ProcessSingleton` lives in the `chrome/` browser layer, **not** in the
  content layer
- If you embed Chromium via the Content API directly (not via CEF, not via the
  Chrome layer), there is no built-in singleton enforcement

### The Content API and profiles

`content::BrowserContext` fully supports multiple instances with different
storage paths in the same process:

- `BrowserContext::GetPath()` is a pure virtual method returning the storage
  directory. Each implementation returns a different path.
- Each `BrowserContext` gets its own `StoragePartition` instances, managing
  cookies, localStorage, IndexedDB, etc. independently.
- There is no `SingletonLock` at the content layer.
- The `content_shell` example browser creates `ShellBrowserContext` instances
  with no mechanism preventing multiple instances with different paths.

### Electron proves it works

Electron uses the Chromium Content API directly and provides
`session.fromPartition()` to create isolated sessions:

- `session.fromPartition('persist:work')` creates a persistent session with its
  own cookies, cache, and storage in a dedicated directory
- `session.fromPartition('persist:personal')` creates another completely
  isolated session
- Sessions without the `persist:` prefix are in-memory only
- **All of these run in the same main process**

Under the hood, Electron's `ElectronBrowserContext` class extends Chromium's
`content::BrowserContext`. Each partition gets its own instance with a separate
`GetPath()` return value, creating fully isolated storage.

### What CEF does differently

CEF introduces its own constraints on top of Chromium:

| Concept           | Chromium                                                   | CEF                                                                                    |
| ----------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| User data dir     | `--user-data-dir` flag, parent of profiles                 | `CefSettings.root_cache_path`                                                          |
| Profile dir       | Subdirectory under user data dir                           | `CefSettings.cache_path` or `CefRequestContextSettings.cache_path`                     |
| Process lock      | `SingletonLock` in user data dir, lives in `chrome/` layer | CEF enforces its own singleton based on `root_cache_path` (since CEF 120)              |
| Multiple profiles | Fully supported via multiple `BrowserContext` instances    | Designed to work via `CefRequestContext`, but post-M128 Chrome runtime has regressions |

CEF's `root_cache_path` maps to Chromium's user data directory. Marshall
Greenblatt (CEF creator) confirmed: "CEF has a 'root cache path' directory
containing 'cache path' directories. These directories are equivalent (e.g.
'user data == root cache path' and 'profile' == 'cache path')."

The post-M128 Chrome runtime regression is the practical blocker for CEF: after
CEF removed the Alloy bootstrap and consolidated to Chrome bootstrap only
(M128), `cache_path` settings are being ignored and the "default" profile is
always used. This is a **bug**, not an architectural constraint.

## Implications for ts4

### If using the Chromium Content API directly

- Multiple browser profiles can coexist in a **single process**
- No need for one process per profile
- No need for a launcher to manage profile processes
- The entire XPC-based multi-process profile architecture from ts3 becomes
  unnecessary for profile isolation
- This is exactly what Electron does

### If staying with CEF

- The one-profile-per-process constraint remains (due to the M128 regression)
- The ts3 architecture (one CEF process per profile, XPC Mach port transfer) is
  still required
- This may be fixed in future CEF versions, but there is no timeline

### Multi-process may still be desirable

Even though Chromium supports multiple profiles in one process, there are
reasons to keep some process separation:

- **Crash isolation:** One misbehaving webpage should not crash the entire
  application. Chromium's own multi-process model (one renderer per tab/site)
  handles this internally, so this may already be covered.
- **Memory isolation:** Heavy webpages consuming excessive memory can be killed
  without affecting the terminal or other tabs.
- **Security:** Process boundaries are security boundaries. A compromised
  renderer cannot access another profile's data if they are in separate
  processes.

However, these are **Chromium's own renderer processes** — Chromium already
spawns separate renderer processes for each tab/site internally. The question
for ts4 is whether the **browser host process** (the one that manages
`BrowserContext` objects) needs to be per-profile. The answer is no — one host
process can manage multiple profiles, each with its own set of renderer
processes.

## Impact on Issue 406 (Chromium Framerate PoC)

This finding affects the PoC design:

- If using the Content API directly, the CEF off-screen rendering path
  (`OnPaint`, `shared_texture_enabled`, `SetWindowlessFrameRate`) is not
  available
- The Content API requires implementing the compositor integration yourself,
  which is more work but gives more control over frame delivery
- The PoC should test both approaches if feasible: CEF (simpler, known path) and
  Content API (more control, no profile constraint)

## Impact on Issue 405 (Architecture)

The architecture recommended in Issue 405 (Ghostty fork + out-of-process
Chromium) assumed one Chromium process per profile. If multiple profiles work in
one process via the Content API, the architecture simplifies:

**Before (CEF, one process per profile):**

```
Ghostty Fork
├── Terminal panes (in-process)
├── XPC → Chromium Profile 1 process
├── XPC → Chromium Profile 2 process
└── XPC → Chromium Profile N process
```

**After (Content API, one process, multiple profiles):**

```
Ghostty Fork
├── Terminal panes (in-process)
└── XPC → Single Chromium process
         ├── BrowserContext "work" (Profile 1)
         ├── BrowserContext "personal" (Profile 2)
         └── BrowserContext "guest" (Profile N)
```

The launcher process (`termsurf-launcher`) may no longer be needed. A single
Chromium host process manages all profiles, each with isolated storage.

## Decision

This finding does not change the immediate next step (Issue 407 Chromium
framerate PoC). But it expands the design space:

1. **CEF path:** Proven, simpler to start with, but carries the
   one-profile-per-process constraint and the 300 MB framework size.
2. **Content API path:** More work upfront, but eliminates the profile
   constraint, gives more control over frame delivery, and produces a smaller
   binary.

The PoC should start with CEF (proven path, faster to build) and measure
framerate. If CEF's framerate is sufficient, we can decide later whether to
migrate to the Content API for the profile flexibility. If CEF's framerate is
insufficient, the Content API becomes necessary regardless.

## Sources

- [Chromium Profile Architecture](https://www.chromium.org/developers/design-documents/profile-architecture/)
- [Chromium User Data Directory](https://chromium.googlesource.com/chromium/src/+/master/docs/user_data_dir.md)
- [Chromium ProcessSingleton header](https://chromium.googlesource.com/chromium/src/+/HEAD/chrome/browser/process_singleton.h)
- [Chromium BrowserContext header](https://chromium.googlesource.com/chromium/src/+/master/content/public/browser/browser_context.h)
- [Chromium Content module README](https://chromium.googlesource.com/chromium/src/+/HEAD/content/README.md)
- [CEF cef_settings_t documentation](https://cef-builds.spotifycdn.com/docs/121.0/structcef__settings__t.html)
- [CEF Issue #3670 — root_cache_path warning](https://github.com/chromiumembedded/cef/issues/3670)
- [CEF Issue #3685 — Delete Alloy bootstrap (M128)](https://github.com/chromiumembedded/cef/issues/3685)
- [CefSharp Issue #4961 — cache_path ignored post-M128](https://github.com/cefsharp/CefSharp/issues/4961)
- [Java-CEF Issue #484 — Cannot start multiple processes since CEF 121](https://github.com/chromiumembedded/java-cef/issues/484)
- [CEF Forum — root_cache_path equals user data directory](https://www.magpcss.org/ceforum/viewtopic.php?f=6&t=19674)
- [CEF Forum — One cache multiple instances](https://www.magpcss.org/ceforum/viewtopic.php?f=6&t=18598)
- [Electron Session API](https://www.electronjs.org/docs/latest/api/session)
- [Electron Process Model](https://www.electronjs.org/docs/latest/tutorial/process-model)
- [Electron PR #46089 — ElectronBrowserContext refactor](https://github.com/electron/electron/pull/46089)
