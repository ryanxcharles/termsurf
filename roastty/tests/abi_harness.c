#include <assert.h>
#include <stdint.h>
#include <string.h>

#include "roastty.h"

static void wakeup_cb(void *userdata) {
  (void)userdata;
}

static bool action_cb(roastty_app_t app,
                      roastty_target_s target,
                      roastty_action_s action) {
  (void)app;
  (void)target;
  (void)action;
  return false;
}

static bool read_clipboard_cb(void *userdata,
                              roastty_clipboard_e clipboard,
                              void *state) {
  (void)userdata;
  (void)clipboard;
  (void)state;
  return false;
}

static void confirm_read_clipboard_cb(void *userdata,
                                      const char *str,
                                      void *state,
                                      roastty_clipboard_request_e request) {
  (void)userdata;
  (void)str;
  (void)state;
  (void)request;
}

static void write_clipboard_cb(void *userdata,
                               roastty_clipboard_e clipboard,
                               const roastty_clipboard_content_s *content,
                               size_t len,
                               bool confirm) {
  (void)userdata;
  (void)clipboard;
  (void)content;
  (void)len;
  (void)confirm;
}

static void close_surface_cb(void *userdata, bool process_alive) {
  (void)userdata;
  (void)process_alive;
}

static void assert_config_bool(roastty_config_t config,
                               const char *key,
                               bool expected) {
  bool value = !expected;
  assert(roastty_config_get(config, &value, key, strlen(key)));
  assert(value == expected);
}

static void assert_config_string(roastty_config_t config,
                                 const char *key,
                                 const char *expected) {
  const char *value = NULL;
  assert(roastty_config_get(config, &value, key, strlen(key)));
  assert(value != NULL);
  assert(strcmp(value, expected) == 0);
}

static void assert_config_double(roastty_config_t config,
                                 const char *key,
                                 double expected) {
  double value = -1.0;
  assert(roastty_config_get(config, &value, key, strlen(key)));
  assert(value == expected);
}

static void assert_config_uintptr(roastty_config_t config,
                                  const char *key,
                                  uintptr_t expected) {
  uintptr_t value = 0;
  assert(roastty_config_get(config, &value, key, strlen(key)));
  assert(value == expected);
}

static void assert_mouse_event_abi(void) {
  roastty_mouse_event_free(NULL);

  roastty_mouse_event_t event = NULL;
  assert(roastty_mouse_event_new(NULL) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_event_new(&event) == ROASTTY_SUCCESS);
  assert(event != NULL);

  assert(roastty_mouse_event_set_action(event, ROASTTY_MOUSE_ACTION_MOTION) ==
         ROASTTY_SUCCESS);
  assert(roastty_mouse_event_get_action(event) == ROASTTY_MOUSE_ACTION_MOTION);
  assert(roastty_mouse_event_set_action(event, ROASTTY_MOUSE_ACTION_RELEASE) ==
         ROASTTY_SUCCESS);
  assert(roastty_mouse_event_get_action(event) == ROASTTY_MOUSE_ACTION_RELEASE);
  assert(roastty_mouse_event_set_action(event, ROASTTY_MOUSE_ACTION_PRESS) ==
         ROASTTY_SUCCESS);
  assert(roastty_mouse_event_set_action(event, 9999) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_event_get_action(event) == ROASTTY_MOUSE_ACTION_PRESS);

  int button = -1;
  assert(!roastty_mouse_event_get_button(event, &button));
  assert(roastty_mouse_event_set_button(event, ROASTTY_MOUSE_BUTTON_LEFT) ==
         ROASTTY_SUCCESS);
  assert(roastty_mouse_event_get_button(event, &button));
  assert(button == ROASTTY_MOUSE_BUTTON_LEFT);
  assert(roastty_mouse_event_get_button(event, NULL));
  assert(roastty_mouse_event_set_button(event, ROASTTY_MOUSE_BUTTON_EIGHT) ==
         ROASTTY_SUCCESS);
  assert(roastty_mouse_event_get_button(event, &button));
  assert(button == ROASTTY_MOUSE_BUTTON_EIGHT);
  assert(roastty_mouse_event_set_button(event, 9999) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_event_get_button(event, &button));
  assert(button == ROASTTY_MOUSE_BUTTON_EIGHT);
  roastty_mouse_event_clear_button(event);
  assert(!roastty_mouse_event_get_button(event, &button));

  roastty_mouse_mods_s mods = {
      .shift = true,
      .alt = false,
      .ctrl = true,
  };
  roastty_mouse_event_set_mods(event, mods);
  roastty_mouse_mods_s got_mods = roastty_mouse_event_get_mods(event);
  assert(got_mods.shift);
  assert(!got_mods.alt);
  assert(got_mods.ctrl);

  roastty_mouse_position_s pos = {
      .x = 12.5f,
      .y = -4.0f,
  };
  roastty_mouse_event_set_position(event, pos);
  roastty_mouse_position_s got_pos = roastty_mouse_event_get_position(event);
  assert(got_pos.x == 12.5f);
  assert(got_pos.y == -4.0f);

  roastty_mouse_event_free(event);
}

