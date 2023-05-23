use std::{cell::RefCell, sync::Arc};

use extism::{CurrentPlugin, Plugin, PluginBuilder, UserData, Val, ValType};

use super::{world::material::color::Color, FileHelper, Rect};

pub struct ModManager {
    mods: Vec<Mod>,
}

impl ModManager {
    pub fn init(file_helper: &FileHelper) -> Self {
        let mut mods = vec![];

        let call_ctx = ModCallContext { post_world_render_target: Arc::default() };

        let fns = vec![extism::Function::new(
            "draw_rect",
            [ValType::F32, ValType::F32, ValType::F32, ValType::F32],
            [],
            Some(UserData::new(call_ctx.post_world_render_target.clone())),
            draw_rect,
        )];

        for path in file_helper.mod_files() {
            log::info!("Loading mod {path:?}");
            let wasm =
                std::fs::read(&path).expect(format!("Failed to read mod file: {path:?}").as_str());
            let plugin = PluginBuilder::new_with_module(wasm)
                .with_wasi(true)
                .with_functions(fns.clone())
                .build(None)
                .expect(format!("Failed to instantiate mod: {path:?}").as_str());

            mods.push(Mod { call_ctx: call_ctx.clone(), plugin });
        }

        Self { mods }
    }

    pub fn mods(&self) -> &[Mod] {
        &self.mods
    }

    pub fn mods_mut(&mut self) -> &mut [Mod] {
        &mut self.mods
    }
}

#[derive(Clone)]
pub struct ModCallContext {
    post_world_render_target: Arc<RefCell<Option<*mut dyn PostWorldRenderTarget>>>,
}

impl ModCallContext {
    #[allow(clippy::transmute_ptr_to_ptr)]
    pub fn with_post_world_render_target<T: PostWorldRenderTarget>(
        &mut self,
        t: &mut T,
        f: impl FnOnce(&mut Self),
    ) {
        // TODO: this transmute could easily be UB, but I couldn't figure out any other way to do this
        // it's only being used to extend the lifetime of `t`, which will never be stored in `post_world_render_target` after this function returns
        *self.post_world_render_target.borrow_mut() =
            Some(unsafe { std::mem::transmute(t as *mut dyn PostWorldRenderTarget) });
        f(self);
        *self.post_world_render_target.borrow_mut() = None;
    }
}

pub struct Mod {
    call_ctx: ModCallContext,
    plugin: Plugin<'static>,
}

pub trait PostWorldRenderTarget {
    fn rectangle(&mut self, rect: Rect<f32>, color: Color);
}

impl Mod {
    pub fn test_fn(&mut self) -> String {
        self.plugin
            .call_map("test", [], |bytes| {
                String::from_utf8(bytes.to_vec()).map_err(Into::into)
            })
            .expect("call_map test failed")
    }

    pub fn post_world_render<T: PostWorldRenderTarget>(&mut self, target: &mut T) {
        self.call_ctx.with_post_world_render_target(target, |_| {
            self.plugin.call("post_world_render", []).unwrap();
        });
    }
}

#[allow(clippy::unnecessary_wraps)]
#[allow(clippy::needless_pass_by_value)]
fn draw_rect(
    _plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    user_data: UserData,
) -> Result<(), extism::Error> {
    let rt: &mut Arc<RefCell<Option<*mut dyn PostWorldRenderTarget>>> =
        unsafe { &mut *user_data.as_ptr().cast() };
    let rt = unsafe { &mut *rt.borrow_mut().unwrap() };

    let rect = Rect::new_wh(
        inputs[0].unwrap_f32(),
        inputs[1].unwrap_f32(),
        inputs[2].unwrap_f32(),
        inputs[3].unwrap_f32(),
    );
    rt.rectangle(rect, Color::CYAN);

    Ok(())
}
