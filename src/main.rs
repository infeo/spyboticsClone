//! Demonstrates sprite z ordering
//!
//! Sprites are originally from <https://opengameart.org/content/bat-32x32>, edited to show
//! layering and blending.


use amethyst::{
    assets::{AssetStorage, Handle, Loader, ProgressCounter, Directory},
    core::{Hidden, Transform, TransformBundle},
    ecs::{Entity, Entities, Join, Read,ReadStorage, WriteStorage,World, WorldExt},
    input::{InputBundle,InputHandler,StringBindings,get_mouse_button,is_close_requested, ElementState},
    prelude::*,
    renderer::{
        camera::Projection,
        plugins::{RenderFlat2D, RenderToWindow},
        types::DefaultBackend,
        Camera, ImageFormat, RenderingBundle, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture, Transparent,
    },
    window::ScreenDimensions,
    winit::VirtualKeyCode,
    winit::MouseButton,
};

use log::info;
use std::{env, io, path};
use std::{thread, time};
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::PathBuf;
use rand::prelude::*;

mod spriteIds;

//For the meaning of 'static, see https://doc.rust-lang.org/1.9.0/book/lifetimes.html
static GAME_PATH: &'static str = "D:\\Projekte\\spyboticsClone\\";
static CONFIG_PATH: &'static str = "resource\\config\\display.ron";
static ASSET_PATH: &'static str = "resource\\spybotics-icons\\";
static SPRITE_SHEET_NAME: &'static str = "spritesheet_extended.png";
static RON_FILE_NAME: &'static str = "spritesheet_extended.ron";


#[derive(Debug, Clone)]
struct LoadedSpriteSheet {
    sprite_sheet_handle: Handle<SpriteSheet>,
    sprite_count: u32,
    sprite_rows: u32,
    sprite_columns: u32,
    sprite_w: u32,
    sprite_h: u32,
}

struct Program {

}
#[derive(Debug,Default)]
struct GameField {
   field_columns: u32,
   field_rows: u32,
}

impl GameField {
    fn new() -> Self {
        GameField {
            field_columns: 10,
            field_rows: 10
        }
    }
}

#[derive(Debug, Default)]
struct Spybotics {
    /// The camera entity
    camera: Option<Entity>,
    /// The bat entities.
    entities: Vec<Entity>,
    /// Whether or not to add the transparent component to the entities
    pause: bool,

    game_field: GameField,
    /// Information about the loaded sprite sheet.
    loaded_sprite_sheet: Option<Handle<SpriteSheet>>,
    /// Z-axis position of the camera.
    ///
    /// The Z axis increases "out of the screen" if the camera faces the XY plane (i.e. towards the
    /// origin from (0.0, 0.0, 1.0)). This is the default orientation, when no rotation is applied to the
    /// camera's transform.
    camera_z: f32,
    /// Depth (Z-axis distance) that the camera can see.
    ///
    /// The camera cannot see things on the limits of its view, i.e. entities with the same Z
    /// coordinate cannot be seen, and entities at `Z - camera_depth_vision` also cannot be seen.
    /// Entities with Z coordinates between these limits are visible.
    camera_depth_vision: f32,

}

impl SimpleState for Spybotics {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let StateData { world, .. } = data;

        self.loaded_sprite_sheet = Some(self.load_sprite_sheet(world));

        // //wait until the sprites are loaded
        // let one_second = time::Duration::from_secs(1);
        // thread::sleep(one_second);

        self.initialise_camera(world);
        self.draw_field(world);
    }

    fn handle_event(&mut self, data: StateData<'_, GameData<'_,'_>>, event: StateEvent) -> SimpleTrans {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) {
                return Trans::Quit;
            };

            let input = InputHandler::<StringBindings>::new();
            if let Some(mouse_position) = input.mouse_position() {
                info!("Mouse position {} {}",mouse_position.0, mouse_position.1);
            } else {
                info!("Could not determine mouse position.");
            }

            match get_mouse_button(&event) {

                Some((MouseButton::Left, ElementState::Pressed)) => {
                    self.pause = !self.pause;
                    info!(
                        "Animation paused {}",
                        if self.pause {
                            "enabled"
                        } else {
                            "paused"
                        }
                    );
                }

                _ => {}
            };
        }

        Trans::None
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_,'_>>) -> SimpleTrans{
        let StateData { world, .. } = data;
        // if !self.pause {
        //     self.draw_sprites(world);
        // }
        Trans::None
    }
}

impl Spybotics {

    fn new() -> Self {
        Spybotics {
            camera: None,
            entities: Vec::new(),
            pause: false,
            game_field: GameField::new(),
            loaded_sprite_sheet: None,
            camera_z: 0.0,
            camera_depth_vision: 0.0,
        }
    }
    /// This method initialises a camera which will view our sprite.
    fn initialise_camera(&mut self, world: &mut World) {

        self.camera_z = 1.0;
        self.camera_depth_vision = 5.0;

        self.adjust_camera(world);
    }

