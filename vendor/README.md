# Vendored crates

`usvg/` is usvg 0.45.1 (MIT / Apache-2.0, https://github.com/linebender/resvg) with one change, in
`src/text/layout.rs` (`form_glyph_clusters`): a glyph cluster's advance is the sum of its
glyphs' advances, not the widest glyph's. With the original, a Devanagari cluster such as "मा"
(consonant + vowel sign) was too narrow and the next letter was drawn over the vowel sign, so
"समाप्त" came out as "समप्त" in PNG and PDF exports.

Drop the vendored copy (and `[patch.crates-io]` in `Cargo.toml`) once resvg/svg2pdf move to a
usvg release with the fix.
