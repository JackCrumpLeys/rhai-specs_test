use specs::*;
use crate::specs_rhai_magic::*;

/// A dynamic system that represents and calls the script.
pub struct DynamicSystem {
    pub dependencies: Dependencies,
    /// just a dummy, you would want an actual script handle here
    pub script: fn(ScriptInput),
}

impl<'a> System<'a> for DynamicSystem {
    type SystemData = ScriptSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let meta = data.meta_table;
        let reads: Vec<&dyn Reflection> = data
            .reads
            .iter()
            .map(|resource| {
                // explicitly use the type because we're dealing with `&Resource` which is
                // implemented by a lot of types; we don't want to accidentally
                // get a `&Box<Resource>` and cast it to a `&Resource`.
                let res = Box::as_ref(resource);

                meta.get(res).expect("Not registered in meta table")
            })
            .collect();

        let writes: Vec<&mut dyn Reflection> = data
            .writes
            .iter_mut()
            .map(|resource| {
                // explicitly use the type because we're dealing with `&mut Resource` which is
                // implemented by a lot of types; we don't want to accidentally get a
                // `&mut Box<Resource>` and cast it to a `&mut Resource`.
                let res = Box::as_mut(resource);

                // For some reason this needs a type ascription, otherwise Rust will think it's
                // a `&mut (Reflection + '_)` (as opposed to `&mut (Reflection + 'static)`.
                let res: &mut dyn Reflection = meta.get_mut(res).expect(
                    "Not registered in meta \
                     table",
                );

                res
            })
            .collect();

        let input = ScriptInput { reads, writes };

        // call the script with the input
        (self.script)(input);
    }

    fn accessor<'b>(&'b self) -> AccessorCow<'a, 'b, Self> {
        AccessorCow::Ref(&self.dependencies)
    }

    fn setup(&mut self, _res: &mut World) {
        // this could call a setup function of the script
    }
}