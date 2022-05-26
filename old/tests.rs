use std::time;
use rhai::Engine;
use crate::{load_script, tick};

#[test]
fn test_basic_script_functionality() {
    let engine:Engine = Engine::new();

    let mut scripts = Vec::new();
    scripts.push(load_script(
        r#"C:\Users\jackc\CLionProjects\rhai-specs_test\scripts\test.rhai"#
            .parse()
            .unwrap(),
        &engine
    ));

    let mut second = time::Instant::now();
    let mut i: i32 = 0;
    loop {
        i += 1;
        tick(&mut scripts, &engine);
        if second.elapsed().as_secs() == 1 {
            break;
        }
    }
    let mut script = scripts[0].clone();
    let mut add:i32 = engine.call_fn(&mut script.scope, &script.script_ast, "add", (10, 5)).unwrap();
    assert_eq!(add, 15)
}