/**
 * @file event.h
 *
 * Key event representation and manipulation.
 */

#ifndef TERMSURF_VT_KEY_EVENT_H
#define TERMSURF_VT_KEY_EVENT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <termsurf/vt/result.h>
#include <termsurf/vt/allocator.h>

/**
 * Opaque handle to a key event.
 * 
 * This handle represents a keyboard input event containing information about
 * the physical key pressed, modifiers, and generated text.
 *
 * @ingroup key
 */
typedef struct TermSurfKeyEvent *TermSurfKeyEvent;

/**
 * Keyboard input event types.
 *
 * @ingroup key
 */
typedef enum {
    /** Key was released */
    TERMSURF_KEY_ACTION_RELEASE = 0,
    /** Key was pressed */
    TERMSURF_KEY_ACTION_PRESS = 1,
    /** Key is being repeated (held down) */
    TERMSURF_KEY_ACTION_REPEAT = 2,
} TermSurfKeyAction;

/**
 * Keyboard modifier keys bitmask.
 *
 * A bitmask representing all keyboard modifiers. This tracks which modifier keys 
 * are pressed and, where supported by the platform, which side (left or right) 
 * of each modifier is active.
 *
 * Use the TERMSURF_MODS_* constants to test and set individual modifiers.
 *
 * Modifier side bits are only meaningful when the corresponding modifier bit is set.
 * Not all platforms support distinguishing between left and right modifier 
 * keys and TermSurf is built to expect that some platforms may not provide this
 * information.
 *
 * @ingroup key
 */
typedef uint16_t TermSurfMods;

/** Shift key is pressed */
#define TERMSURF_MODS_SHIFT (1 << 0)
/** Control key is pressed */
#define TERMSURF_MODS_CTRL (1 << 1)
/** Alt/Option key is pressed */
#define TERMSURF_MODS_ALT (1 << 2)
/** Super/Command/Windows key is pressed */
#define TERMSURF_MODS_SUPER (1 << 3)
/** Caps Lock is active */
#define TERMSURF_MODS_CAPS_LOCK (1 << 4)
/** Num Lock is active */
#define TERMSURF_MODS_NUM_LOCK (1 << 5)

/**
 * Right shift is pressed (0 = left, 1 = right).
 * Only meaningful when TERMSURF_MODS_SHIFT is set.
 */
#define TERMSURF_MODS_SHIFT_SIDE (1 << 6)
/**
 * Right ctrl is pressed (0 = left, 1 = right).
 * Only meaningful when TERMSURF_MODS_CTRL is set.
 */
#define TERMSURF_MODS_CTRL_SIDE (1 << 7)
/**
 * Right alt is pressed (0 = left, 1 = right).
 * Only meaningful when TERMSURF_MODS_ALT is set.
 */
#define TERMSURF_MODS_ALT_SIDE (1 << 8)
/**
 * Right super is pressed (0 = left, 1 = right).
 * Only meaningful when TERMSURF_MODS_SUPER is set.
 */
#define TERMSURF_MODS_SUPER_SIDE (1 << 9)

/**
 * Physical key codes.
 *
 * The set of key codes that TermSurf is aware of. These represent physical keys 
 * on the keyboard and are layout-independent. For example, the "a" key on a US 
 * keyboard is the same as the "ф" key on a Russian keyboard, but both will 
 * report the same key_a value.
 *
 * Layout-dependent strings are provided separately as UTF-8 text and are produced 
 * by the platform. These values are based on the W3C UI Events KeyboardEvent code 
 * standard. See: https://www.w3.org/TR/uievents-code
 *
 * @ingroup key
 */
typedef enum {
    TERMSURF_KEY_UNIDENTIFIED = 0,

    // Writing System Keys (W3C § 3.1.1)
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

    // Functional Keys (W3C § 3.1.2)
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

    // Control Pad Section (W3C § 3.2)
    TERMSURF_KEY_DELETE,
    TERMSURF_KEY_END,
    TERMSURF_KEY_HELP,
    TERMSURF_KEY_HOME,
    TERMSURF_KEY_INSERT,
    TERMSURF_KEY_PAGE_DOWN,
    TERMSURF_KEY_PAGE_UP,

    // Arrow Pad Section (W3C § 3.3)
    TERMSURF_KEY_ARROW_DOWN,
    TERMSURF_KEY_ARROW_LEFT,
    TERMSURF_KEY_ARROW_RIGHT,
    TERMSURF_KEY_ARROW_UP,

    // Numpad Section (W3C § 3.4)
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

    // Function Section (W3C § 3.5)
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

    // Media Keys (W3C § 3.6)
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

    // Legacy, Non-standard, and Special Keys (W3C § 3.7)
    TERMSURF_KEY_COPY,
    TERMSURF_KEY_CUT,
    TERMSURF_KEY_PASTE,
} TermSurfKey;

