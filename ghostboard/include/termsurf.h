// TermSurf embedding API. The documentation for the embedding API is
// only within the Zig source files that define the implementations. This
// isn't meant to be a general purpose embedding API (yet) so there hasn't
// been documentation or example work beyond that.
//
// The only consumer of this API is the macOS app, but the API is built to
// be more general purpose.
#ifndef TERMSURF_H
#define TERMSURF_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

//-------------------------------------------------------------------
// Macros

#define TERMSURF_SUCCESS 0

//-------------------------------------------------------------------
// Types

// Opaque types
typedef void* termsurf_app_t;
typedef void* termsurf_config_t;
typedef void* termsurf_surface_t;
typedef void* termsurf_inspector_t;

// All the types below are fully defined and must be kept in sync with
// their Zig counterparts. Any changes to these types MUST have an associated
// Zig change.
typedef enum {
  TERMSURF_PLATFORM_INVALID,
  TERMSURF_PLATFORM_MACOS,
  TERMSURF_PLATFORM_IOS,
} termsurf_platform_e;

typedef enum {
  TERMSURF_CLIPBOARD_STANDARD,
  TERMSURF_CLIPBOARD_SELECTION,
} termsurf_clipboard_e;

typedef struct {
  const char *mime;
  const char *data;
} termsurf_clipboard_content_s;

typedef enum {
  TERMSURF_CLIPBOARD_REQUEST_PASTE,
  TERMSURF_CLIPBOARD_REQUEST_OSC_52_READ,
  TERMSURF_CLIPBOARD_REQUEST_OSC_52_WRITE,
} termsurf_clipboard_request_e;

typedef enum {
  TERMSURF_MOUSE_RELEASE,
  TERMSURF_MOUSE_PRESS,
} termsurf_input_mouse_state_e;

typedef enum {
  TERMSURF_MOUSE_UNKNOWN,
  TERMSURF_MOUSE_LEFT,
  TERMSURF_MOUSE_RIGHT,
  TERMSURF_MOUSE_MIDDLE,
  TERMSURF_MOUSE_FOUR,
  TERMSURF_MOUSE_FIVE,
  TERMSURF_MOUSE_SIX,
  TERMSURF_MOUSE_SEVEN,
  TERMSURF_MOUSE_EIGHT,
  TERMSURF_MOUSE_NINE,
  TERMSURF_MOUSE_TEN,
  TERMSURF_MOUSE_ELEVEN,
} termsurf_input_mouse_button_e;

typedef enum {
  TERMSURF_MOUSE_MOMENTUM_NONE,
  TERMSURF_MOUSE_MOMENTUM_BEGAN,
  TERMSURF_MOUSE_MOMENTUM_STATIONARY,
  TERMSURF_MOUSE_MOMENTUM_CHANGED,
  TERMSURF_MOUSE_MOMENTUM_ENDED,
  TERMSURF_MOUSE_MOMENTUM_CANCELLED,
  TERMSURF_MOUSE_MOMENTUM_MAY_BEGIN,
} termsurf_input_mouse_momentum_e;

typedef enum {
  TERMSURF_COLOR_SCHEME_LIGHT = 0,
  TERMSURF_COLOR_SCHEME_DARK = 1,
} termsurf_color_scheme_e;

// This is a packed struct (see src/input/mouse.zig) but the C standard
// afaik doesn't let us reliably define packed structs so we build it up
// from scratch.
typedef int termsurf_input_scroll_mods_t;

typedef enum {
  TERMSURF_MODS_NONE = 0,
  TERMSURF_MODS_SHIFT = 1 << 0,
  TERMSURF_MODS_CTRL = 1 << 1,
  TERMSURF_MODS_ALT = 1 << 2,
  TERMSURF_MODS_SUPER = 1 << 3,
  TERMSURF_MODS_CAPS = 1 << 4,
  TERMSURF_MODS_NUM = 1 << 5,
  TERMSURF_MODS_SHIFT_RIGHT = 1 << 6,
  TERMSURF_MODS_CTRL_RIGHT = 1 << 7,
  TERMSURF_MODS_ALT_RIGHT = 1 << 8,
  TERMSURF_MODS_SUPER_RIGHT = 1 << 9,
} termsurf_input_mods_e;

typedef enum {
  TERMSURF_BINDING_FLAGS_CONSUMED = 1 << 0,
  TERMSURF_BINDING_FLAGS_ALL = 1 << 1,
  TERMSURF_BINDING_FLAGS_GLOBAL = 1 << 2,
  TERMSURF_BINDING_FLAGS_PERFORMABLE = 1 << 3,
} termsurf_binding_flags_e;

typedef enum {
  TERMSURF_ACTION_RELEASE,
  TERMSURF_ACTION_PRESS,
  TERMSURF_ACTION_REPEAT,
} termsurf_input_action_e;