static void set_mouse_encoder_option(roastty_mouse_encoder_t encoder,
                                     roastty_mouse_encoder_option_e option,
                                     const void *value) {
  assert(roastty_mouse_encoder_setopt(encoder, option, value) == ROASTTY_SUCCESS);
}

static void assert_mouse_encoder_abi(void) {
  roastty_mouse_encoder_free(NULL);
  roastty_mouse_encoder_reset(NULL);

  roastty_mouse_encoder_t encoder = NULL;
  assert(roastty_mouse_encoder_new(NULL) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_new(&encoder) == ROASTTY_SUCCESS);
  assert(encoder != NULL);

  assert(roastty_mouse_encoder_setopt(NULL,
                                      ROASTTY_MOUSE_ENCODER_OPTION_EVENT,
                                      NULL) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_setopt(encoder,
                                      ROASTTY_MOUSE_ENCODER_OPTION_EVENT,
                                      NULL) == ROASTTY_INVALID_VALUE);

  int event_mode = ROASTTY_MOUSE_TRACKING_ANY;
  set_mouse_encoder_option(encoder, ROASTTY_MOUSE_ENCODER_OPTION_EVENT, &event_mode);
  int format = ROASTTY_MOUSE_FORMAT_SGR;
  set_mouse_encoder_option(encoder, ROASTTY_MOUSE_ENCODER_OPTION_FORMAT, &format);
  roastty_mouse_encoder_size_s size = {
      .size = sizeof(roastty_mouse_encoder_size_s),
      .screen_width = 1000,
      .screen_height = 1000,
      .cell_width = 1,
      .cell_height = 1,
      .padding_top = 0,
      .padding_bottom = 0,
      .padding_right = 0,
      .padding_left = 0,
  };
  set_mouse_encoder_option(encoder, ROASTTY_MOUSE_ENCODER_OPTION_SIZE, &size);
  bool any_button_pressed = true;
  set_mouse_encoder_option(encoder,
                           ROASTTY_MOUSE_ENCODER_OPTION_ANY_BUTTON_PRESSED,
                           &any_button_pressed);

  int invalid_enum = 9999;
  assert(roastty_mouse_encoder_setopt(encoder,
                                      ROASTTY_MOUSE_ENCODER_OPTION_EVENT,
                                      &invalid_enum) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_setopt(encoder,
                                      ROASTTY_MOUSE_ENCODER_OPTION_FORMAT,
                                      &invalid_enum) == ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_setopt(encoder, 9999, &invalid_enum) ==
         ROASTTY_INVALID_VALUE);
  roastty_mouse_encoder_size_s bad_size = size;
  bad_size.cell_width = 0;
  assert(roastty_mouse_encoder_setopt(encoder,
                                      ROASTTY_MOUSE_ENCODER_OPTION_SIZE,
                                      &bad_size) == ROASTTY_INVALID_VALUE);
  bad_size = size;
  bad_size.size = sizeof(size_t);
  assert(roastty_mouse_encoder_setopt(encoder,
                                      ROASTTY_MOUSE_ENCODER_OPTION_SIZE,
                                      &bad_size) == ROASTTY_INVALID_VALUE);
  struct tiny_size {
    size_t size;
  };
  struct tiny_size tiny_size = {
      .size = sizeof(tiny_size),
  };
  assert(roastty_mouse_encoder_setopt(encoder,
                                      ROASTTY_MOUSE_ENCODER_OPTION_SIZE,
                                      &tiny_size) == ROASTTY_INVALID_VALUE);

  roastty_mouse_event_t event = NULL;
  assert(roastty_mouse_event_new(&event) == ROASTTY_SUCCESS);
  assert(roastty_mouse_event_set_action(event, ROASTTY_MOUSE_ACTION_PRESS) ==
         ROASTTY_SUCCESS);
  assert(roastty_mouse_event_set_button(event, ROASTTY_MOUSE_BUTTON_LEFT) ==
         ROASTTY_SUCCESS);
  roastty_mouse_position_s pos = {
      .x = 0.0f,
      .y = 0.0f,
  };
  roastty_mouse_event_set_position(event, pos);

  size_t written = 0;
  assert(roastty_mouse_encoder_encode(NULL, event, NULL, 0, &written) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_encode(encoder, NULL, NULL, 0, &written) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_encode(encoder, event, NULL, 1, &written) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_mouse_encoder_encode(encoder, event, NULL, 0, NULL) ==
         ROASTTY_INVALID_VALUE);

  assert(roastty_mouse_encoder_encode(encoder, event, NULL, 0, &written) ==
         ROASTTY_OUT_OF_SPACE);
  assert(written == strlen("\x1b[<0;1;1M"));

  uint8_t tiny[2] = {0};
  assert(roastty_mouse_encoder_encode(encoder, event, tiny, sizeof(tiny), &written) ==
         ROASTTY_OUT_OF_SPACE);
  assert(written == strlen("\x1b[<0;1;1M"));

  uint8_t buf[32] = {0};
  assert(roastty_mouse_encoder_encode(encoder, event, buf, sizeof(buf), &written) ==
         ROASTTY_SUCCESS);
  assert(written == strlen("\x1b[<0;1;1M"));
  assert(memcmp(buf, "\x1b[<0;1;1M", written) == 0);

  bool track_last_cell = true;
  set_mouse_encoder_option(encoder,
                           ROASTTY_MOUSE_ENCODER_OPTION_TRACK_LAST_CELL,
                           &track_last_cell);
  assert(roastty_mouse_event_set_action(event, ROASTTY_MOUSE_ACTION_MOTION) ==
         ROASTTY_SUCCESS);
  pos.x = 5.0f;
  pos.y = 6.0f;
  roastty_mouse_event_set_position(event, pos);

  assert(roastty_mouse_encoder_encode(encoder, event, NULL, 0, &written) ==
         ROASTTY_OUT_OF_SPACE);
  assert(written > 0);
  assert(roastty_mouse_encoder_encode(encoder, event, buf, sizeof(buf), &written) ==
         ROASTTY_SUCCESS);
  assert(written > 0);
  assert(roastty_mouse_encoder_encode(encoder, event, buf, sizeof(buf), &written) ==
         ROASTTY_SUCCESS);
  assert(written == 0);
  roastty_mouse_encoder_reset(encoder);
  assert(roastty_mouse_encoder_encode(encoder, event, buf, sizeof(buf), &written) ==
         ROASTTY_SUCCESS);
  assert(written > 0);

  roastty_mouse_event_free(event);
  roastty_mouse_encoder_free(encoder);
}

