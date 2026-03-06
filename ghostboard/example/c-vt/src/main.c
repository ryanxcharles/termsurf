#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <termsurf/vt.h>

int main() {
  TermSurfOscParser parser;
  if (termsurf_osc_new(NULL, &parser) != TERMSURF_SUCCESS) {
    return 1;
  }
  
  // Setup change window title command to change the title to "hello"
  termsurf_osc_next(parser, '0');
  termsurf_osc_next(parser, ';');
  const char *title = "hello";
  for (size_t i = 0; i < strlen(title); i++) {
    termsurf_osc_next(parser, title[i]);
  }
  
  // End parsing and get command
  TermSurfOscCommand command = termsurf_osc_end(parser, 0);
  
  // Get and print command type
  TermSurfOscCommandType type = termsurf_osc_command_type(command);
  printf("Command type: %d\n", type);
  
  // Extract and print the title
  if (termsurf_osc_command_data(command, TERMSURF_OSC_DATA_CHANGE_WINDOW_TITLE_STR, &title)) {
    printf("Extracted title: %s\n", title);
  } else {
    printf("Failed to extract title\n");
  }
  
  termsurf_osc_free(parser);
  return 0;
}