// Based on: https://www.w3.org/TR/uievents-code/
typedef enum {
  TERMSURF_KEY_UNIDENTIFIED,

  // "Writing System Keys" § 3.1.1
  TERMSURF_KEY_BACKQUOTE,
  TERMSURF_KEY_BACKSLASH,
  TERMSURF_KEY_BRACKET_LEFT,
  TERMSURF_KEY_BRACKET_RIGHT,
  TERMSURF_KEY_COMMA,
  TERMSURF_KEY_DIGIT_0,
  TERMSURF_KEY_DIGIT_1,
  TERMSURF_KEY_DIGIT_2,
  TERMSURF_KEY_DIGIT_3,
  TERMSURF_KEY_DIGIT_4,
  TERMSURF_KEY_DIGIT_5,
  TERMSURF_KEY_DIGIT_6,
  TERMSURF_KEY_DIGIT_7,
  TERMSURF_KEY_DIGIT_8,
  TERMSURF_KEY_DIGIT_9,
  TERMSURF_KEY_EQUAL,
  TERMSURF_KEY_INTL_BACKSLASH,
  TERMSURF_KEY_INTL_RO,
  TERMSURF_KEY_INTL_YEN,
  TERMSURF_KEY_A,
  TERMSURF_KEY_B,
  TERMSURF_KEY_C,
  TERMSURF_KEY_D,
  TERMSURF_KEY_E,
  TERMSURF_KEY_F,
  TERMSURF_KEY_G,
  TERMSURF_KEY_H,
  TERMSURF_KEY_I,
  TERMSURF_KEY_J,
  TERMSURF_KEY_K,
  TERMSURF_KEY_L,
  TERMSURF_KEY_M,
  TERMSURF_KEY_N,
  TERMSURF_KEY_O,
  TERMSURF_KEY_P,
  TERMSURF_KEY_Q,
  TERMSURF_KEY_R,
  TERMSURF_KEY_S,
  TERMSURF_KEY_T,
  TERMSURF_KEY_U,
  TERMSURF_KEY_V,
  TERMSURF_KEY_W,
  TERMSURF_KEY_X,
  TERMSURF_KEY_Y,
  TERMSURF_KEY_Z,
  TERMSURF_KEY_MINUS,
  TERMSURF_KEY_PERIOD,
  TERMSURF_KEY_QUOTE,
  TERMSURF_KEY_SEMICOLON,
  TERMSURF_KEY_SLASH,

  // "Functional Keys" § 3.1.2
  TERMSURF_KEY_ALT_LEFT,
  TERMSURF_KEY_ALT_RIGHT,
  TERMSURF_KEY_BACKSPACE,
  TERMSURF_KEY_CAPS_LOCK,
  TERMSURF_KEY_CONTEXT_MENU,
  TERMSURF_KEY_CONTROL_LEFT,
  TERMSURF_KEY_CONTROL_RIGHT,
  TERMSURF_KEY_ENTER,
  TERMSURF_KEY_META_LEFT,
  TERMSURF_KEY_META_RIGHT,
  TERMSURF_KEY_SHIFT_LEFT,
  TERMSURF_KEY_SHIFT_RIGHT,
  TERMSURF_KEY_SPACE,
  TERMSURF_KEY_TAB,
  TERMSURF_KEY_CONVERT,
  TERMSURF_KEY_KANA_MODE,
  TERMSURF_KEY_NON_CONVERT,

  // "Control Pad Section" § 3.2
  TERMSURF_KEY_DELETE,
  TERMSURF_KEY_END,
  TERMSURF_KEY_HELP,
  TERMSURF_KEY_HOME,
  TERMSURF_KEY_INSERT,
  TERMSURF_KEY_PAGE_DOWN,
  TERMSURF_KEY_PAGE_UP,

  // "Arrow Pad Section" § 3.3
  TERMSURF_KEY_ARROW_DOWN,
  TERMSURF_KEY_ARROW_LEFT,
  TERMSURF_KEY_ARROW_RIGHT,
  TERMSURF_KEY_ARROW_UP,

  // "Numpad Section" § 3.4
  TERMSURF_KEY_NUM_LOCK,
  TERMSURF_KEY_NUMPAD_0,
  TERMSURF_KEY_NUMPAD_1,
  TERMSURF_KEY_NUMPAD_2,
  TERMSURF_KEY_NUMPAD_3,
  TERMSURF_KEY_NUMPAD_4,
  TERMSURF_KEY_NUMPAD_5,
  TERMSURF_KEY_NUMPAD_6,
  TERMSURF_KEY_NUMPAD_7,
  TERMSURF_KEY_NUMPAD_8,
  TERMSURF_KEY_NUMPAD_9,
  TERMSURF_KEY_NUMPAD_ADD,
  TERMSURF_KEY_NUMPAD_BACKSPACE,
  TERMSURF_KEY_NUMPAD_CLEAR,
  TERMSURF_KEY_NUMPAD_CLEAR_ENTRY,
  TERMSURF_KEY_NUMPAD_COMMA,
  TERMSURF_KEY_NUMPAD_DECIMAL,
  TERMSURF_KEY_NUMPAD_DIVIDE,
  TERMSURF_KEY_NUMPAD_ENTER,
  TERMSURF_KEY_NUMPAD_EQUAL,
  TERMSURF_KEY_NUMPAD_MEMORY_ADD,
  TERMSURF_KEY_NUMPAD_MEMORY_CLEAR,
  TERMSURF_KEY_NUMPAD_MEMORY_RECALL,
  TERMSURF_KEY_NUMPAD_MEMORY_STORE,
  TERMSURF_KEY_NUMPAD_MEMORY_SUBTRACT,
  TERMSURF_KEY_NUMPAD_MULTIPLY,
  TERMSURF_KEY_NUMPAD_PAREN_LEFT,
  TERMSURF_KEY_NUMPAD_PAREN_RIGHT,
  TERMSURF_KEY_NUMPAD_SUBTRACT,
  TERMSURF_KEY_NUMPAD_SEPARATOR,
  TERMSURF_KEY_NUMPAD_UP,
  TERMSURF_KEY_NUMPAD_DOWN,
  TERMSURF_KEY_NUMPAD_RIGHT,
  TERMSURF_KEY_NUMPAD_LEFT,
  TERMSURF_KEY_NUMPAD_BEGIN,
  TERMSURF_KEY_NUMPAD_HOME,
  TERMSURF_KEY_NUMPAD_END,
  TERMSURF_KEY_NUMPAD_INSERT,
  TERMSURF_KEY_NUMPAD_DELETE,
  TERMSURF_KEY_NUMPAD_PAGE_UP,
  TERMSURF_KEY_NUMPAD_PAGE_DOWN,

  // "Function Section" § 3.5
  TERMSURF_KEY_ESCAPE,
  TERMSURF_KEY_F1,
  TERMSURF_KEY_F2,
  TERMSURF_KEY_F3,
  TERMSURF_KEY_F4,
  TERMSURF_KEY_F5,
  TERMSURF_KEY_F6,
  TERMSURF_KEY_F7,
  TERMSURF_KEY_F8,
  TERMSURF_KEY_F9,
  TERMSURF_KEY_F10,
  TERMSURF_KEY_F11,
  TERMSURF_KEY_F12,
  TERMSURF_KEY_F13,
  TERMSURF_KEY_F14,
  TERMSURF_KEY_F15,
  TERMSURF_KEY_F16,
  TERMSURF_KEY_F17,
  TERMSURF_KEY_F18,
  TERMSURF_KEY_F19,
  TERMSURF_KEY_F20,
  TERMSURF_KEY_F21,
  TERMSURF_KEY_F22,
  TERMSURF_KEY_F23,
  TERMSURF_KEY_F24,
  TERMSURF_KEY_F25,
  TERMSURF_KEY_FN,
  TERMSURF_KEY_FN_LOCK,
  TERMSURF_KEY_PRINT_SCREEN,
  TERMSURF_KEY_SCROLL_LOCK,
  TERMSURF_KEY_PAUSE,

  // "Media Keys" § 3.6
  TERMSURF_KEY_BROWSER_BACK,
  TERMSURF_KEY_BROWSER_FAVORITES,
  TERMSURF_KEY_BROWSER_FORWARD,
  TERMSURF_KEY_BROWSER_HOME,
  TERMSURF_KEY_BROWSER_REFRESH,
  TERMSURF_KEY_BROWSER_SEARCH,
  TERMSURF_KEY_BROWSER_STOP,
  TERMSURF_KEY_EJECT,
  TERMSURF_KEY_LAUNCH_APP_1,
  TERMSURF_KEY_LAUNCH_APP_2,
  TERMSURF_KEY_LAUNCH_MAIL,
  TERMSURF_KEY_MEDIA_PLAY_PAUSE,
  TERMSURF_KEY_MEDIA_SELECT,
  TERMSURF_KEY_MEDIA_STOP,
  TERMSURF_KEY_MEDIA_TRACK_NEXT,
  TERMSURF_KEY_MEDIA_TRACK_PREVIOUS,
  TERMSURF_KEY_POWER,
  TERMSURF_KEY_SLEEP,
  TERMSURF_KEY_AUDIO_VOLUME_DOWN,
  TERMSURF_KEY_AUDIO_VOLUME_MUTE,
  TERMSURF_KEY_AUDIO_VOLUME_UP,
  TERMSURF_KEY_WAKE_UP,

  // "Legacy, Non-standard, and Special Keys" § 3.7
  TERMSURF_KEY_COPY,
  TERMSURF_KEY_CUT,
  TERMSURF_KEY_PASTE,
} termsurf_input_key_e;

