//! 临时诊断:列出 storyboard 在指定时刻可见的元素及其状态。
//! usage: sbat <map.osu 或 .osb> <ms> [...]
use osu_parse::storyboard::{loader, timeline};

fn main() {
    let path = std::env::args().nth(1).expect("usage: sbat <file> <ms>...");
    let loaded = loader::load_beatmap(std::path::Path::new(&path), false).expect("load");
    let sb = loaded.story;
    eprintln!("elements={} videos={} warnings={}", sb.elements.len(), sb.videos.len(), sb.warnings.len());
    let compiled = timeline::CompiledStoryboard::compile(sb);
    eprintln!("compiled elements={} total_commands={}", compiled.elements.len(), compiled.total_commands);
    for ms in std::env::args().skip(2) {
        let t: f32 = ms.parse().expect("ms");
        println!("--- t={t}ms ---");
        let mut n = 0;
        for (idx, el) in compiled.elements.iter().enumerate() {
            if let Some(st) = el.state_at(t) {
                if st.alpha <= 0.001 { continue; }
                n += 1;
                println!(
                    "{:>3} [{}] {:?} {:?} pos=({:.0},{:.0}) scale=({:.2},{:.2}) rot={:.2} alpha={:.2} add={} colour=({:.2},{:.2},{:.2})",
                    n, idx, el.layer, el.path, st.x, st.y, st.scale_x, st.scale_y, st.rotation, st.alpha, st.additive,
                    st.colour[0], st.colour[1], st.colour[2]
                );
            }
        }
        println!("visible: {n}");
    }
}
