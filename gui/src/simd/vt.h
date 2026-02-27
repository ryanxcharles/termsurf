#if defined(TERMSURF_SIMD_VT_H_) == defined(HWY_TARGET_TOGGLE)
#ifdef TERMSURF_SIMD_VT_H_
#undef TERMSURF_SIMD_VT_H_
#else
#define TERMSURF_SIMD_VT_H_
#endif

#include <hwy/highway.h>

HWY_BEFORE_NAMESPACE();
namespace termsurf {
namespace HWY_NAMESPACE {

namespace hn = hwy::HWY_NAMESPACE;

}  // namespace HWY_NAMESPACE
}  // namespace termsurf
HWY_AFTER_NAMESPACE();

#if HWY_ONCE

namespace termsurf {

typedef void (*PrintFunc)(const char32_t* chars, size_t count);

}  // namespace termsurf

#endif  // HWY_ONCE

#endif  // TERMSURF_SIMD_VT_H_