typedef struct {
  termsurf_input_action_e action;
  termsurf_input_mods_e mods;
  termsurf_input_mods_e consumed_mods;
  uint32_t keycode;
  const char* text;
  uint32_t unshifted_codepoint;
  bool composing;
} termsurf_input_key_s;

typedef enum {
  TERMSURF_TRIGGER_PHYSICAL,
  TERMSURF_TRIGGER_UNICODE,
  TERMSURF_TRIGGER_CATCH_ALL,
} termsurf_input_trigger_tag_e;

typedef union {
  termsurf_input_key_e translated;
  termsurf_input_key_e physical;
  uint32_t unicode;
  // catch_all has no payload
} termsurf_input_trigger_key_u;

typedef struct {
  termsurf_input_trigger_tag_e tag;
  termsurf_input_trigger_key_u key;
  termsurf_input_mods_e mods;
} termsurf_input_trigger_s;

typedef struct {
  const char* action_key;
  const char* action;
  const char* title;
  const char* description;
} termsurf_command_s;

typedef enum {
  TERMSURF_BUILD_MODE_DEBUG,
  TERMSURF_BUILD_MODE_RELEASE_SAFE,
  TERMSURF_BUILD_MODE_RELEASE_FAST,
  TERMSURF_BUILD_MODE_RELEASE_SMALL,
} termsurf_build_mode_e;

typedef struct {
  termsurf_build_mode_e build_mode;
  const char* version;
  uintptr_t version_len;
} termsurf_info_s;

typedef struct {
  const char* message;
} termsurf_diagnostic_s;

typedef struct {
  const char* ptr;
  uintptr_t len;
  bool sentinel;
} termsurf_string_s;

typedef struct {
  double tl_px_x;
  double tl_px_y;
  uint32_t offset_start;
  uint32_t offset_len;
  const char* text;
  uintptr_t text_len;
} termsurf_text_s;

typedef enum {
  TERMSURF_POINT_ACTIVE,
  TERMSURF_POINT_VIEWPORT,
  TERMSURF_POINT_SCREEN,
  TERMSURF_POINT_SURFACE,
} termsurf_point_tag_e;

typedef enum {
  TERMSURF_POINT_COORD_EXACT,
  TERMSURF_POINT_COORD_TOP_LEFT,
  TERMSURF_POINT_COORD_BOTTOM_RIGHT,
} termsurf_point_coord_e;

typedef struct {
  termsurf_point_tag_e tag;
  termsurf_point_coord_e coord;
  uint32_t x;
  uint32_t y;
} termsurf_point_s;

typedef struct {
  termsurf_point_s top_left;
  termsurf_point_s bottom_right;
  bool rectangle;
} termsurf_selection_s;

typedef struct {
  const char* key;
  const char* value;
} termsurf_env_var_s;

typedef struct {
  void* nsview;
} termsurf_platform_macos_s;

typedef struct {
  void* uiview;
} termsurf_platform_ios_s;

typedef union {
  termsurf_platform_macos_s macos;
  termsurf_platform_ios_s ios;
} termsurf_platform_u;

typedef enum {
  TERMSURF_SURFACE_CONTEXT_WINDOW = 0,
  TERMSURF_SURFACE_CONTEXT_TAB = 1,
  TERMSURF_SURFACE_CONTEXT_SPLIT = 2,
} termsurf_surface_context_e;

typedef struct {
  termsurf_platform_e platform_tag;
  termsurf_platform_u platform;
  void* userdata;
  double scale_factor;
  float font_size;
  const char* working_directory;
  const char* command;
  termsurf_env_var_s* env_vars;
  size_t env_var_count;
  const char* initial_input;
  bool wait_after_command;
  termsurf_surface_context_e context;
} termsurf_surface_config_s;

typedef struct {
  uint16_t columns;
  uint16_t rows;
  uint32_t width_px;
  uint32_t height_px;
  uint32_t cell_width_px;
  uint32_t cell_height_px;
} termsurf_surface_size_s;

