use core::panicking::AssertKind::Match;
use std::any::{Any, type_name, TypeId};
use rhai::{Engine, Scope, AST};
use specs::prelude::*;
use specs::shred::cell::{Ref, RefMut};
use specs::shred::{CastFrom, DynamicSystemData, FetchMut, MetaTable};
use specs::Component;
use specs::{Read, World, WorldExt};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

pub struct Dependencies {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
}

impl Accessor for Dependencies {
    fn try_new() -> Option<Self> {
        // there's no default for this
        None
    }

    fn reads(&self) -> Vec<ResourceId> {
        let mut reads = self.reads.clone();
        reads.push(ResourceId::new::<ReflectionTable>());

        reads
    }

    fn writes(&self) -> Vec<ResourceId> {
        self.writes.clone()
    }
}

pub type ReflectionTable = MetaTable<dyn ScriptableComponent>;

// gets data
pub struct ScriptSystemData<'a> {
    pub(crate) meta_table: Read<'a, ReflectionTable>,
    pub(crate) reads: Vec<Ref<'a, Box<dyn Resource + 'static>>>,
    pub(crate) writes: Vec<RefMut<'a, Box<dyn Resource + 'static>>>,
}

impl<'a> DynamicSystemData<'a> for ScriptSystemData<'a> {
    type Accessor = Dependencies;

    fn setup(_accessor: &Dependencies, _res: &mut World) {}

    fn fetch(access: &Dependencies, res: &'a World) -> Self {
        let reads = access
            .reads
            .iter()
            .map(|id| {
                res.try_fetch_internal(id.clone())
                    .expect("bug: the requested resource does not exist")
                    .borrow()
            })
            .collect();
        let writes = access
            .writes
            .iter()
            .map(|id| {
                res.try_fetch_internal(id.clone())
                    .expect("bug: the requested resource does not exist")
                    .borrow_mut()
            })
            .collect();

        ScriptSystemData {
            meta_table: SystemData::fetch(res),
            reads,
            writes,
        }
    }
}

/// Maps resource names to resource ids.
pub struct ResourceTable {
    map: HashMap<String, ResourceId>,
}

impl ResourceTable {
    pub(crate) fn new() -> Self {
        ResourceTable {
            map: HashMap::default(),
        }
    }

    pub(crate) fn register<T: Resource>(&mut self, name: &str) {
        self.map
            .insert(name.to_owned(), ResourceId::new_with_dynamic_id::<T>(0));
    }

    fn get(&self, name: &str) -> ResourceId {
        self.map.get(name).cloned().unwrap()
    }
}

/// trait that all components that scripts can access should implement
trait ScriptableComponent {
    fn setup(&mut self, name: &str) {
        println!("setting up: {}", name)
    }
}

/// dummy component for testing
#[derive(Component, Debug)]
#[storage(VecStorage)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl ScriptableComponent for Position {}

// necessary for `MetaTable`
unsafe impl<T> CastFrom<T> for dyn ScriptableComponent
where
    T: ScriptableComponent + 'static,
{
    fn cast(t: &T) -> &Self {
        t
    }

    fn cast_mut(t: &mut T) -> &mut Self {
        t
    }
}

struct WorldHelper {
    engine: Engine,
    unassigned_scripts: HashMap<String ,Script>,
    script_map: HashMap<TypeId, Script>,
    world: World
}

impl WorldHelper {
    fn register_scriptable<S: ScriptableComponent>(&mut self){
        let mut scripts = &self.unassigned_scripts;
        let name = type_name::<S>();
        self.world.register::<S>();
        let matching_script = scripts.get(type_name::<S>());
        match matching_script {
            Some(_script) => self.script_map.insert(TypeId::of::<S>(), scripts.remove(name).unwrap()) // no protection cause we already know it exists
            None => println!("Could not find a script for: {}, resorting to 'default' script", name) // TODO: default script
        };

    }
}

fn main() {
    let engine: Engine = Engine::new();

    let mut scripts = Vec::new();
    for file in PathBuf::from(r#"C:\Users\jackc\CLionProjects\rhai-specs_test\scripts"#)
        .read_dir()
        .unwrap()
    {
        match file {
            Ok(file) => scripts.push(load_script(file.path(), &engine)),
            Err(file) => println!("error getting file {}", file),
        }
    }
    TypeId::of::<Position>();

    for script in scripts {
        println!("{}", script.name)
    }

    let mut world: World = WorldExt::new();

    world.register::<Position>();

    world
        .create_entity()
        .with(Position { x: 4.0, y: 7.0 })
        .build();

    let mut hello_world = HelloWorld;
    hello_world.run_now(&world);
    world.maintain();
}

/// load a script from a file path
fn load_script(path: PathBuf, engine: &Engine) -> Script {
    let ast: AST = engine.compile_file(path.clone()).unwrap();

    let mut scope = Scope::new();
    engine.run_ast_with_scope(&mut scope, &ast);

    let _result: () = engine.call_fn(&mut scope, &ast, "load", ()).unwrap();
    let mut scripts: Script = Script {
        name: path.file_name().unwrap().to_str().unwrap().split('.').collect::<Vec<&str>>()[0].to_string(),
        script_ast: ast,
        scope,
        last_run: Instant::now(),
    };
    scripts
}

/// tests stuff
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

pub struct ScriptInput<'a> {
    pub(crate) reads: HashMap<&'a str, &'a dyn ScriptableComponent>,
    pub(crate) writes: HashMap<&'a str, &'a mut dyn ScriptableComponent>,
}

#[derive(Clone, Debug)]
struct Script {
    name: String,
    script_ast: AST,
    scope: Scope<'static>,
    last_run: Instant,
}

impl Script {
    fn call_method(&self, input: &mut ScriptInput) {
        println!("pp")
    }
}

struct HelloWorld;

impl<'a> System<'a> for HelloWorld {
    type SystemData = ReadStorage<'a, Position>;

    fn run(&mut self, position: Self::SystemData) {
        use specs::Join;

        for position in position.join() {
            println!("Hello, {:?}", &position);
        }
    }
}
