#import "libtermsurf_webkit.h"

#import <Cocoa/Cocoa.h>
#import <QuartzCore/QuartzCore.h>
#import <WebKit/WebKit.h>

#include <atomic>

@interface CAContext : NSObject
+ (instancetype)remoteContextWithOptions:(NSDictionary *)options;
@property(nonatomic, readonly) uint32_t contextId;
@property(nonatomic, retain) CALayer *layer;
- (void)invalidate;
@end

struct CallbackState {
    ts_initialized_cb on_initialized = nullptr;
    void *on_initialized_data = nullptr;
    ts_tab_ready_cb on_tab_ready = nullptr;
    void *on_tab_ready_data = nullptr;
    ts_ca_context_id_cb on_ca_context_id = nullptr;
    void *on_ca_context_id_data = nullptr;
    ts_url_changed_cb on_url_changed = nullptr;
    void *on_url_changed_data = nullptr;
    ts_loading_state_cb on_loading_state = nullptr;
    void *on_loading_state_data = nullptr;
    ts_title_changed_cb on_title_changed = nullptr;
    void *on_title_changed_data = nullptr;
    ts_cursor_changed_cb on_cursor_changed = nullptr;
    void *on_cursor_changed_data = nullptr;
    ts_target_url_changed_cb on_target_url_changed = nullptr;
    void *on_target_url_changed_data = nullptr;
    ts_javascript_dialog_request_cb on_javascript_dialog_request = nullptr;
    void *on_javascript_dialog_request_data = nullptr;
    ts_console_message_cb on_console_message = nullptr;
    void *on_console_message_data = nullptr;
    ts_http_auth_request_cb on_http_auth_request = nullptr;
    void *on_http_auth_request_data = nullptr;
    ts_renderer_crashed_cb on_renderer_crashed = nullptr;
    void *on_renderer_crashed_data = nullptr;
};

static CallbackState g_callbacks;
static std::atomic<int> g_next_tab_id{1};

struct BrowserContext {
    WKWebsiteDataStore *data_store;
};

struct WebContents;

@interface TSNavigationDelegate : NSObject <WKNavigationDelegate>
@property(nonatomic) WebContents *owner;
@end

struct WebContents {
    int tab_id;
    NSWindow *window;
    WKWebView *web_view;
    TSNavigationDelegate *navigation_delegate;
    CAContext *remote_context;
    int width;
    int height;
};

static NSString *stringFromCString(const char *value)
{
    if (!value)
        return @"";
    return [NSString stringWithUTF8String:value] ?: @"";
}

static NSURL *urlFromCString(const char *value)
{
    NSString *string = stringFromCString(value);
    if ([string length] == 0)
        return nil;

    NSURL *url = [NSURL URLWithString:string];
    if (url && url.scheme)
        return url;

    return [NSURL fileURLWithPath:string];
}

static void withCString(NSString *value, void (^block)(const char *))
{
    block(value ? [value UTF8String] : "");
}

static void fireLoading(WebContents *contents, NSString *url, int loading)
{
    if (!g_callbacks.on_loading_state)
        return;
    withCString(url, ^(const char *c_url) {
        g_callbacks.on_loading_state(contents, c_url, loading, g_callbacks.on_loading_state_data);
    });
}

static void fireUrl(WebContents *contents, NSString *url)
{
    if (!g_callbacks.on_url_changed)
        return;
    withCString(url, ^(const char *c_url) {
        g_callbacks.on_url_changed(contents, c_url, g_callbacks.on_url_changed_data);
    });
}

static void fireTitle(WebContents *contents, NSString *title)
{
    if (!g_callbacks.on_title_changed)
        return;
    withCString(title, ^(const char *c_title) {
        g_callbacks.on_title_changed(contents, c_title, g_callbacks.on_title_changed_data);
    });
}

