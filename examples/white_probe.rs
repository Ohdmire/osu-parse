//! 诊断:编译 storyboard,打印白色精灵在各时刻的 alpha(求值层隔离)。
use osu_parse::storyboard::timeline::CompiledStoryboard;
use osu_parse::storyboard::loader::load_from_texts;

fn main() {
    let osu = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let osb = std::env::args().nth(2).and_then(|p| std::fs::read_to_string(p).ok());
    let sb = load_from_texts(&osu, osb.as_deref(), true).expect("parse");
    let c = CompiledStoryboard::compile(sb);
    for e in &c.elements {
        if e.path.to_lowercase().contains("white") {
            println!("== {} (start {} end {})", e.path, e.start, e.end);
            for t in (160000..=162000).step_by(50) {
                if let Some(st) = e.state_at(t as f32) {
                    println!("t={t} alpha={:.4} additive={}", st.alpha, st.additive);
                }
            }
        }
    }
}
