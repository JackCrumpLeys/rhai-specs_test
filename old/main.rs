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

use specs::Component;

use specs::prelude::*;
use specs::AccessorCow;
use specs::shred::{CastFrom, DynamicSystemData, Fetch, FetchMut, MetaTable};
use specs::shred::cell::{Ref, RefMut};
use crate::specs_rhai_magic::{create_script_sys, Reflection, ReflectionTable, ResourceTable};

pub struct component_reflect

// -- Step 1 - Define your resource type and an interface for registering it --

#[derive(Debug, Component)]
#[storage(VecStorage)]
pub struct ScriptableResource {
    fields: HashMap<String, String>,
}

#[derive(Debug)]
pub struct ScriptingInterface {
    id_alloc: u64,
    type_map: HashMap<String, u64>,
}

/// holds named Scripting resources.
impl ScriptingInterface {

    /// create a new ScriptingInterface
    pub fn new() -> Self {
        ScriptingInterface {
            id_alloc: 1, /* Start with `1` so systems don't fetch it accidentally (via
                          * `Fetch<ScriptingResource>`) */
            type_map: HashMap::new(),
        }
    }

    /// Registers a run-time resource as `name` and adds it to `world`.
    pub fn add_rt_resource(&mut self, name: &str, res: ScriptableResource, world: &mut World) {
        self.type_map.insert(name.to_owned(), self.id_alloc);
        self.id_alloc += 1;

        let id = self.resource_id(name).unwrap();
        world.insert_by_id(id, res);
    }

    /// remove a named resource from the world.
    pub fn remove_rt_resource(
        &mut self,
        name: &str,
        world: &mut World,
    ) -> Option<ScriptableResource> {
        let id = self.type_map.remove(name);

        id.and_then(|id| {
            world.remove_by_id(ResourceId::new_with_dynamic_id::<ScriptableResource>(id))
        })
    }

    /// flush all resources from the world.
    pub fn clear_rt_resources(&mut self, world: &mut World) {
        for &dynamic_id in self.type_map.values() {
            world.remove_by_id::<ScriptableResource>(ResourceId::new_with_dynamic_id::<
                ScriptableResource,
            >(dynamic_id));
        }

        self.type_map.clear();
        self.id_alloc = 1;
    }

    /// Returns the resource ID for the dynamic type identified by `name`
    pub fn resource_id(&self, name: &str) -> Option<ResourceId> {
        self.type_map
            .get(name)
            .cloned()
            .map(ResourceId::new_with_dynamic_id::<ScriptableResource>)
    }
}

// -- Step 2 - Setup the World --

/// setup the world,  put  your resources here.
fn setup_world() -> World {
    let mut world = WorldExt::new();

    let mut interface = ScriptingInterface::new();

    interface.add_rt_resource(
        "Foo",
        ScriptableResource {
            fields: vec![("foo_field".to_owned(), "5".to_owned())]
                .into_iter()
                .collect(),
        },
        &mut world,
    );

    // Make it accessible via the world
    world.insert(interface);

    world
}

// -- Step 3 - Preparations for fetching `ScriptingResource` from systems --

/// accessor for scripting resources
// TODO: implement writes
pub struct ScriptingResAccessor {
    reads: Vec<ResourceId>,
    // could also add `writes` here
}

impl ScriptingResAccessor {
    pub fn new(reads: &[&str], world: &World) -> Self {
        let interface = world.fetch::<ScriptingInterface>();

        ScriptingResAccessor {
            reads: reads
                .into_iter()
                .flat_map(|&name| interface.resource_id(name))
                .collect(),
        }
    }
}

impl Accessor for ScriptingResAccessor {
    fn try_new() -> Option<Self> {
        None
    }

    /// fetch reads
    fn reads(&self) -> Vec<ResourceId> {
        self.reads.clone()
    }

    // TODO: implement writes
    /// fetch writes
    fn writes(&self) -> Vec<ResourceId> {
        vec![]
    }
}

pub struct ScriptingResData<'a> {
    reads: Vec<Fetch<'a, ScriptableResource>>,
}