int main(int argc, char **argv) {
  assert(roastty_init((uintptr_t)argc, argv) == ROASTTY_SUCCESS);

  roastty_config_free(NULL);
  roastty_app_free(NULL);
  roastty_surface_free(NULL);
  roastty_config_load_cli_args(NULL);
  roastty_config_load_default_files(NULL);
  roastty_config_load_recursive_files(NULL);
  roastty_config_load_file(NULL, NULL);
  roastty_config_finalize(NULL);
  assert(roastty_config_diagnostics_count(NULL) == 0);
  assert(roastty_config_get_diagnostic(NULL, 0).message != NULL);
  assert(roastty_app_userdata(NULL) == NULL);
  roastty_app_tick(NULL);
  roastty_app_set_focus(NULL, true);
  roastty_app_set_color_scheme(NULL, ROASTTY_COLOR_SCHEME_DARK);
  roastty_app_update_config(NULL, NULL);
  assert(!roastty_app_needs_confirm_quit(NULL));
  assert(!roastty_app_has_global_keybinds(NULL));
  assert(roastty_surface_userdata(NULL) == NULL);
  assert(roastty_surface_app(NULL) == NULL);
  roastty_surface_update_config(NULL, NULL);
  assert(!roastty_surface_needs_confirm_quit(NULL));
  assert(!roastty_surface_process_exited(NULL));
  roastty_surface_set_content_scale(NULL, 1.0, 1.0);
  roastty_surface_set_focus(NULL, true);
  roastty_surface_set_occlusion(NULL, true);
  roastty_surface_set_color_scheme(NULL, ROASTTY_COLOR_SCHEME_DARK);
  roastty_surface_set_size(NULL, 1, 1);
  roastty_surface_size_s null_size = roastty_surface_size(NULL);
  assert(null_size.width_px == 0);
  assert(null_size.height_px == 0);
  assert(null_size.columns == 0);
  assert(null_size.rows == 0);
  assert(null_size.cell_width_px == 0);
  assert(null_size.cell_height_px == 0);
  assert(roastty_surface_foreground_pid(NULL) == 0);
  roastty_surface_request_close(NULL);
  assert_mouse_event_abi();
  assert_mouse_encoder_abi();

  roastty_info_s info = roastty_info();
  assert(info.version != NULL);
  assert(info.version_len > 0);

  roastty_config_t config = roastty_config_new();
  assert(config != NULL);
  roastty_config_load_cli_args(config);
  roastty_config_load_default_files(config);
  roastty_config_load_recursive_files(config);
  roastty_config_load_file(config, "/tmp/nonexistent-roastty-config");
  roastty_config_finalize(config);
  assert(roastty_config_diagnostics_count(config) == 0);
  roastty_diagnostic_s diagnostic = roastty_config_get_diagnostic(config, 0);
  assert(diagnostic.message != NULL);

  bool bool_value = false;
  assert(!roastty_config_get(NULL,
                             &bool_value,
                             "initial-window",
                             strlen("initial-window")));
  assert(!roastty_config_get(config,
                             NULL,
                             "initial-window",
                             strlen("initial-window")));
  assert(!roastty_config_get(config, &bool_value, NULL, strlen("initial-window")));
  assert(!roastty_config_get(config,
                             &bool_value,
                             "not-a-real-key",
                             strlen("not-a-real-key")));

  assert_config_bool(config, "initial-window", true);
  assert_config_bool(config, "quit-after-last-window-closed", false);
  assert_config_string(config, "window-save-state", "default");
  assert_config_string(config, "window-decoration", "auto");
  assert_config_string(config, "window-theme", "auto");
  assert_config_double(config, "background-opacity", 1.0);
  assert_config_double(config, "bell-audio-volume", 0.5);
  assert_config_uintptr(config, "notify-on-command-finish-after", 5000);

  int16_t optional_position = 123;
  assert(!roastty_config_get(config,
                             &optional_position,
                             "window-position-x",
                             strlen("window-position-x")));
  assert(optional_position == 123);
  assert(!roastty_config_get(config,
                             &optional_position,
                             "window-position-y",
                             strlen("window-position-y")));
  assert(optional_position == 123);

  const char *nullable_title = (const char *)0x1;
  assert(roastty_config_get(config, &nullable_title, "title", strlen("title")));
  assert(nullable_title == NULL);

  roastty_config_path_s path = {
      .path = (const char *)0x1,
      .optional = true,
  };
  assert(!roastty_config_get(config,
                             &path,
                             "bell-audio-path",
                             strlen("bell-audio-path")));
  assert(path.path == (const char *)0x1);
  assert(path.optional == true);

  const char padded_key[] = "window-theme-with-extra-bytes";
  const char *theme = NULL;
  assert(roastty_config_get(config, &theme, padded_key, strlen("window-theme")));
  assert(theme != NULL);
  assert(strcmp(theme, "auto") == 0);

  roastty_config_t clone = roastty_config_clone(config);
  assert(clone != NULL);
  roastty_config_free(clone);

  roastty_string_s open_path = roastty_config_open_path();
  assert(open_path.ptr != NULL);
  assert(open_path.len == strlen("roastty-config"));
  assert(open_path.sentinel == false);
  roastty_string_free(open_path);

  uintptr_t app_userdata = 0xA991;
  roastty_runtime_config_s runtime = {
      .userdata = (void *)app_userdata,
      .supports_selection_clipboard = true,
      .wakeup_cb = wakeup_cb,
      .action_cb = action_cb,
      .read_clipboard_cb = read_clipboard_cb,
      .confirm_read_clipboard_cb = confirm_read_clipboard_cb,
      .write_clipboard_cb = write_clipboard_cb,
      .close_surface_cb = close_surface_cb,
  };

  roastty_app_t app = roastty_app_new(&runtime, config);
  assert(app != NULL);
  assert((uintptr_t)roastty_app_userdata(app) == app_userdata);
  roastty_app_tick(app);
  roastty_app_set_focus(app, true);
  roastty_app_set_color_scheme(app, ROASTTY_COLOR_SCHEME_DARK);
  roastty_app_update_config(app, config);
  assert(!roastty_app_needs_confirm_quit(app));
  assert(!roastty_app_has_global_keybinds(app));

  uintptr_t surface_userdata = 0x5A5A;
  roastty_surface_config_s surface_config = roastty_surface_config_new();
  surface_config.userdata = (void *)surface_userdata;
  surface_config.scale_factor = 2.0;
  surface_config.context = ROASTTY_SURFACE_CONTEXT_WINDOW;

  roastty_app_t app_with_null_runtime = roastty_app_new(NULL, config);
  assert(app_with_null_runtime != NULL);
  assert(roastty_app_userdata(app_with_null_runtime) == NULL);
  roastty_app_free(app_with_null_runtime);

  roastty_surface_t surface_with_null_config = roastty_surface_new(app, NULL);
  assert(surface_with_null_config != NULL);
  assert(roastty_surface_app(surface_with_null_config) == app);
  roastty_surface_free(surface_with_null_config);

  assert(roastty_surface_new(NULL, &surface_config) == NULL);

  roastty_surface_t surface = roastty_surface_new(app, &surface_config);
  assert(surface != NULL);
  assert(roastty_surface_app(surface) == app);
  assert((uintptr_t)roastty_surface_userdata(surface) == surface_userdata);

  roastty_surface_set_content_scale(surface, 2.0, 2.0);
  roastty_surface_set_focus(surface, true);
  roastty_surface_set_occlusion(surface, false);
  roastty_surface_set_color_scheme(surface, ROASTTY_COLOR_SCHEME_LIGHT);
  roastty_surface_set_size(surface, 1024, 768);

  roastty_surface_size_s size = roastty_surface_size(surface);
  assert(size.width_px == 1024);
  assert(size.height_px == 768);
  assert(size.columns == 0);
  assert(size.rows == 0);
  assert(size.cell_width_px == 0);
  assert(size.cell_height_px == 0);

  assert(roastty_surface_foreground_pid(surface) == 0);
  assert(!roastty_surface_needs_confirm_quit(surface));
  assert(!roastty_surface_process_exited(surface));

  roastty_string_s tty_name = roastty_surface_tty_name(surface);
  assert(tty_name.ptr != NULL);
  assert(tty_name.len == strlen("roastty-skeleton-tty"));
  assert(tty_name.sentinel == true);
  roastty_string_free(tty_name);

  roastty_surface_request_close(surface);
  roastty_surface_free(surface);

  roastty_string_s empty_tty = roastty_surface_tty_name(NULL);
  assert(empty_tty.ptr == NULL);
  assert(empty_tty.len == 0);
  assert(empty_tty.sentinel == false);
  roastty_string_free(empty_tty);

  for (int i = 0; i < 16; i++) {
    roastty_surface_t loop_surface = roastty_surface_new(app, &surface_config);
    assert(loop_surface != NULL);
    roastty_surface_free(loop_surface);
  }

  roastty_app_free(app);
  roastty_config_free(config);
  return 0;
}