// Config types

// config.Color
typedef struct {
  uint8_t r;
  uint8_t g;
  uint8_t b;
} termsurf_config_color_s;

// config.ColorList
typedef struct {
  const termsurf_config_color_s* colors;
  size_t len;
} termsurf_config_color_list_s;

// config.RepeatableCommand
typedef struct {
  const termsurf_command_s* commands;
  size_t len;
} termsurf_config_command_list_s;

// config.Palette
typedef struct {
  termsurf_config_color_s colors[256];
} termsurf_config_palette_s;

// config.QuickTerminalSize
typedef enum {
  TERMSURF_QUICK_TERMINAL_SIZE_NONE,
  TERMSURF_QUICK_TERMINAL_SIZE_PERCENTAGE,
  TERMSURF_QUICK_TERMINAL_SIZE_PIXELS,
} termsurf_quick_terminal_size_tag_e;

typedef union {
  float percentage;
  uint32_t pixels;
} termsurf_quick_terminal_size_value_u;

typedef struct {
  termsurf_quick_terminal_size_tag_e tag;
  termsurf_quick_terminal_size_value_u value;
} termsurf_quick_terminal_size_s;

typedef struct {
  termsurf_quick_terminal_size_s primary;
  termsurf_quick_terminal_size_s secondary;
} termsurf_config_quick_terminal_size_s;

// apprt.Target.Key
typedef enum {
  TERMSURF_TARGET_APP,
  TERMSURF_TARGET_SURFACE,
} termsurf_target_tag_e;

typedef union {
  termsurf_surface_t surface;
} termsurf_target_u;

typedef struct {
  termsurf_target_tag_e tag;
  termsurf_target_u target;
} termsurf_target_s;

// apprt.action.SplitDirection
typedef enum {
  TERMSURF_SPLIT_DIRECTION_RIGHT,
  TERMSURF_SPLIT_DIRECTION_DOWN,
  TERMSURF_SPLIT_DIRECTION_LEFT,
  TERMSURF_SPLIT_DIRECTION_UP,
} termsurf_action_split_direction_e;

// apprt.action.GotoSplit
typedef enum {
  TERMSURF_GOTO_SPLIT_PREVIOUS,
  TERMSURF_GOTO_SPLIT_NEXT,
  TERMSURF_GOTO_SPLIT_UP,
  TERMSURF_GOTO_SPLIT_LEFT,
  TERMSURF_GOTO_SPLIT_DOWN,
  TERMSURF_GOTO_SPLIT_RIGHT,
} termsurf_action_goto_split_e;

// apprt.action.GotoWindow
typedef enum {
  TERMSURF_GOTO_WINDOW_PREVIOUS,
  TERMSURF_GOTO_WINDOW_NEXT,
} termsurf_action_goto_window_e;

// apprt.action.ResizeSplit.Direction
typedef enum {
  TERMSURF_RESIZE_SPLIT_UP,
  TERMSURF_RESIZE_SPLIT_DOWN,
  TERMSURF_RESIZE_SPLIT_LEFT,
  TERMSURF_RESIZE_SPLIT_RIGHT,
} termsurf_action_resize_split_direction_e;

// apprt.action.ResizeSplit
typedef struct {
  uint16_t amount;
  termsurf_action_resize_split_direction_e direction;
} termsurf_action_resize_split_s;

// apprt.action.MoveTab
typedef struct {
  ssize_t amount;
} termsurf_action_move_tab_s;

// apprt.action.GotoTab
typedef enum {
  TERMSURF_GOTO_TAB_PREVIOUS = -1,
  TERMSURF_GOTO_TAB_NEXT = -2,
  TERMSURF_GOTO_TAB_LAST = -3,
} termsurf_action_goto_tab_e;

// apprt.action.Fullscreen
typedef enum {
  TERMSURF_FULLSCREEN_NATIVE,
  TERMSURF_FULLSCREEN_NON_NATIVE,
  TERMSURF_FULLSCREEN_NON_NATIVE_VISIBLE_MENU,
  TERMSURF_FULLSCREEN_NON_NATIVE_PADDED_NOTCH,
} termsurf_action_fullscreen_e;

// apprt.action.FloatWindow
typedef enum {
  TERMSURF_FLOAT_WINDOW_ON,
  TERMSURF_FLOAT_WINDOW_OFF,
  TERMSURF_FLOAT_WINDOW_TOGGLE,
} termsurf_action_float_window_e;

// apprt.action.SecureInput
typedef enum {
  TERMSURF_SECURE_INPUT_ON,
  TERMSURF_SECURE_INPUT_OFF,
  TERMSURF_SECURE_INPUT_TOGGLE,
} termsurf_action_secure_input_e;

// apprt.action.Inspector
typedef enum {
  TERMSURF_INSPECTOR_TOGGLE,
  TERMSURF_INSPECTOR_SHOW,
  TERMSURF_INSPECTOR_HIDE,
} termsurf_action_inspector_e;

// apprt.action.QuitTimer
typedef enum {
  TERMSURF_QUIT_TIMER_START,
  TERMSURF_QUIT_TIMER_STOP,
} termsurf_action_quit_timer_e;

// apprt.action.Readonly
typedef enum {
  TERMSURF_READONLY_OFF,
  TERMSURF_READONLY_ON,
} termsurf_action_readonly_e;

// apprt.action.DesktopNotification.C
typedef struct {
  const char* title;
  const char* body;
} termsurf_action_desktop_notification_s;

// apprt.action.SetTitle.C
typedef struct {
  const char* title;
} termsurf_action_set_title_s;

// apprt.action.PromptTitle
typedef enum {
  TERMSURF_PROMPT_TITLE_SURFACE,
  TERMSURF_PROMPT_TITLE_TAB,
} termsurf_action_prompt_title_e;

// apprt.action.Pwd.C
typedef struct {
  const char* pwd;
} termsurf_action_pwd_s;

