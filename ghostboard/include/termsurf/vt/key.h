/**
 * @file key.h
 *
 * Key encoding module - encode key events into terminal escape sequences.
 */

#ifndef TERMSURF_VT_KEY_H
#define TERMSURF_VT_KEY_H

/** @defgroup key Key Encoding
 *
 * Utilities for encoding key events into terminal escape sequences,
 * supporting both legacy encoding as well as Kitty Keyboard Protocol.
 *
 * ## Basic Usage
 *
 * 1. Create an encoder instance with termsurf_key_encoder_new()
 * 2. Configure encoder options with termsurf_key_encoder_setopt().
 * 3. For each key event:
 *    - Create a key event with termsurf_key_event_new()
 *    - Set event properties (action, key, modifiers, etc.)
 *    - Encode with termsurf_key_encoder_encode()
 *    - Free the event with termsurf_key_event_free()
 *    - Note: You can also reuse the same key event multiple times by
 *      changing its properties.
 * 4. Free the encoder with termsurf_key_encoder_free() when done
 *
 * ## Example
 *
 * @code{.c}
 * #include <assert.h>
 * #include <stdio.h>
 * #include <termsurf/vt.h>
 * 
 * int main() {
 *   // Create encoder
 *   TermSurfKeyEncoder encoder;
 *   TermSurfResult result = termsurf_key_encoder_new(NULL, &encoder);
 *   assert(result == TERMSURF_SUCCESS);
 * 
 *   // Enable Kitty keyboard protocol with all features
 *   termsurf_key_encoder_setopt(encoder, TERMSURF_KEY_ENCODER_OPT_KITTY_FLAGS, 
 *                              &(uint8_t){TERMSURF_KITTY_KEY_ALL});
 * 
 *   // Create and configure key event for Ctrl+C press
 *   TermSurfKeyEvent event;
 *   result = termsurf_key_event_new(NULL, &event);
 *   assert(result == TERMSURF_SUCCESS);
 *   termsurf_key_event_set_action(event, TERMSURF_KEY_ACTION_PRESS);
 *   termsurf_key_event_set_key(event, TERMSURF_KEY_C);
 *   termsurf_key_event_set_mods(event, TERMSURF_MODS_CTRL);
 * 
 *   // Encode the key event
 *   char buf[128];
 *   size_t written = 0;
 *   result = termsurf_key_encoder_encode(encoder, event, buf, sizeof(buf), &written);
 *   assert(result == TERMSURF_SUCCESS);
 * 
 *   // Use the encoded sequence (e.g., write to terminal)
 *   fwrite(buf, 1, written, stdout);
 * 
 *   // Cleanup
 *   termsurf_key_event_free(event);
 *   termsurf_key_encoder_free(encoder);
 *   return 0;
 * }
 * @endcode
 *
 * For a complete working example, see example/c-vt-key-encode in the
 * repository.
 *
 * @{
 */

#include <termsurf/vt/key/event.h>
#include <termsurf/vt/key/encoder.h>

/** @} */

#endif /* TERMSURF_VT_KEY_H */
