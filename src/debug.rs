// ------------------------------------------------------
// Debug
// ------------------------------------------------------

// #if !defined(MI_DEBUG_UNINIT)
const MI_DEBUG_UNINIT: usize = (0xD0);
// #endif
// #if !defined(MI_DEBUG_FREED)
const MI_DEBUG_FREED: usize = (0xDF);
// #endif
// #if !defined(MI_DEBUG_PADDING)
const MI_DEBUG_PADDING: usize = (0xDE);
// #endif

// #if (MI_DEBUG)
// use our own assertion to print without memory allocation
// void _mi_assert_fail(const char *assertion, const char *fname, unsigned int line, const char *func);
// #define mi_assert(expr) ((expr) ? (void)0 : _mi_assert_fail(#expr, __FILE__, __LINE__, __func__))
// #else
// #define mi_assert(x)
// #endif

// #if (MI_DEBUG > 1)
// #define mi_assert_internal mi_assert
// #else
// #define mi_assert_internal(x)
// #endif

// #if (MI_DEBUG > 2)
// #define mi_assert_expensive mi_assert
// #else
// #define mi_assert_expensive(x)
// #endif