// terminal.MouseShape
typedef enum {
  TERMSURF_MOUSE_SHAPE_DEFAULT,
  TERMSURF_MOUSE_SHAPE_CONTEXT_MENU,
  TERMSURF_MOUSE_SHAPE_HELP,
  TERMSURF_MOUSE_SHAPE_POINTER,
  TERMSURF_MOUSE_SHAPE_PROGRESS,
  TERMSURF_MOUSE_SHAPE_WAIT,
  TERMSURF_MOUSE_SHAPE_CELL,
  TERMSURF_MOUSE_SHAPE_CROSSHAIR,
  TERMSURF_MOUSE_SHAPE_TEXT,
  TERMSURF_MOUSE_SHAPE_VERTICAL_TEXT,
  TERMSURF_MOUSE_SHAPE_ALIAS,
  TERMSURF_MOUSE_SHAPE_COPY,
  TERMSURF_MOUSE_SHAPE_MOVE,
  TERMSURF_MOUSE_SHAPE_NO_DROP,
  TERMSURF_MOUSE_SHAPE_NOT_ALLOWED,
  TERMSURF_MOUSE_SHAPE_GRAB,
  TERMSURF_MOUSE_SHAPE_GRABBING,
  TERMSURF_MOUSE_SHAPE_ALL_SCROLL,
  TERMSURF_MOUSE_SHAPE_COL_RESIZE,
  TERMSURF_MOUSE_SHAPE_ROW_RESIZE,
  TERMSURF_MOUSE_SHAPE_N_RESIZE,
  TERMSURF_MOUSE_SHAPE_E_RESIZE,
  TERMSURF_MOUSE_SHAPE_S_RESIZE,
  TERMSURF_MOUSE_SHAPE_W_RESIZE,
  TERMSURF_MOUSE_SHAPE_NE_RESIZE,
  TERMSURF_MOUSE_SHAPE_NW_RESIZE,
  TERMSURF_MOUSE_SHAPE_SE_RESIZE,
  TERMSURF_MOUSE_SHAPE_SW_RESIZE,
  TERMSURF_MOUSE_SHAPE_EW_RESIZE,
  TERMSURF_MOUSE_SHAPE_NS_RESIZE,
  TERMSURF_MOUSE_SHAPE_NESW_RESIZE,
  TERMSURF_MOUSE_SHAPE_NWSE_RESIZE,
  TERMSURF_MOUSE_SHAPE_ZOOM_IN,
  TERMSURF_MOUSE_SHAPE_ZOOM_OUT,
} termsurf_action_mouse_shape_e;

// apprt.action.MouseVisibility
typedef enum {
  TERMSURF_MOUSE_VISIBLE,
  TERMSURF_MOUSE_HIDDEN,
} termsurf_action_mouse_visibility_e;

// apprt.action.MouseOverLink
typedef struct {
  const char* url;
  size_t len;
} termsurf_action_mouse_over_link_s;

// apprt.action.SizeLimit
typedef struct {
  uint32_t min_width;
  uint32_t min_height;
  uint32_t max_width;
  uint32_t max_height;
} termsurf_action_size_limit_s;

// apprt.action.InitialSize
typedef struct {
  uint32_t width;
  uint32_t height;
} termsurf_action_initial_size_s;

// apprt.action.CellSize
typedef struct {
  uint32_t width;
  uint32_t height;
} termsurf_action_cell_size_s;

// renderer.Health
typedef enum {
  TERMSURF_RENDERER_HEALTH_OK,
  TERMSURF_RENDERER_HEALTH_UNHEALTHY,
} termsurf_action_renderer_health_e;

// apprt.action.KeySequence
typedef struct {
  bool active;
  termsurf_input_trigger_s trigger;
} termsurf_action_key_sequence_s;

// apprt.action.KeyTable.Tag
typedef enum {
  TERMSURF_KEY_TABLE_ACTIVATE,
  TERMSURF_KEY_TABLE_DEACTIVATE,
  TERMSURF_KEY_TABLE_DEACTIVATE_ALL,
} termsurf_action_key_table_tag_e;

// apprt.action.KeyTable.CValue
typedef union {
  struct {
    const char *name;
    size_t len;
  } activate;
} termsurf_action_key_table_u;

// apprt.action.KeyTable.C
typedef struct {
  termsurf_action_key_table_tag_e tag;
  termsurf_action_key_table_u value;
} termsurf_action_key_table_s;

// apprt.action.ColorKind
typedef enum {
  TERMSURF_ACTION_COLOR_KIND_FOREGROUND = -1,
  TERMSURF_ACTION_COLOR_KIND_BACKGROUND = -2,
  TERMSURF_ACTION_COLOR_KIND_CURSOR = -3,
} termsurf_action_color_kind_e;

// apprt.action.ColorChange
typedef struct {
  termsurf_action_color_kind_e kind;
  uint8_t r;
  uint8_t g;
  uint8_t b;
} termsurf_action_color_change_s;

// apprt.action.ConfigChange
typedef struct {
  termsurf_config_t config;
} termsurf_action_config_change_s;

// apprt.action.ReloadConfig
typedef struct {
  bool soft;
} termsurf_action_reload_config_s;

// apprt.action.OpenUrlKind
typedef enum {
  TERMSURF_ACTION_OPEN_URL_KIND_UNKNOWN,
  TERMSURF_ACTION_OPEN_URL_KIND_TEXT,
  TERMSURF_ACTION_OPEN_URL_KIND_HTML,
} termsurf_action_open_url_kind_e;

// apprt.action.OpenUrl.C
typedef struct {
  termsurf_action_open_url_kind_e kind;
  const char* url;
  uintptr_t len;
} termsurf_action_open_url_s;

// apprt.action.CloseTabMode
typedef enum {
  TERMSURF_ACTION_CLOSE_TAB_MODE_THIS,
  TERMSURF_ACTION_CLOSE_TAB_MODE_OTHER,
  TERMSURF_ACTION_CLOSE_TAB_MODE_RIGHT,
} termsurf_action_close_tab_mode_e;

// apprt.surface.Message.ChildExited
typedef struct {
  uint32_t exit_code;
  uint64_t timetime_ms;
} termsurf_surface_message_childexited_s;

