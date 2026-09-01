# osu-parse

osu! 文件解析共享库:`.osu` 谱面、`.osb` storyboard、`.osr` 回放,全部为
osu!lazer 解码器的逐行对齐移植。供 osu-replay-judge / osu-replay-render /
osu-storyboard-render 共用,消除此前散落各处的多套解析实现。

## 模块

| 模块 | 内容 |
|---|---|
| `beatmap` | `.osu` 全 section 解析(General/Editor/Metadata/Difficulty/TimingPoints/Colours/Events/HitObjects),`LegacyBeatmapDecoder` 语义:红绿点分离、AR 回落 OD、难度钳制、曲线字符串隐式分段 |
| `storyboard` | `.osb` / `.osu [Events]` 解析(parser/model)、33 种缓动(easing)、循环展开与通道采样(timeline)、`.osu` Events + 谱组共用 `.osb` 合并(loader) |
| `replay` | `.osr` 二进制解析(`LegacyScoreDecoder`):.NET 字符串/LZMA 帧/随机种子/stable 帧怪癖修正 |
| `samples` | 采样侧数据:逐 timing point 的 bank/音量、物件 hitSample/edgeSets 覆盖 |
| `process` | 解码后处理(HR/EZ 难度 mod、堆叠算法、嵌套物件生成) |
| `path` | SliderPath + PathApproximator(线性/完美圆/贝塞尔/B-spline/Catmull) |
| `vec2` / `mods` | f32 向量(osuTK 语义)、legacy mod 位标志 |

唯一依赖 `lzma-rs`(纯 Rust,回放解压)。

## 用法

```rust
use osu_parse::{beatmap, replay, storyboard};

let map = beatmap::decode(&std::fs::read_to_string("map.osu")?)?;
// map.metadata.title / map.general.audio_filename / map.background
// map.objects / map.timing_points / map.sample_data ...

let rep = replay::decode_file("replay.osr", map.version)?;

// storyboard:合并所选难度 Events 与同目录共享 .osb
let loaded = storyboard::loader::load_beatmap(std::path::Path::new("map.osu"), true);
```

判定引擎/HP/分数(osu-replay-judge)与 wgpu 渲染(osu-storyboard-render、
osu-replay-render)在各自仓库中,以本 crate 为解析底座。