/**
 * Create a new key event instance.
 * 
 * Creates a new key event with default values. The event must be freed using
 * termsurf_key_event_free() when no longer needed.
 * 
 * @param allocator Pointer to the allocator to use for memory management, or NULL to use the default allocator
 * @param event Pointer to store the created key event handle
 * @return TERMSURF_SUCCESS on success, or an error code on failure
 * 
 * @ingroup key
 */
TermSurfResult termsurf_key_event_new(const TermSurfAllocator *allocator, TermSurfKeyEvent *event);

/**
 * Free a key event instance.
 * 
 * Releases all resources associated with the key event. After this call,
 * the event handle becomes invalid and must not be used.
 * 
 * @param event The key event handle to free (may be NULL)
 * 
 * @ingroup key
 */
void termsurf_key_event_free(TermSurfKeyEvent event);

/**
 * Set the key action (press, release, repeat).
 *
 * @param event The key event handle, must not be NULL
 * @param action The action to set
 *
 * @ingroup key
 */
void termsurf_key_event_set_action(TermSurfKeyEvent event, TermSurfKeyAction action);

/**
 * Get the key action (press, release, repeat).
 *
 * @param event The key event handle, must not be NULL
 * @return The key action
 *
 * @ingroup key
 */
TermSurfKeyAction termsurf_key_event_get_action(TermSurfKeyEvent event);

/**
 * Set the physical key code.
 *
 * @param event The key event handle, must not be NULL
 * @param key The physical key code to set
 *
 * @ingroup key
 */
void termsurf_key_event_set_key(TermSurfKeyEvent event, TermSurfKey key);

/**
 * Get the physical key code.
 *
 * @param event The key event handle, must not be NULL
 * @return The physical key code
 *
 * @ingroup key
 */
TermSurfKey termsurf_key_event_get_key(TermSurfKeyEvent event);

/**
 * Set the modifier keys bitmask.
 *
 * @param event The key event handle, must not be NULL
 * @param mods The modifier keys bitmask to set
 *
 * @ingroup key
 */
void termsurf_key_event_set_mods(TermSurfKeyEvent event, TermSurfMods mods);

/**
 * Get the modifier keys bitmask.
 *
 * @param event The key event handle, must not be NULL
 * @return The modifier keys bitmask
 *
 * @ingroup key
 */
TermSurfMods termsurf_key_event_get_mods(TermSurfKeyEvent event);

/**
 * Set the consumed modifiers bitmask.
 *
 * @param event The key event handle, must not be NULL
 * @param consumed_mods The consumed modifiers bitmask to set
 *
 * @ingroup key
 */
void termsurf_key_event_set_consumed_mods(TermSurfKeyEvent event, TermSurfMods consumed_mods);

/**
 * Get the consumed modifiers bitmask.
 *
 * @param event The key event handle, must not be NULL
 * @return The consumed modifiers bitmask
 *
 * @ingroup key
 */
TermSurfMods termsurf_key_event_get_consumed_mods(TermSurfKeyEvent event);

/**
 * Set whether the key event is part of a composition sequence.
 *
 * @param event The key event handle, must not be NULL
 * @param composing Whether the key event is part of a composition sequence
 *
 * @ingroup key
 */
void termsurf_key_event_set_composing(TermSurfKeyEvent event, bool composing);

/**
 * Get whether the key event is part of a composition sequence.
 *
 * @param event The key event handle, must not be NULL
 * @return Whether the key event is part of a composition sequence
 *
 * @ingroup key
 */
bool termsurf_key_event_get_composing(TermSurfKeyEvent event);

/**
 * Set the UTF-8 text generated by the key event.
 *
 * The key event does NOT take ownership of the text pointer. The caller
 * must ensure the string remains valid for the lifetime needed by the event.
 *
 * @param event The key event handle, must not be NULL
 * @param utf8 The UTF-8 text to set (or NULL for empty)
 * @param len Length of the UTF-8 text in bytes
 *
 * @ingroup key
 */
void termsurf_key_event_set_utf8(TermSurfKeyEvent event, const char *utf8, size_t len);

/**
 * Get the UTF-8 text generated by the key event.
 *
 * The returned pointer is valid until the event is freed or the UTF-8 text is modified.
 *
 * @param event The key event handle, must not be NULL
 * @param len Pointer to store the length of the UTF-8 text in bytes (may be NULL)
 * @return The UTF-8 text (or NULL for empty)
 *
 * @ingroup key
 */
const char *termsurf_key_event_get_utf8(TermSurfKeyEvent event, size_t *len);

/**
 * Set the unshifted Unicode codepoint.
 *
 * @param event The key event handle, must not be NULL
 * @param codepoint The unshifted Unicode codepoint to set
 *
 * @ingroup key
 */
void termsurf_key_event_set_unshifted_codepoint(TermSurfKeyEvent event, uint32_t codepoint);

/**
 * Get the unshifted Unicode codepoint.
 *
 * @param event The key event handle, must not be NULL
 * @return The unshifted Unicode codepoint
 *
 * @ingroup key
 */
uint32_t termsurf_key_event_get_unshifted_codepoint(TermSurfKeyEvent event);

#endif /* TERMSURF_VT_KEY_EVENT_H */