// terminal.osc.Command.ProgressReport.State
typedef enum {
  TERMSURF_PROGRESS_STATE_REMOVE,
  TERMSURF_PROGRESS_STATE_SET,
  TERMSURF_PROGRESS_STATE_ERROR,
  TERMSURF_PROGRESS_STATE_INDETERMINATE,
  TERMSURF_PROGRESS_STATE_PAUSE,
} termsurf_action_progress_report_state_e;

// terminal.osc.Command.ProgressReport.C
typedef struct {
  termsurf_action_progress_report_state_e state;
  // -1 if no progress was reported, otherwise 0-100 indicating percent
  // completeness.
  int8_t progress;
} termsurf_action_progress_report_s;

// apprt.action.CommandFinished.C
typedef struct {
  // -1 if no exit code was reported, otherwise 0-255
  int16_t exit_code;
  // number of nanoseconds that command was running for
  uint64_t duration;
} termsurf_action_command_finished_s;

// apprt.action.StartSearch.C
typedef struct {
  const char* needle;
} termsurf_action_start_search_s;

// apprt.action.SearchTotal
typedef struct {
  ssize_t total;
} termsurf_action_search_total_s;

// apprt.action.SearchSelected
typedef struct {
  ssize_t selected;
} termsurf_action_search_selected_s;

// terminal.Scrollbar
typedef struct {
  uint64_t total;
  uint64_t offset;
  uint64_t len;
} termsurf_action_scrollbar_s;

// apprt.Action.Key
typedef enum {
  TERMSURF_ACTION_QUIT,
  TERMSURF_ACTION_NEW_WINDOW,
  TERMSURF_ACTION_NEW_TAB,
  TERMSURF_ACTION_CLOSE_TAB,
  TERMSURF_ACTION_NEW_SPLIT,
  TERMSURF_ACTION_CLOSE_ALL_WINDOWS,
  TERMSURF_ACTION_TOGGLE_MAXIMIZE,
  TERMSURF_ACTION_TOGGLE_FULLSCREEN,
  TERMSURF_ACTION_TOGGLE_TAB_OVERVIEW,
  TERMSURF_ACTION_TOGGLE_WINDOW_DECORATIONS,
  TERMSURF_ACTION_TOGGLE_QUICK_TERMINAL,
  TERMSURF_ACTION_TOGGLE_COMMAND_PALETTE,
  TERMSURF_ACTION_TOGGLE_VISIBILITY,
  TERMSURF_ACTION_TOGGLE_BACKGROUND_OPACITY,
  TERMSURF_ACTION_MOVE_TAB,
  TERMSURF_ACTION_GOTO_TAB,
  TERMSURF_ACTION_GOTO_SPLIT,
  TERMSURF_ACTION_GOTO_WINDOW,
  TERMSURF_ACTION_RESIZE_SPLIT,
  TERMSURF_ACTION_EQUALIZE_SPLITS,
  TERMSURF_ACTION_TOGGLE_SPLIT_ZOOM,
  TERMSURF_ACTION_PRESENT_TERMINAL,
  TERMSURF_ACTION_SIZE_LIMIT,
  TERMSURF_ACTION_RESET_WINDOW_SIZE,
  TERMSURF_ACTION_INITIAL_SIZE,
  TERMSURF_ACTION_CELL_SIZE,
  TERMSURF_ACTION_SCROLLBAR,
  TERMSURF_ACTION_RENDER,
  TERMSURF_ACTION_INSPECTOR,
  TERMSURF_ACTION_SHOW_GTK_INSPECTOR,
  TERMSURF_ACTION_RENDER_INSPECTOR,
  TERMSURF_ACTION_DESKTOP_NOTIFICATION,
  TERMSURF_ACTION_SET_TITLE,
  TERMSURF_ACTION_PROMPT_TITLE,
  TERMSURF_ACTION_PWD,
  TERMSURF_ACTION_MOUSE_SHAPE,
  TERMSURF_ACTION_MOUSE_VISIBILITY,
  TERMSURF_ACTION_MOUSE_OVER_LINK,
  TERMSURF_ACTION_RENDERER_HEALTH,
  TERMSURF_ACTION_OPEN_CONFIG,
  TERMSURF_ACTION_QUIT_TIMER,
  TERMSURF_ACTION_FLOAT_WINDOW,
  TERMSURF_ACTION_SECURE_INPUT,
  TERMSURF_ACTION_KEY_SEQUENCE,
  TERMSURF_ACTION_KEY_TABLE,
  TERMSURF_ACTION_COLOR_CHANGE,
  TERMSURF_ACTION_RELOAD_CONFIG,
  TERMSURF_ACTION_CONFIG_CHANGE,
  TERMSURF_ACTION_CLOSE_WINDOW,
  TERMSURF_ACTION_RING_BELL,
  TERMSURF_ACTION_UNDO,
  TERMSURF_ACTION_REDO,
  TERMSURF_ACTION_CHECK_FOR_UPDATES,
  TERMSURF_ACTION_OPEN_URL,
  TERMSURF_ACTION_SHOW_CHILD_EXITED,
  TERMSURF_ACTION_PROGRESS_REPORT,
  TERMSURF_ACTION_SHOW_ON_SCREEN_KEYBOARD,
  TERMSURF_ACTION_COMMAND_FINISHED,
  TERMSURF_ACTION_START_SEARCH,
  TERMSURF_ACTION_END_SEARCH,
  TERMSURF_ACTION_SEARCH_TOTAL,
  TERMSURF_ACTION_SEARCH_SELECTED,
  TERMSURF_ACTION_READONLY,
  TERMSURF_ACTION_COPY_TITLE_TO_CLIPBOARD,
} termsurf_action_tag_e;