    fn adjust_camera(&mut self, world: &mut World) {
        if let Some(camera) = self.camera.take() {
            world
                .delete_entity(camera)
                .expect("Failed to delete camera entity.");
        }

        let (width, height) = {
            let dim = world.read_resource::<ScreenDimensions>();
            (dim.width(), dim.height())
        };

        let mut camera_transform = Transform::default();
        // camera_transform.set_translation_xyz((width as f32) * 0.5, (height as f32) * 0.5, self.camera_z);
        camera_transform.set_translation_xyz(0.0,0.0, self.camera_z);

        let camera = world
            .create_entity()
            .with(camera_transform)
            // Define the view that the camera can see. It makes sense to keep the `near` value as
            // 0.0, as this means it starts seeing anything that is 0 units in front of it. The
            // `far` value is the distance the camera can see facing the origin.
            .with(Camera::from(Projection::orthographic(
                -width / 2.0,
                width / 2.0,
                -height / 2.0,
                height / 2.0,
                0.0,
                self.camera_depth_vision,
            )))
            .build();

        self.camera = Some(camera);
    }

    // fn redraw_sprites(&mut self, world: &mut World) {
    //     let &SpriteSheet {
    //         sprites,
    //         ..
    //     } = self
    //         .loaded_sprite_sheet
    //         .as_ref()
    //         .expect("Expected sprite sheet to be loaded.");
    //
    //
    //     // Delete any existing entities
    //     self.entities.drain(..).for_each(|entity| {
    //         world
    //             .delete_entity(entity)
    //             .expect("Failed to delete entity.")
    //     });
    //
    //     self.draw_sprites(world);
    // }

    fn draw_sprites(&mut self, world: &mut World) {
        // let sprite_count = {
        //     let asset_storage = world.read_resource::<AssetStorage<SpriteSheet>>();
        //     asset_storage
        //         .get(self.loaded_sprite_sheet.as_ref().unwrap())
        //         .expect("Why is this so complicated????")
        //         .sprites.len()
        // };

        // Delete any existing entities
        self.entities.drain(..).for_each(|entity| {
            world
                .delete_entity(entity)
                .expect("Failed to delete entity.")
        });

        let mut common_transform = Transform::default();
        common_transform.set_translation_x(-350.0 * 0.5);
        common_transform.set_translation_y(-350.0 * 0.5);

        let cols = 10;
        // Create an entity per sprite.
        for i in 0..144 {

            let mut sprite_transform = Transform::default();
            let mut random_gen = rand::thread_rng();
            // sprite_transform.set_translation_xyz((i % cols * 32) as f32, ((i / cols * 32) as f32), -1.0);
            sprite_transform.set_translation_xyz(random_gen.gen_range(100.0,500.0),random_gen.gen_range(100.0,500.0), -1.0);

            sprite_transform.concat(&common_transform);

            let sprite_render = SpriteRender {
                sprite_sheet: self.loaded_sprite_sheet.as_ref().unwrap().clone(),
                sprite_number: i,
            };

            let entity_builder = world
                .create_entity()
                .with(sprite_render)
                .with(sprite_transform);

            self.entities.push(entity_builder.build());
        }
    }

    fn draw_field(&mut self,world: &mut World){

        // Delete any existing entities
        self.entities.drain(..).for_each(|entity| {
            world
                .delete_entity(entity)
                .expect("Failed to delete entity.")
        });

        let mut common_transform = Transform::default();
        common_transform.set_translation_x(-350.0 * 0.5);
        common_transform.set_translation_y(-350.0 * 0.5);

        for i in 0..self.game_field.field_rows {
            for j in 0..self.game_field.field_columns {

                let mut sprite_transform = Transform::default();
                sprite_transform.set_translation_xyz((i * 32) as f32, (j * 32) as f32, -1.0);

                sprite_transform.concat(&common_transform);

                let sprite_render = SpriteRender {
                    sprite_sheet: self.loaded_sprite_sheet.as_ref().unwrap().clone(),
                    sprite_number: spriteIds::UPLOADZONE,
                };

                let entity_builder = world
                    .create_entity()
                    .with(sprite_render)
                    .with(sprite_transform);

                self.entities.push(entity_builder.build());
            }
        }
    }
    /// Loads and returns a handle to a sprite sheet.
    ///
    /// The sprite sheet consists of two parts:
    ///
    /// * texture: the pixel data
    /// * `SpriteSheet`: the layout information of the sprites on the image
    fn load_sprite_sheet(&mut self,world: &mut World) -> Handle<SpriteSheet> {

        let texture_handle = {
            let loader = world.read_resource::<Loader>();
            let texture_storage = world.read_resource::<AssetStorage<Texture>>();
            loader.load(
                SPRITE_SHEET_NAME,
                ImageFormat::default(),
                (),
                &texture_storage,
            )
        };

        let loader = world.read_resource::<Loader>();
        loader.load(
            RON_FILE_NAME,
            SpriteSheetFormat(texture_handle),
            (),
            &world.read_resource::<AssetStorage<SpriteSheet>>(),
        )
    }
}


fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = path::PathBuf::from(GAME_PATH);

    let display_config_path = app_root.join(CONFIG_PATH);
    println!("{:?}",display_config_path.to_str());

    let assets_dir = app_root.join(ASSET_PATH);
    println!("{:?}",assets_dir.to_str());

    let game_data = GameDataBuilder::default()
        .with_bundle(TransformBundle::new())?
        .with_bundle(InputBundle::<StringBindings>::new())?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderFlat2D::default()),
        )?;

    let mut game = Application::new(assets_dir, Spybotics::new(), game_data)?;
    game.run();

    Ok(())
}