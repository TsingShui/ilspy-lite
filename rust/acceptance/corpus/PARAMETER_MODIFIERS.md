# Optional custom modifiers on parameters and returns

A parameter or return type can carry `modopt` custom modifiers (attribute-like
annotations such as `[CallConvCdecl]`) or `modreq` modifiers, which are part of the
type contract. The signature decoders rejected any modifier, so a method whose
return type carried a `modopt` was refused with
`byref/modified parameter needs IL fallback` (658 first blockers, 39 recovered).

`Ty::from_param_scoped` and the constructed/value/member parameter decoders now
refuse only **required** modifiers; optional ones do not change the stack category,
byref-ness or identity of the parameter, and are ignored. The existing tests for
required modifiers and byref parameters are unchanged and still pass.

Evidence: the fixed-corpus scan moved by **+39 in 2 images** with zero regressions
(`report/modopt/corpus-scan`), 490 Rust tests, fmt and clippy pass. No owned C#
fixture can spell a `modopt`, so the before/after is the corpus scan itself rather
than a hand-built mutation; the widening is limited to modifiers the CLI defines as
optional.
