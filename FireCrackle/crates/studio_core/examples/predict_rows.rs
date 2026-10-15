//! Read-only audit: predict_rows PROJECT.json LABELS.jsonl FRAME...
use studio_core::{
    config::Settings,
    label::LabelStore,
    project::Project,
    session::{Move, Session},
};
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert!(
        args.len() >= 3,
        "predict_rows PROJECT.json LABELS.jsonl FRAME..."
    );
    let project = Project::load(std::path::Path::new(&args[0])).expect("project");
    let text = std::fs::read_to_string(&args[1]).expect("label log");
    let store = LabelStore::replay(&text, project.frames.len() as u32)
        .expect("labels")
        .store;
    let mut session = Session::new(project, store, Settings::default()).expect("session");
    for n in &args[2..] {
        let n: u32 = n.parse().expect("frame index");
        session.go(Move::To(n));
        println!(
            "{}",
            serde_json::json!({"frame":n,"pts":session.project().frames[session.current() as usize].pts,"suggestion":session.suggestion()})
        );
    }
}