static void exportContext(WebContents *contents)
{
    if (!contents || !contents->web_view)
        return;

    [contents->web_view layoutSubtreeIfNeeded];
    if (!contents->remote_context) {
        contents->remote_context = [CAContext remoteContextWithOptions:@{
            @"kCAContextCIFilterBehavior" : @"ignore",
        }];
        contents->remote_context.layer = contents->web_view.layer;
    }

    if (g_callbacks.on_ca_context_id) {
        g_callbacks.on_ca_context_id(
            contents,
            contents->remote_context.contextId,
            contents->width,
            contents->height,
            g_callbacks.on_ca_context_id_data);
    }
}

@implementation TSNavigationDelegate
- (void)webView:(WKWebView *)webView didStartProvisionalNavigation:(WKNavigation *)navigation
{
    (void)navigation;
    fireLoading(self.owner, webView.URL.absoluteString, 1);
}

- (void)webView:(WKWebView *)webView didCommitNavigation:(WKNavigation *)navigation
{
    (void)navigation;
    fireUrl(self.owner, webView.URL.absoluteString);
}

- (void)webView:(WKWebView *)webView didFinishNavigation:(WKNavigation *)navigation
{
    (void)navigation;
    fireUrl(self.owner, webView.URL.absoluteString);
    [webView evaluateJavaScript:@"document.title" completionHandler:^(id result, NSError *error) {
        if (error)
            NSLog(@"[libtermsurf_webkit] document.title evaluation failed: %@", error);
        NSString *title = [result isKindOfClass:NSString.class] ? result : webView.title;
        fireTitle(self.owner, title);
        fireLoading(self.owner, webView.URL.absoluteString, 0);
        exportContext(self.owner);
    }];
}

- (void)webView:(WKWebView *)webView didFailNavigation:(WKNavigation *)navigation withError:(NSError *)error
{
    (void)navigation;
    NSLog(@"[libtermsurf_webkit] navigation failed: %@", error);
    fireLoading(self.owner, webView.URL.absoluteString, 0);
}

- (void)webView:(WKWebView *)webView didFailProvisionalNavigation:(WKNavigation *)navigation withError:(NSError *)error
{
    (void)navigation;
    NSLog(@"[libtermsurf_webkit] provisional navigation failed: %@", error);
    fireLoading(self.owner, webView.URL.absoluteString, 0);
}
@end

int ts_content_main(int argc, const char *const *argv)
{
    (void)argc;
    (void)argv;

    @autoreleasepool {
        NSApplication *application = [NSApplication sharedApplication];
        [application setActivationPolicy:NSApplicationActivationPolicyAccessory];

        dispatch_async(dispatch_get_main_queue(), ^{
            if (g_callbacks.on_initialized)
                g_callbacks.on_initialized(g_callbacks.on_initialized_data);
        });

        [application run];
    }

    return 0;
}

void ts_set_on_initialized(ts_initialized_cb callback, void *user_data)
{
    g_callbacks.on_initialized = callback;
    g_callbacks.on_initialized_data = user_data;
}

void ts_post_task(ts_task_cb task, void *user_data)
{
    if (!task)
        return;
    dispatch_async(dispatch_get_main_queue(), ^{
        task(user_data);
    });
}

void ts_quit(void)
{
    dispatch_async(dispatch_get_main_queue(), ^{
        [NSApp terminate:nil];
    });
}

ts_browser_context_t ts_create_browser_context(const char *path)
{
    (void)path;
    BrowserContext *context = new BrowserContext;
    context->data_store = [WKWebsiteDataStore defaultDataStore];
    return context;
}

ts_browser_context_t ts_create_incognito_browser_context(void)
{
    BrowserContext *context = new BrowserContext;
    context->data_store = [WKWebsiteDataStore nonPersistentDataStore];
    return context;
}

void ts_destroy_browser_context(ts_browser_context_t ctx)
{
    delete static_cast<BrowserContext *>(ctx);
}

