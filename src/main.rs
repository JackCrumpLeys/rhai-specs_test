use rhai::{Engine, EvalAltResult, Scope, AST};
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread::sleep;
use std::time;
use std::time::Instant;

fn main() {
    test_script()
}

fn load_script(path: PathBuf, engine: &Engine) -> Script {
    let ast: AST = engine.compile_file(path.clone()).unwrap();

    let mut scope = Scope::new();
    engine.run_ast_with_scope(&mut scope, &ast);

    let _result: () = engine.call_fn(&mut scope, &ast, "load", ()).unwrap();
    let mut scripts: Script = Script {
        name: path.file_name().unwrap().to_str().unwrap().to_string(),
        script_ast: ast,
        scope,
        last_run: Instant::now(),
    };
    scripts
}

fn tick(scripts: &mut Vec<Script>, engine: &Engine) {
    for script in scripts {
        let new_last_run = Instant::now();
        let _result: () = engine
            .call_fn(
                &mut script.scope,
                &script.script_ast,
                "update",
                (script.last_run.elapsed().as_secs_f64() as f64,),
            )
            .unwrap();
        script.last_run = new_last_run
    }

}

struct Script {
    name: String,
    script_ast: AST,
    scope: Scope<'static>,
    last_run: Instant,
}

fn configure_engine(engine: &mut Engine) {
    engine.set_max_call_levels(100);
}

fn test_script() {
    let mut engine = Engine::new();

    configure_engine(&mut engine);

    let mut scripts = Vec::new();
    scripts.push(load_script(
        r#"C:\Users\jackc\CLionProjects\rhai-specs_test\scripts\test.rhai"#
            .parse()
            .unwrap(),
        &engine,
    ));

    let mut second = time::Instant::now();
    let mut i: i32 = 0;
    loop {
        i += 1;
        tick(&mut scripts, &engine);
        if second.elapsed().as_secs() == 1 {
            println!("{}", i);
            println!("{}", scripts[0].scope.get_value::<f64>("all").unwrap());
            assert!(scripts[0].scope.get_value::<f64>("all").unwrap());
            break;
        }
    }
}
