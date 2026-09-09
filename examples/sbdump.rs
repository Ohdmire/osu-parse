//! 临时诊断:列出 t 时刻可见元素(编译序),并反查原始元素打印完整命令。
use osu_parse::storyboard::{loader, timeline, model::Layer};

fn main() {
    let path = std::env::args().nth(1).expect("map path");
    let ms: f32 = std::env::args().nth(2).expect("ms").parse().unwrap();
    let only: Option<usize> = std::env::args().nth(3).and_then(|s| s.parse().ok());
    let mut loaded = loader::load_beatmap(std::path::Path::new(&path), false).unwrap();
    let raw_snapshot = loaded.story.elements.clone();
    let mut order: Vec<usize> = (0..loaded.story.elements.len()).collect();
    order.sort_by_key(|&i| loaded.story.elements[i].sprite().layer as u8);
    let compiled = timeline::CompiledStoryboard::compile(std::mem::take(&mut loaded.story));
    let raw = raw_snapshot;
    for (ci, el) in compiled.elements.iter().enumerate() {
        let Some(st) = el.state_at(ms) else { continue };
        if st.alpha <= 0.001 { continue; }
        if let Some(want) = only { if ci != want { continue; } }
        println!(
            "[compiled {ci}] raw[{}] {:?} pos=({:.0},{:.0}) scale=({:.2},{:.2}) rot={:.2} alpha={:.2} add={} colour=({:.2},{:.2},{:.2})",
            order[ci], el.path, st.x, st.y, st.scale_x, st.scale_y, st.rotation, st.alpha, st.additive,
            st.colour[0], st.colour[1], st.colour[2]
        );
        if only.is_some() {
            let s = raw[order[ci]].sprite();
            println!("    origin={:?} decl_pos=({},{})", s.origin, s.x, s.y);
            for c in &s.commands { println!("    top {c:?}"); }
            for l in &s.loops {
                println!("    L,{},{},{} 条", l.start_time, l.total_iterations, l.commands.len());
                for c in &l.commands { println!("      {c:?}"); }
            }
        }
    }
    let _ = Layer::Background;
}
