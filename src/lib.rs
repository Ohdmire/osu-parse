//! Shared osu! file parsing for the replay tools:
//!
//! - [`beatmap`] — `.osu` text parsing (all sections) + the raw decode
//!   structures, a line-accurate port of lazer's `LegacyBeatmapDecoder`;
//! - [`storyboard`] — `.osb` / `.osu [Events]` storyboard parsing
//!   (parser/model/easing/timeline) plus the `.osu` + shared `.osb`
//!   merge loader;
//! - [`replay`] — `.osr` binary parsing (port of `LegacyScoreDecoder`);
//! - [`process`] — post-decode lazer processing pipeline (difficulty
//!   mods, stacking, nested object creation);
//! - [`samples`] — `.osu` sample-side data (banks/volumes/hit samples);
//! - [`path`], [`vec2`], [`mods`] — slider path geometry, f32 vector
//!   math and legacy mod bitflags.
//!
//! Everything here is pure CPU parsing with no rendering or judgement
//! dependencies; consumers (osu-replay-judge, osu-replay-render,
//! osu-storyboard-render) build on top of these types.

pub mod beatmap;
pub mod mods;
pub mod path;
pub mod process;
pub mod replay;
pub mod samples;
pub mod storyboard;
pub mod vec2;