ts_web_contents_t ts_create_web_contents(ts_browser_context_t ctx, const char *url, int width, int height, bool dark)
{
    (void)dark;
    BrowserContext *context = static_cast<BrowserContext *>(ctx);
    if (!context)
        return nullptr;

    WebContents *contents = new WebContents;
    contents->tab_id = g_next_tab_id.fetch_add(1);
    contents->width = width;
    contents->height = height;

    NSRect frame = NSMakeRect(80, 80, MAX(width, 64), MAX(height, 64));
    contents->window = [[NSWindow alloc] initWithContentRect:frame styleMask:NSWindowStyleMaskBorderless backing:NSBackingStoreBuffered defer:NO];
    contents->window.releasedWhenClosed = NO;
    contents->window.title = @"libtermsurf_webkit";

    WKWebViewConfiguration *configuration = [[WKWebViewConfiguration alloc] init];
    configuration.websiteDataStore = context->data_store;
    contents->web_view = [[WKWebView alloc] initWithFrame:contents->window.contentView.bounds configuration:configuration];
    contents->web_view.autoresizingMask = NSViewWidthSizable | NSViewHeightSizable;
    contents->web_view.wantsLayer = YES;

    contents->navigation_delegate = [[TSNavigationDelegate alloc] init];
    contents->navigation_delegate.owner = contents;
    contents->web_view.navigationDelegate = contents->navigation_delegate;

    [contents->window.contentView addSubview:contents->web_view];
    [contents->window orderFront:nil];

    if (g_callbacks.on_tab_ready)
        g_callbacks.on_tab_ready(contents, contents->tab_id, g_callbacks.on_tab_ready_data);

    exportContext(contents);
    ts_load_url(contents, url);
    return contents;
}

ts_web_contents_t ts_create_devtools_web_contents(
    ts_browser_context_t ctx,
    int inspected_tab_id,
    int width,
    int height,
    bool dark)
{
    (void)ctx;
    (void)inspected_tab_id;
    (void)width;
    (void)height;
    (void)dark;
    return nullptr;
}

void ts_destroy_web_contents(ts_web_contents_t wc)
{
    WebContents *contents = static_cast<WebContents *>(wc);
    if (!contents)
        return;

    [contents->remote_context invalidate];
    contents->web_view.navigationDelegate = nil;
    [contents->web_view removeFromSuperview];
    [contents->window close];
    delete contents;
}

void ts_load_url(ts_web_contents_t wc, const char *url)
{
    WebContents *contents = static_cast<WebContents *>(wc);
    NSURL *ns_url = urlFromCString(url);
    if (!contents || !ns_url)
        return;

    if (ns_url.isFileURL) {
        NSURL *directory = [ns_url URLByDeletingLastPathComponent];
        [contents->web_view loadFileURL:ns_url allowingReadAccessToURL:directory];
        return;
    }

    [contents->web_view loadRequest:[NSURLRequest requestWithURL:ns_url]];
}

void ts_set_view_size(
    ts_web_contents_t wc,
    int width,
    int height,
    double screen_x,
    double screen_y,
    double screen_width,
    double screen_height,
    double screen_scale)
{
    (void)screen_x;
    (void)screen_y;
    (void)screen_width;
    (void)screen_height;
    (void)screen_scale;

    WebContents *contents = static_cast<WebContents *>(wc);
    if (!contents)
        return;

    contents->width = width;
    contents->height = height;
    NSRect frame = contents->window.frame;
    frame.size = NSMakeSize(MAX(width, 64), MAX(height, 64));
    [contents->window setFrame:frame display:YES animate:NO];
    contents->web_view.frame = contents->window.contentView.bounds;
    [contents->web_view layoutSubtreeIfNeeded];
    exportContext(contents);
}

void ts_forward_mouse_event(ts_web_contents_t wc, int type, int button, int x, int y, int click_count, int modifiers)
{
    (void)wc;
    (void)type;
    (void)button;
    (void)x;
    (void)y;
    (void)click_count;
    (void)modifiers;
}

void ts_forward_mouse_move(ts_web_contents_t wc, int x, int y, int modifiers)
{
    (void)wc;
    (void)x;
    (void)y;
    (void)modifiers;
}