typedef union {
  termsurf_action_split_direction_e new_split;
  termsurf_action_fullscreen_e toggle_fullscreen;
  termsurf_action_move_tab_s move_tab;
  termsurf_action_goto_tab_e goto_tab;
  termsurf_action_goto_split_e goto_split;
  termsurf_action_goto_window_e goto_window;
  termsurf_action_resize_split_s resize_split;
  termsurf_action_size_limit_s size_limit;
  termsurf_action_initial_size_s initial_size;
  termsurf_action_cell_size_s cell_size;
  termsurf_action_scrollbar_s scrollbar;
  termsurf_action_inspector_e inspector;
  termsurf_action_desktop_notification_s desktop_notification;
  termsurf_action_set_title_s set_title;
  termsurf_action_prompt_title_e prompt_title;
  termsurf_action_pwd_s pwd;
  termsurf_action_mouse_shape_e mouse_shape;
  termsurf_action_mouse_visibility_e mouse_visibility;
  termsurf_action_mouse_over_link_s mouse_over_link;
  termsurf_action_renderer_health_e renderer_health;
  termsurf_action_quit_timer_e quit_timer;
  termsurf_action_float_window_e float_window;
  termsurf_action_secure_input_e secure_input;
  termsurf_action_key_sequence_s key_sequence;
  termsurf_action_key_table_s key_table;
  termsurf_action_color_change_s color_change;
  termsurf_action_reload_config_s reload_config;
  termsurf_action_config_change_s config_change;
  termsurf_action_open_url_s open_url;
  termsurf_action_close_tab_mode_e close_tab_mode;
  termsurf_surface_message_childexited_s child_exited;
  termsurf_action_progress_report_s progress_report;
  termsurf_action_command_finished_s command_finished;
  termsurf_action_start_search_s start_search;
  termsurf_action_search_total_s search_total;
  termsurf_action_search_selected_s search_selected;
  termsurf_action_readonly_e readonly;
} termsurf_action_u;

typedef struct {
  termsurf_action_tag_e tag;
  termsurf_action_u action;
} termsurf_action_s;

typedef void (*termsurf_runtime_wakeup_cb)(void*);
typedef void (*termsurf_runtime_read_clipboard_cb)(void*,
                                                  termsurf_clipboard_e,
                                                  void*);
typedef void (*termsurf_runtime_confirm_read_clipboard_cb)(
    void*,
    const char*,
    void*,
    termsurf_clipboard_request_e);
typedef void (*termsurf_runtime_write_clipboard_cb)(void*,
                                                   termsurf_clipboard_e,
                                                   const termsurf_clipboard_content_s*,
                                                   size_t,
                                                   bool);
typedef void (*termsurf_runtime_close_surface_cb)(void*, bool);
typedef bool (*termsurf_runtime_action_cb)(termsurf_app_t,
                                          termsurf_target_s,
                                          termsurf_action_s);

typedef struct {
  void* userdata;
  bool supports_selection_clipboard;
  termsurf_runtime_wakeup_cb wakeup_cb;
  termsurf_runtime_action_cb action_cb;
  termsurf_runtime_read_clipboard_cb read_clipboard_cb;
  termsurf_runtime_confirm_read_clipboard_cb confirm_read_clipboard_cb;
  termsurf_runtime_write_clipboard_cb write_clipboard_cb;
  termsurf_runtime_close_surface_cb close_surface_cb;
} termsurf_runtime_config_s;

// apprt.ipc.Target.Key
typedef enum {
  TERMSURF_IPC_TARGET_CLASS,
  TERMSURF_IPC_TARGET_DETECT,
} termsurf_ipc_target_tag_e;

typedef union {
  char *klass;
} termsurf_ipc_target_u;

typedef struct {
  termsurf_ipc_target_tag_e tag;
  termsurf_ipc_target_u target;
} chostty_ipc_target_s;

// apprt.ipc.Action.NewWindow
typedef struct {
  // This should be a null terminated list of strings.
  const char **arguments;
} termsurf_ipc_action_new_window_s;

typedef union {
  termsurf_ipc_action_new_window_s new_window;
} termsurf_ipc_action_u;

// apprt.ipc.Action.Key
typedef enum {
  TERMSURF_IPC_ACTION_NEW_WINDOW,
} termsurf_ipc_action_tag_e;

//-------------------------------------------------------------------
// Published API

int termsurf_init(uintptr_t, char**);
void termsurf_cli_try_action(void);
termsurf_info_s termsurf_info(void);
const char* termsurf_translate(const char*);
void termsurf_string_free(termsurf_string_s);

termsurf_config_t termsurf_config_new();
void termsurf_config_free(termsurf_config_t);
termsurf_config_t termsurf_config_clone(termsurf_config_t);
void termsurf_config_load_cli_args(termsurf_config_t);
void termsurf_config_load_file(termsurf_config_t, const char*);
void termsurf_config_load_default_files(termsurf_config_t);
void termsurf_config_load_recursive_files(termsurf_config_t);
void termsurf_config_finalize(termsurf_config_t);
bool termsurf_config_get(termsurf_config_t, void*, const char*, uintptr_t);
termsurf_input_trigger_s termsurf_config_trigger(termsurf_config_t,
                                               const char*,
                                               uintptr_t);
uint32_t termsurf_config_diagnostics_count(termsurf_config_t);
termsurf_diagnostic_s termsurf_config_get_diagnostic(termsurf_config_t, uint32_t);
termsurf_string_s termsurf_config_open_path(void);

termsurf_app_t termsurf_app_new(const termsurf_runtime_config_s*,
                              termsurf_config_t);
void termsurf_app_free(termsurf_app_t);
void termsurf_app_tick(termsurf_app_t);
void* termsurf_app_userdata(termsurf_app_t);
void termsurf_app_set_focus(termsurf_app_t, bool);
bool termsurf_app_key(termsurf_app_t, termsurf_input_key_s);
bool termsurf_app_key_is_binding(termsurf_app_t, termsurf_input_key_s);
void termsurf_app_keyboard_changed(termsurf_app_t);
void termsurf_app_open_config(termsurf_app_t);
void termsurf_app_update_config(termsurf_app_t, termsurf_config_t);
bool termsurf_app_needs_confirm_quit(termsurf_app_t);
bool termsurf_app_has_global_keybinds(termsurf_app_t);
void termsurf_app_set_color_scheme(termsurf_app_t, termsurf_color_scheme_e);

termsurf_surface_config_s termsurf_surface_config_new();

termsurf_surface_t termsurf_surface_new(termsurf_app_t,
                                      const termsurf_surface_config_s*);
