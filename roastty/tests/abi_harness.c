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

static roastty_key_mods_s empty_key_mods(void) {
  roastty_key_mods_s mods = {
      .shift = false,
      .ctrl = false,
      .alt = false,
      .super = false,
      .caps_lock = false,
      .num_lock = false,
      .shift_side = ROASTTY_KEY_SIDE_LEFT,
      .ctrl_side = ROASTTY_KEY_SIDE_LEFT,
      .alt_side = ROASTTY_KEY_SIDE_LEFT,
      .super_side = ROASTTY_KEY_SIDE_LEFT,
  };
  return mods;
}

static void set_key_encoder_option(roastty_key_encoder_t encoder,
                                   roastty_key_encoder_option_e option,
                                   const void *value) {
  assert(roastty_key_encoder_setopt(encoder, option, value) == ROASTTY_SUCCESS);
}

static void assert_key_event_and_encoder_abi(void) {
  roastty_key_event_free(NULL);
  roastty_key_encoder_free(NULL);

  assert(ROASTTY_KEY_UNIDENTIFIED == 0);
  assert(ROASTTY_KEY_KEY_A == 20);
  assert(ROASTTY_KEY_ALT_LEFT == 51);
  assert(ROASTTY_KEY_ARROW_UP == 78);
  assert(ROASTTY_KEY_NUMPAD0 == 80);
  assert(ROASTTY_KEY_F1 == 121);
  assert(ROASTTY_KEY_BROWSER_BACK == 151);
  assert(ROASTTY_KEY_PASTE == 175);

  roastty_key_event_t event = NULL;
  assert(roastty_key_event_new(NULL) == ROASTTY_INVALID_VALUE);
  assert(roastty_key_event_new(&event) == ROASTTY_SUCCESS);
  assert(event != NULL);

  assert(roastty_key_event_set_action(event, ROASTTY_KEY_ACTION_REPEAT) ==
         ROASTTY_SUCCESS);
  assert(roastty_key_event_get_action(event) == ROASTTY_KEY_ACTION_REPEAT);
  assert(roastty_key_event_set_action(event, 9999) == ROASTTY_INVALID_VALUE);
  assert(roastty_key_event_get_action(event) == ROASTTY_KEY_ACTION_REPEAT);

  assert(roastty_key_event_set_key(event, ROASTTY_KEY_ARROW_UP) ==
         ROASTTY_SUCCESS);
  assert(roastty_key_event_get_key(event) == ROASTTY_KEY_ARROW_UP);
  assert(roastty_key_event_set_key(event, 176) == ROASTTY_INVALID_VALUE);
  assert(roastty_key_event_get_key(event) == ROASTTY_KEY_ARROW_UP);

  roastty_key_mods_s mods = empty_key_mods();
  mods.shift = true;
  mods.ctrl = true;
  mods.shift_side = ROASTTY_KEY_SIDE_RIGHT;
  mods.ctrl_side = ROASTTY_KEY_SIDE_RIGHT;
  assert(roastty_key_event_set_mods(event, mods) == ROASTTY_SUCCESS);
  roastty_key_mods_s got_mods = roastty_key_event_get_mods(event);
  assert(got_mods.shift);
  assert(got_mods.ctrl);
  assert(got_mods.shift_side == ROASTTY_KEY_SIDE_RIGHT);
  assert(got_mods.ctrl_side == ROASTTY_KEY_SIDE_RIGHT);
  mods.shift_side = 2;
  assert(roastty_key_event_set_mods(event, mods) == ROASTTY_INVALID_VALUE);

  roastty_key_mods_s consumed = empty_key_mods();
  consumed.alt = true;
  consumed.alt_side = ROASTTY_KEY_SIDE_RIGHT;
  assert(roastty_key_event_set_consumed_mods(event, consumed) ==
         ROASTTY_SUCCESS);
  roastty_key_mods_s got_consumed = roastty_key_event_get_consumed_mods(event);
  assert(got_consumed.alt);
  assert(got_consumed.alt_side == ROASTTY_KEY_SIDE_RIGHT);

  assert(roastty_key_event_set_composing(event, true) == ROASTTY_SUCCESS);
  assert(roastty_key_event_get_composing(event));
  assert(roastty_key_event_set_unshifted_codepoint(event, 'A') ==
         ROASTTY_SUCCESS);
  assert(roastty_key_event_get_unshifted_codepoint(event) == 'A');

  uint8_t text[] = {'o', 'k'};
  assert(roastty_key_event_set_utf8(event, text, sizeof(text)) ==
         ROASTTY_SUCCESS);
  text[0] = 'n';
  size_t text_len = 0;
  const uint8_t *text_ptr = roastty_key_event_get_utf8(event, &text_len);
  assert(text_ptr != NULL);
  assert(text_len == 2);
  assert(memcmp(text_ptr, "ok", text_len) == 0);
  uint8_t bad_utf8[] = {0xff};
  assert(roastty_key_event_set_utf8(event, bad_utf8, sizeof(bad_utf8)) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_key_event_set_utf8(event, NULL, 1) == ROASTTY_INVALID_VALUE);
  assert(roastty_key_event_set_utf8(event, NULL, 0) == ROASTTY_SUCCESS);
  assert(roastty_key_event_get_utf8(event, &text_len) == NULL);
  assert(text_len == 0);
  assert(roastty_key_event_get_utf8(NULL, &text_len) == NULL);
  assert(text_len == 0);

  roastty_key_encoder_t encoder = NULL;
  assert(roastty_key_encoder_new(NULL) == ROASTTY_INVALID_VALUE);
  assert(roastty_key_encoder_new(&encoder) == ROASTTY_SUCCESS);
  assert(encoder != NULL);
  assert(roastty_key_encoder_setopt(encoder,
                                    ROASTTY_KEY_ENCODER_OPTION_ALT_ESC_PREFIX,
                                    NULL) == ROASTTY_INVALID_VALUE);
  int invalid_option = 9999;
  assert(roastty_key_encoder_setopt(encoder, invalid_option, &invalid_option) ==
         ROASTTY_INVALID_VALUE);
  uint8_t bad_flags = 0x20;
  assert(roastty_key_encoder_setopt(encoder,
                                    ROASTTY_KEY_ENCODER_OPTION_KITTY_FLAGS,
                                    &bad_flags) == ROASTTY_INVALID_VALUE);
  int bad_option_as_alt = 4;
  assert(roastty_key_encoder_setopt(
             encoder,
             ROASTTY_KEY_ENCODER_OPTION_MACOS_OPTION_AS_ALT,
             &bad_option_as_alt) == ROASTTY_INVALID_VALUE);

  bool enabled = true;
  bool disabled = false;
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_CURSOR_KEY_APPLICATION,
                         &enabled);
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_KEYPAD_KEY_APPLICATION,
                         &enabled);
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_IGNORE_KEYPAD_WITH_NUMLOCK,
                         &enabled);
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_ALT_ESC_PREFIX,
                         &enabled);
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_MODIFY_OTHER_KEYS_STATE_2,
                         &disabled);
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_BACKARROW_KEY_MODE,
                         &enabled);
  int option_as_alt = ROASTTY_OPTION_AS_ALT_RIGHT;
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_MACOS_OPTION_AS_ALT,
                         &option_as_alt);

  assert(roastty_key_event_set_action(event, ROASTTY_KEY_ACTION_PRESS) ==
         ROASTTY_SUCCESS);
  assert(roastty_key_event_set_composing(event, false) == ROASTTY_SUCCESS);
  assert(roastty_key_event_set_key(event, ROASTTY_KEY_KEY_C) == ROASTTY_SUCCESS);
  mods = empty_key_mods();
  mods.ctrl = true;
  assert(roastty_key_event_set_mods(event, mods) == ROASTTY_SUCCESS);
  assert(roastty_key_event_set_consumed_mods(event, empty_key_mods()) ==
         ROASTTY_SUCCESS);
  assert(roastty_key_event_set_utf8(event, NULL, 0) == ROASTTY_SUCCESS);

  size_t written = 0;
  assert(roastty_key_encoder_encode(NULL, event, NULL, 0, &written) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_key_encoder_encode(encoder, NULL, NULL, 0, &written) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_key_encoder_encode(encoder, event, NULL, 1, &written) ==
         ROASTTY_INVALID_VALUE);
  assert(roastty_key_encoder_encode(encoder, event, NULL, 0, NULL) ==
         ROASTTY_INVALID_VALUE);

  assert(roastty_key_encoder_encode(encoder, event, NULL, 0, &written) ==
         ROASTTY_OUT_OF_SPACE);
  assert(written == 1);
  uint8_t buf[64] = {0};
  assert(roastty_key_encoder_encode(encoder, event, buf, sizeof(buf), &written) ==
         ROASTTY_SUCCESS);
  assert(written == 1);
  assert(buf[0] == 0x03);

  uint8_t kitty_flags = 0x1f;
  set_key_encoder_option(encoder,
                         ROASTTY_KEY_ENCODER_OPTION_KITTY_FLAGS,
                         &kitty_flags);
  assert(roastty_key_event_set_action(event, ROASTTY_KEY_ACTION_RELEASE) ==
         ROASTTY_SUCCESS);
  assert(roastty_key_event_set_key(event, ROASTTY_KEY_CONTROL_LEFT) ==
         ROASTTY_SUCCESS);
  mods = empty_key_mods();
  mods.ctrl = true;
  assert(roastty_key_event_set_mods(event, mods) == ROASTTY_SUCCESS);
  assert(roastty_key_encoder_encode(encoder, event, NULL, 0, &written) ==
         ROASTTY_OUT_OF_SPACE);
  assert(written == strlen("\x1b[57442;5:3u"));
  assert(roastty_key_encoder_encode(encoder, event, buf, sizeof(buf), &written) ==
         ROASTTY_SUCCESS);
  assert(memcmp(buf, "\x1b[57442;5:3u", written) == 0);

  roastty_key_encoder_free(encoder);
  roastty_key_event_free(event);
}