impl<'a> DynamicSystemData<'a> for ScriptingResData<'a> {
    type Accessor = ScriptingResAccessor;

    fn setup(_accessor: &Self::Accessor, _world: &mut World) {}

    fn fetch(access: &ScriptingResAccessor, world: &'a World) -> Self {
        ScriptingResData {
            reads: access
                .reads
                .iter()
                .map(|id| {
                    world
                        .try_fetch_by_id(id.clone())
                        .expect("Resource no longer exists")
                })
                .collect(),
        }
    }
}

// -- Step 4 - Actually defining a system --

struct MySys {
    accessor: ScriptingResAccessor,
}

impl<'a> System<'a> for MySys {
    type SystemData = ScriptingResData<'a>;

    fn run(&mut self, data: Self::SystemData) {
        for scripting_resource in data.reads {
            println!(
                "Fields of run-time resource: {:?}",
                scripting_resource.fields
            );
        }
    }

    fn accessor<'b>(&'b self) -> AccessorCow<'a, 'b, Self> {
        AccessorCow::Ref(&self.accessor)
    }
}

// -- Step 5 - Putting things together --

fn main() {
    let world = setup_world();

    let mut my_system = MySys {
        accessor: ScriptingResAccessor::new(&["Foo"], &world),
    };

    my_system.run_now(&world);
}



// fn main() {
//
//     /// Some resource
//     #[derive(Debug, Default)]
//     struct Foo{
//         int:i32
//     };
//
//     impl Reflection for Foo {
//         fn call_method(&self, s: &str) {
//             match s {
//                 "foo" => println!("Hello from Foo"),
//                 "bar" => println!("You gotta ask somebody else"),
//                 _ => panic!("The error handling of this example is non-ideal"),
//             }
//         }
//
//         fn mut_call_method(&mut self, s: &str) {
//             self.int +=1;
//             println!("{} {}", self.int, s)
//         }
//     }
//
//     #[derive(Component, Debug)]
//     #[storage(VecStorage)]
//     pub struct Name { pub name: String }
//
//     #[derive(Component, Debug)]
//     #[storage(VecStorage)]
//     pub struct Position { pub x: f32, pub y: f32, }
//
//
//
//     /// Another resource
//     #[derive(Debug, Default)]
//     struct Bar;
//
//     impl Reflection for Bar {
//         fn call_method(&self, s: &str) {
//             match s {
//                 "bar" => println!("Hello from Bar"),
//                 "foo" => println!("You gotta ask somebody else"),
//                 _ => panic!("The error handling of this example is non-ideal"),
//             }
//         }
//     }
//
//     struct Yar{
//         i:i32
//     }
//
//     struct NormalSys;
//
//     impl<'a> System<'a> for NormalSys {
//         type SystemData = (Read<'a, Foo>, Read<'a, Bar>);
//
//         fn run(&mut self, (foo, bar): Self::SystemData) {
//             println!("Fetched foo: {:?}", &foo as &Foo);
//             println!("Fetched bar: {:?}", &bar as &Bar);
//         }
//     }
//
//     let mut res:World = WorldExt::new();
//
//
//     {
//         let mut table: FetchMut<MetaTable<dyn Reflection>> = res.entry().or_insert_with(|| ReflectionTable::new());
//
//         table.register(&Foo { int: 1 });
//         table.register(&Bar);
//     }
//
//     {
//         let mut table: FetchMut<ResourceTable> = res.entry().or_insert_with(|| ResourceTable::new());
//         table.register::<Foo>("Foo");
//         table.register::<Bar>("Bar");
//     }
//
//     res.register::<Name>();
//     res.register::<Position>();
//
//
//     let mut dispatcher = DispatcherBuilder::new()
//         .with(NormalSys, "normal", &[])
//         .build();
//     dispatcher.setup(&mut res);
//
//     let script0 = create_script_sys(&res);
//
//     // it is recommended you create a second dispatcher dedicated to scripts,
//     // that'll allow you to rebuild if necessary
//     let mut scripts = DispatcherBuilder::new()
//         .with(script0, "script0", &[])
//         .build();
//     scripts.setup(&mut res);
//
//     // Game loop
//     let mut i:i32 = 0;
//     loop {
//         // dispatcher.dispatch(&res);
//         scripts.dispatch(&res);
//         i += 1;
//
//         if i == 10{
//             break;
//         }
//     }
// }
//
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
