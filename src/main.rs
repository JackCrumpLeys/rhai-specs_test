mod systems;
mod specs_rhai_magic;
mod tests;

use crate::systems::*;

use rhai::{Engine, EvalAltResult, Scope, AST};
use std::collections::HashMap;
use std::path::PathBuf;
use std::thread::sleep;
use std::time;
use std::time::Instant;

use specs::prelude::*;
use specs::AccessorCow;
use specs::shred::{CastFrom, DynamicSystemData, MetaTable};
use specs::shred::cell::{Ref, RefMut};
use crate::specs_rhai_magic::{create_script_sys, Reflection, ReflectionTable, ResourceTable};

fn main() {

    /// Some resource
    #[derive(Debug, Default)]
    struct Foo{
        int:i32
    };

    impl Reflection for Foo {
        fn call_method(&self, s: &str) {
            match s {
                "foo" => println!("Hello from Foo"),
                "bar" => println!("You gotta ask somebody else"),
                _ => panic!("The error handling of this example is non-ideal"),
            }
        }

        fn mut_call_method(&mut self, s: &str) {
            self.int +=1;
            println!("{} {}", self.int, s)
        }
    }

    /// Another resource
    #[derive(Debug, Default)]
    struct Bar;

    impl Reflection for Bar {
        fn call_method(&self, s: &str) {
            match s {
                "bar" => println!("Hello from Bar"),
                "foo" => println!("You gotta ask somebody else"),
                _ => panic!("The error handling of this example is non-ideal"),
            }
        }
    }

    struct Yar{
        i:i32
    }

    struct NormalSys;

    impl<'a> System<'a> for NormalSys {
        type SystemData = (Read<'a, Foo>, Read<'a, Bar>);

        fn run(&mut self, (foo, bar): Self::SystemData) {
            println!("Fetched foo: {:?}", &foo as &Foo);
            println!("Fetched bar: {:?}", &bar as &Bar);
        }
    }

    let mut res = World::empty();


    {
        let mut table = res.entry().or_insert_with(|| ReflectionTable::new());

        table.register(&Foo { int: 1 });
        table.register(&Bar);
    }

    {
        let mut table = res.entry().or_insert_with(|| ResourceTable::new());
        table.register::<Foo>("Foo");
        table.register::<Bar>("Bar");
    }

    let mut dispatcher = DispatcherBuilder::new()
        .with(NormalSys, "normal", &[])
        .build();
    dispatcher.setup(&mut res);

    let script0 = create_script_sys(&res);

    // it is recommended you create a second dispatcher dedicated to scripts,
    // that'll allow you to rebuild if necessary
    let mut scripts = DispatcherBuilder::new()
        .with(script0, "script0", &[])
        .build();
    scripts.setup(&mut res);

    // Game loop
    let mut i:i32 = 0;
    loop {
        // dispatcher.dispatch(&res);
        scripts.dispatch(&res);
        i += 1;

        if i == 10{
            break;
        }
    }
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

#[derive(Clone, Debug)]
struct Script {
    name: String,
    script_ast: AST,
    scope: Scope<'static>,
    last_run: Instant,
}

impl Reflection for Script {
    fn call_method(&self, s: &str) {
        println!("pp")
    }
}