static void feed_osc(roastty_osc_parser_t parser, const char *bytes) {
  for (size_t i = 0; bytes[i] != '\0'; i++) {
    roastty_osc_next(parser, (uint8_t)bytes[i]);
  }
}

static void assert_osc_parser_abi(void) {
  roastty_osc_free(NULL);
  roastty_osc_reset(NULL);
  roastty_osc_next(NULL, 'x');
  assert(roastty_osc_new(NULL) == ROASTTY_INVALID_VALUE);
  assert(roastty_osc_end(NULL, 0) == NULL);
  assert(roastty_osc_command_type(NULL) == ROASTTY_OSC_COMMAND_INVALID);
  assert(!roastty_osc_command_data(NULL,
                                   ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR,
                                   NULL));

  assert(ROASTTY_OSC_COMMAND_INVALID == 0);
  assert(ROASTTY_OSC_COMMAND_CHANGE_WINDOW_TITLE == 1);
  assert(ROASTTY_OSC_COMMAND_CHANGE_WINDOW_ICON == 2);
  assert(ROASTTY_OSC_COMMAND_SEMANTIC_PROMPT == 3);
  assert(ROASTTY_OSC_COMMAND_CONTEXT_SIGNAL == 24);

  roastty_osc_parser_t parser = NULL;
  assert(roastty_osc_new(&parser) == ROASTTY_SUCCESS);
  assert(parser != NULL);

  feed_osc(parser, "0;from-c");
  roastty_osc_command_t command = roastty_osc_end(parser, 0);
  assert(command != NULL);
  assert(roastty_osc_command_type(command) ==
         ROASTTY_OSC_COMMAND_CHANGE_WINDOW_TITLE);

  const char *title = NULL;
  assert(roastty_osc_command_data(command,
                                  ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR,
                                  &title));
  assert(title != NULL);
  assert(strcmp(title, "from-c") == 0);

  const char *unchanged = (const char *)0x1;
  assert(!roastty_osc_command_data(command,
                                   ROASTTY_OSC_COMMAND_DATA_INVALID,
                                   &unchanged));
  assert(unchanged == (const char *)0x1);
  assert(!roastty_osc_command_data(command,
                                   ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR,
                                   NULL));

  feed_osc(parser, "0;second");
  command = roastty_osc_end(parser, 0);
  assert(command != NULL);
  title = NULL;
  assert(roastty_osc_command_data(command,
                                  ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR,
                                  &title));
  assert(title != NULL);
  assert(strcmp(title, "second") == 0);

  feed_osc(parser, "7;file://host/path");
  command = roastty_osc_end(parser, 0);
  assert(command != NULL);
  assert(roastty_osc_command_type(command) == ROASTTY_OSC_COMMAND_REPORT_PWD);
  title = (const char *)0x1;
  assert(!roastty_osc_command_data(command,
                                   ROASTTY_OSC_COMMAND_DATA_CHANGE_WINDOW_TITLE_STR,
                                   &title));
  assert(title == (const char *)0x1);

  feed_osc(parser, "4;2;?");
  command = roastty_osc_end(parser, 0x07);
  assert(command != NULL);
  assert(roastty_osc_command_type(command) == ROASTTY_OSC_COMMAND_COLOR_OPERATION);

  feed_osc(parser, "0;bad");
  assert(roastty_osc_end(parser, 9999) == NULL);

  roastty_osc_reset(parser);
  roastty_osc_free(parser);
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
  assert_key_event_and_encoder_abi();
  assert_osc_parser_abi();

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
