//! osu! storyboard 源文件（.osb / .osu Events 节）解析与时间轴求值，
//! 以及 .osu Events + 谱组共用 .osb 的合并加载器。

pub mod easing;
pub mod loader;
pub mod model;
pub mod parser;
pub mod timeline;
