/**
 * @file result.h
 *
 * Result codes for libtermsurf-vt operations.
 */

#ifndef TERMSURF_VT_RESULT_H
#define TERMSURF_VT_RESULT_H

/**
 * Result codes for libtermsurf-vt operations.
 */
typedef enum {
    /** Operation completed successfully */
    TERMSURF_SUCCESS = 0,
    /** Operation failed due to failed allocation */
    TERMSURF_OUT_OF_MEMORY = -1,
    /** Operation failed due to invalid value */
    TERMSURF_INVALID_VALUE = -2,
} TermSurfResult;

#endif /* TERMSURF_VT_RESULT_H */
