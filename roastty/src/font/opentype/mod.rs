//! OpenType / SFNT table parsing.
//!
//! Faithful port of upstream `font/opentype/`. The face metric extraction
//! (`Face::getMetrics`) reads the raw `head`/`hhea`/`os2`/`post` table bytes
//! supplied by CoreText (`CTFontCopyTable`) and parses the OpenType tables
//! directly, so these parsers are the pure-Rust prerequisite for the font face
//! path.
//!
//! This slice ports the shared SFNT scalar types and the `head`, `hhea`,
//! `post`, and `os2` tables — the four tables `Face::getMetrics` reads. The
//! whole-file SFNT table-directory reader and the CoreText FFI land in later
//! experiments.

pub(crate) mod head;
pub(crate) mod hhea;
pub(crate) mod os2;
pub(crate) mod post;
pub(crate) mod sfnt;
pub(crate) mod svg;
