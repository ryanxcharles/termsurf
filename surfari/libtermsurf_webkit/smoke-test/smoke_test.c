#include "libtermsurf_webkit.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct State {
    ts_browser_context_t persistent_context;
    ts_browser_context_t incognito_context;
    ts_web_contents_t web_contents;
    const char *first_url;
    const char *second_url;
    int initialized;
    int tab_ready;
    int context_id_count;
    int url_changed;
    int loading_started;
    int loading_finished;
    int title_changed;
    int navigations_finished;
    int resized;
};

static void fail(const char *message)
{
    fprintf(stderr, "SMOKE_FAIL %s\n", message);
    fflush(stderr);
    exit(1);
}

static void finish(void *user_data)
{
    struct State *state = (struct State *)user_data;
    if (!state->initialized)
        fail("initialized callback missing");
    if (!state->persistent_context || !state->incognito_context)
        fail("context creation failed");
    if (!state->web_contents)
        fail("web contents creation failed");
    if (!state->tab_ready)
        fail("tab ready callback missing");
    if (!state->context_id_count)
        fail("ca context id callback missing");
    if (!state->url_changed)
        fail("url changed callback missing");
    if (!state->loading_started || !state->loading_finished)
        fail("loading callbacks missing");
    if (!state->title_changed)
        fail("title changed callback missing");
    if (state->navigations_finished < 2)
        fail("second navigation did not finish");
    if (!state->resized)
        fail("resize callback missing");

    ts_destroy_web_contents(state->web_contents);
    ts_destroy_browser_context(state->persistent_context);
    ts_destroy_browser_context(state->incognito_context);
    printf("SMOKE_PASS initialized=%d tab_ready=%d ca_context=%d url=%d loading_started=%d loading_finished=%d title=%d navigations=%d resized=%d\n",
        state->initialized,
        state->tab_ready,
        state->context_id_count,
        state->url_changed,
        state->loading_started,
        state->loading_finished,
        state->title_changed,
        state->navigations_finished,
        state->resized);
    fflush(stdout);
    ts_quit();
}

static void resize_after_navigation(void *user_data)
{
    struct State *state = (struct State *)user_data;
    ts_set_view_size(state->web_contents, 640, 480, 0, 0, 640, 480, 2.0);
    state->resized = 1;
    ts_post_task(finish, state);
}

static void on_initialized(void *user_data)
{
    struct State *state = (struct State *)user_data;
    state->initialized = 1;
    puts("CALLBACK initialized");
    state->persistent_context = ts_create_browser_context(NULL);
    state->incognito_context = ts_create_incognito_browser_context();
    state->web_contents = ts_create_web_contents(state->persistent_context, state->first_url, 320, 240, false);
}

static void on_tab_ready(ts_web_contents_t wc, int tab_id, void *user_data)
{
    (void)wc;
    struct State *state = (struct State *)user_data;
    if (tab_id <= 0)
        fail("tab id was not positive");
    state->tab_ready = 1;
    printf("CALLBACK tab_ready tab_id=%d\n", tab_id);
}

static void on_ca_context_id(ts_web_contents_t wc, uint32_t context_id, int width, int height, void *user_data)
{
    (void)wc;
    struct State *state = (struct State *)user_data;
    if (!context_id)
        fail("context id was zero");
    if (width <= 0 || height <= 0)
        fail("context size was invalid");
    state->context_id_count++;
    printf("CALLBACK ca_context_id context_id=%u width=%d height=%d\n", context_id, width, height);
}

static void on_url_changed(ts_web_contents_t wc, const char *url, void *user_data)
{
    (void)wc;
    struct State *state = (struct State *)user_data;
    state->url_changed++;
    printf("CALLBACK url_changed url=%s\n", url ? url : "");
}

static void on_loading_state(ts_web_contents_t wc, const char *url, int loading, void *user_data)
{
    (void)wc;
    struct State *state = (struct State *)user_data;
    if (loading)
        state->loading_started++;
    else {
        state->loading_finished++;
        state->navigations_finished++;
        if (state->navigations_finished == 1) {
            ts_load_url(state->web_contents, state->second_url);
        } else if (state->navigations_finished == 2) {
            ts_post_task(resize_after_navigation, state);
        }
    }
    printf("CALLBACK loading_state loading=%d url=%s\n", loading, url ? url : "");
}

static void on_title_changed(ts_web_contents_t wc, const char *title, void *user_data)
{
    (void)wc;
    struct State *state = (struct State *)user_data;
    if (title && strstr(title, "Surfari"))
        state->title_changed++;
    printf("CALLBACK title_changed title=%s\n", title ? title : "");
}

int main(int argc, const char **argv)
{
    if (argc != 3) {
        fprintf(stderr, "usage: %s <first-url> <second-url>\n", argv[0]);
        return 2;
    }

    struct State state = {
        .first_url = argv[1],
        .second_url = argv[2],
    };

    ts_set_on_initialized(on_initialized, &state);
    ts_set_on_tab_ready(on_tab_ready, &state);
    ts_set_on_ca_context_id(on_ca_context_id, &state);
    ts_set_on_url_changed(on_url_changed, &state);
    ts_set_on_loading_state(on_loading_state, &state);
    ts_set_on_title_changed(on_title_changed, &state);

    return ts_content_main(argc, argv);
}