void termsurf_surface_free(termsurf_surface_t);
void* termsurf_surface_userdata(termsurf_surface_t);
termsurf_app_t termsurf_surface_app(termsurf_surface_t);
termsurf_surface_config_s termsurf_surface_inherited_config(termsurf_surface_t, termsurf_surface_context_e);
void termsurf_surface_update_config(termsurf_surface_t, termsurf_config_t);
bool termsurf_surface_needs_confirm_quit(termsurf_surface_t);
bool termsurf_surface_process_exited(termsurf_surface_t);
void termsurf_surface_refresh(termsurf_surface_t);
void termsurf_surface_draw(termsurf_surface_t);
void termsurf_surface_set_content_scale(termsurf_surface_t, double, double);
void termsurf_surface_set_focus(termsurf_surface_t, bool);
void termsurf_surface_pane_focus_changed(termsurf_surface_t, bool);
void termsurf_surface_set_occlusion(termsurf_surface_t, bool);
void termsurf_surface_set_size(termsurf_surface_t, uint32_t, uint32_t);
termsurf_surface_size_s termsurf_surface_size(termsurf_surface_t);
void termsurf_surface_set_color_scheme(termsurf_surface_t,
                                      termsurf_color_scheme_e);
termsurf_input_mods_e termsurf_surface_key_translation_mods(termsurf_surface_t,
                                                          termsurf_input_mods_e);
bool termsurf_surface_is_overlay_forwarding(termsurf_surface_t);
bool termsurf_surface_key(termsurf_surface_t, termsurf_input_key_s);
bool termsurf_surface_key_is_binding(termsurf_surface_t,
                                    termsurf_input_key_s,
                                    termsurf_binding_flags_e*);
void termsurf_surface_text(termsurf_surface_t, const char*, uintptr_t);
void termsurf_surface_preedit(termsurf_surface_t, const char*, uintptr_t);
bool termsurf_surface_mouse_captured(termsurf_surface_t);
bool termsurf_surface_mouse_button(termsurf_surface_t,
                                  termsurf_input_mouse_state_e,
                                  termsurf_input_mouse_button_e,
                                  termsurf_input_mods_e);
void termsurf_surface_mouse_pos(termsurf_surface_t,
                               double,
                               double,
                               termsurf_input_mods_e);
void termsurf_surface_mouse_scroll(termsurf_surface_t,
                                  double,
                                  double,
                                  termsurf_input_scroll_mods_t);
// TermSurf macOS-specific scroll API (Issue 606).
// Carries processed values (for terminal) and raw NSEvent values (for Chromium).
void termsurf_macos_surface_mouse_scroll(termsurf_surface_t,
                                         double, double,
                                         termsurf_input_scroll_mods_t,
                                         double, double,
                                         uint64_t, uint64_t);
void termsurf_surface_mouse_pressure(termsurf_surface_t, uint32_t, double);
void termsurf_surface_ime_point(termsurf_surface_t, double*, double*, double*, double*);
void termsurf_surface_request_close(termsurf_surface_t);
void termsurf_surface_split(termsurf_surface_t, termsurf_action_split_direction_e);
void termsurf_surface_split_with_input(termsurf_surface_t,
                                       termsurf_action_split_direction_e,
                                       const char*);
const char* termsurf_surface_get_pending_input(void);
void termsurf_surface_free_pending_input(const char*);
void termsurf_surface_split_focus(termsurf_surface_t,
                                 termsurf_action_goto_split_e);
void termsurf_surface_split_resize(termsurf_surface_t,
                                  termsurf_action_resize_split_direction_e,
                                  uint16_t);
void termsurf_surface_split_equalize(termsurf_surface_t);
bool termsurf_surface_binding_action(termsurf_surface_t, const char*, uintptr_t);
void termsurf_surface_complete_clipboard_request(termsurf_surface_t,
                                                const char*,
                                                void*,
                                                bool);
bool termsurf_surface_has_selection(termsurf_surface_t);
bool termsurf_surface_read_selection(termsurf_surface_t, termsurf_text_s*);
bool termsurf_surface_read_text(termsurf_surface_t,
                               termsurf_selection_s,
                               termsurf_text_s*);
void termsurf_surface_free_text(termsurf_surface_t, termsurf_text_s*);

#ifdef __APPLE__
void termsurf_surface_set_display_id(termsurf_surface_t, uint32_t);
void* termsurf_surface_quicklook_font(termsurf_surface_t);
bool termsurf_surface_quicklook_word(termsurf_surface_t, termsurf_text_s*);
#endif

termsurf_inspector_t termsurf_surface_inspector(termsurf_surface_t);
void termsurf_inspector_free(termsurf_surface_t);
void termsurf_inspector_set_focus(termsurf_inspector_t, bool);
void termsurf_inspector_set_content_scale(termsurf_inspector_t, double, double);
void termsurf_inspector_set_size(termsurf_inspector_t, uint32_t, uint32_t);
void termsurf_inspector_mouse_button(termsurf_inspector_t,
                                    termsurf_input_mouse_state_e,
                                    termsurf_input_mouse_button_e,
                                    termsurf_input_mods_e);
void termsurf_inspector_mouse_pos(termsurf_inspector_t, double, double);
void termsurf_inspector_mouse_scroll(termsurf_inspector_t,
                                    double,
                                    double,
                                    termsurf_input_scroll_mods_t);
void termsurf_inspector_key(termsurf_inspector_t,
                           termsurf_input_action_e,
                           termsurf_input_key_e,
                           termsurf_input_mods_e);
void termsurf_inspector_text(termsurf_inspector_t, const char*);

#ifdef __APPLE__
bool termsurf_inspector_metal_init(termsurf_inspector_t, void*);
void termsurf_inspector_metal_render(termsurf_inspector_t, void*, void*);
bool termsurf_inspector_metal_shutdown(termsurf_inspector_t);
#endif

// APIs I'd like to get rid of eventually but are still needed for now.
// Don't use these unless you know what you're doing.
void termsurf_set_window_background_blur(termsurf_app_t, void*);

// Benchmark API, if available.
bool termsurf_benchmark_cli(const char*, const char*);

#ifdef __cplusplus
}
#endif

#endif /* TERMSURF_H */