void ts_forward_scroll_event(
    ts_web_contents_t wc,
    int x,
    int y,
    float delta_x,
    float delta_y,
    int phase,
    int momentum_phase,
    bool precise,
    int modifiers)
{
    (void)wc;
    (void)x;
    (void)y;
    (void)delta_x;
    (void)delta_y;
    (void)phase;
    (void)momentum_phase;
    (void)precise;
    (void)modifiers;
}

void ts_forward_key_event(ts_web_contents_t wc, int type, int keycode, const char *utf8, int modifiers)
{
    (void)wc;
    (void)type;
    (void)keycode;
    (void)utf8;
    (void)modifiers;
}

void ts_set_focus(ts_web_contents_t wc, bool focused)
{
    (void)wc;
    (void)focused;
}

void ts_set_gui_active(ts_web_contents_t wc, bool active, const char *reason)
{
    (void)wc;
    (void)active;
    (void)reason;
}

void ts_set_color_scheme(ts_web_contents_t wc, bool dark)
{
    (void)wc;
    (void)dark;
}

bool ts_reply_javascript_dialog(ts_web_contents_t wc, uint64_t request_id, bool accepted, const char *prompt_text)
{
    (void)wc;
    (void)request_id;
    (void)accepted;
    (void)prompt_text;
    return false;
}

bool ts_reply_http_auth(ts_web_contents_t wc, uint64_t request_id, bool accepted, const char *username, const char *password)
{
    (void)wc;
    (void)request_id;
    (void)accepted;
    (void)username;
    (void)password;
    return false;
}

void ts_set_on_tab_ready(ts_tab_ready_cb cb, void *user_data)
{
    g_callbacks.on_tab_ready = cb;
    g_callbacks.on_tab_ready_data = user_data;
}

void ts_set_on_ca_context_id(ts_ca_context_id_cb cb, void *user_data)
{
    g_callbacks.on_ca_context_id = cb;
    g_callbacks.on_ca_context_id_data = user_data;
}

void ts_set_on_url_changed(ts_url_changed_cb cb, void *user_data)
{
    g_callbacks.on_url_changed = cb;
    g_callbacks.on_url_changed_data = user_data;
}

void ts_set_on_loading_state(ts_loading_state_cb cb, void *user_data)
{
    g_callbacks.on_loading_state = cb;
    g_callbacks.on_loading_state_data = user_data;
}

void ts_set_on_title_changed(ts_title_changed_cb cb, void *user_data)
{
    g_callbacks.on_title_changed = cb;
    g_callbacks.on_title_changed_data = user_data;
}

void ts_set_on_cursor_changed(ts_cursor_changed_cb cb, void *user_data)
{
    g_callbacks.on_cursor_changed = cb;
    g_callbacks.on_cursor_changed_data = user_data;
}

void ts_set_on_target_url_changed(ts_target_url_changed_cb cb, void *user_data)
{
    g_callbacks.on_target_url_changed = cb;
    g_callbacks.on_target_url_changed_data = user_data;
}

void ts_set_on_javascript_dialog_request(ts_javascript_dialog_request_cb cb, void *user_data)
{
    g_callbacks.on_javascript_dialog_request = cb;
    g_callbacks.on_javascript_dialog_request_data = user_data;
}

void ts_set_on_console_message(ts_console_message_cb cb, void *user_data)
{
    g_callbacks.on_console_message = cb;
    g_callbacks.on_console_message_data = user_data;
}

void ts_set_on_http_auth_request(ts_http_auth_request_cb cb, void *user_data)
{
    g_callbacks.on_http_auth_request = cb;
    g_callbacks.on_http_auth_request_data = user_data;
}

void ts_set_on_renderer_crashed(ts_renderer_crashed_cb cb, void *user_data)
{
    g_callbacks.on_renderer_crashed = cb;
    g_callbacks.on_renderer_crashed_data = user_data;
}
