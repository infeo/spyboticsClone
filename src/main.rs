//! Demonstrates sprite z ordering
//!
//! Sprites are originally from <https://opengameart.org/content/bat-32x32>, edited to show
//! layering and blending.


use amethyst::{
    assets::{AssetStorage, Handle, Loader, ProgressCounter, Directory},
    core::{Hidden, Transform, TransformBundle,
        geometry::Plane,
        math::{Point2,Point3,Vector2,Vector3},
    },
    ecs::{Entity, Entities, Join, Read,ReadStorage, WriteStorage,World, WorldExt,
          System, SystemData, Component, DenseVecStorage, ReadExpect},
    input::{InputBundle,InputHandler,StringBindings,get_mouse_button,is_close_requested, ElementState, Button},
    prelude::*,
    derive::SystemDesc,
    renderer::{
        camera::{Projection,ActiveCamera},
        plugins::{RenderFlat2D, RenderToWindow},
        types::DefaultBackend,
        Camera, ImageFormat, RenderingBundle, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture, Transparent,
    },
    utils::{application_dir,application_root_dir},
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
// static CONFIG_PATH: &'static str = "resource\\config\\display.ron";
static DISPLAY_PATH: &'static str = "resource/config/display.ron";
static ASSET_PATH: &'static str = "resource/spybotics-icons/";
static SPRITE_SHEET_NAME: &'static str = "spritesheet_extended.png";
static RON_FILE_NAME: &'static str = "spritesheet_extended.ron";

const  GAMEFIELD_EXTENT: (u32,u32) = (15, 15);
const ARENA_HEIGHT: f32 = (32*GAMEFIELD_EXTENT.0) as f32;
const ARENA_WIDTH: f32 = (32*GAMEFIELD_EXTENT.1) as f32;


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

/// Component to carry information about the position about a game tile
/// Grid position contains the coordinates in the game field grid
/// world position contains the coordinates of the lower left corner of the tile on the world
/// world extent gives the size in y and x direction of the tile
#[derive(Debug,Default)]
struct GameTilePosition {
    grid_position: (u32, u32),
    world_position: (f32, f32),
    world_extent: (f32, f32)
}

impl GameTilePosition {
    fn is_inside(&self,world_coordinates:(f32, f32)) -> bool{
        let (left,right,top,bottom) = {
            (
                self.world_position.0,
                self.world_position.0 + self.world_extent.0,
                self.world_position.1 + self.world_extent.1,
                self.world_position.1,
            )
        };
        world_coordinates.0 > left &&
            world_coordinates.0 < right &&
            world_coordinates.1 > bottom &&
            world_coordinates.1 < top
    }
}

impl Component for GameTilePosition {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Default)]
struct GameTileSpriteStack {
    sprite_stack: Vec<Entity>
}

impl Component for GameTileSpriteStack {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Debug, Default)]
struct Walkable {
    walkable: bool,
}

impl Walkable {

    fn new(_walkable: bool) -> Self {
        Walkable {
            walkable: _walkable,
        }
    }

}

impl Component for Walkable {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default,Clone)]
struct HandleHandle {
    sprite_sheet_handle: Option<Handle<SpriteSheet>>,
}

#[derive(Debug, Default)]
struct Spybotics {
    /// The camera entity
    camera: Option<Entity>,
    /// The bat entities. TODO: Think about if this can be removed.
    entities: Vec<Entity>,
    /// Whether or not to add the transparent component to the entities
    pause: bool,

    /// The game field matrix
    game_field: Vec<Entity>,
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

        world.insert(DenseVecStorage::<GameTilePosition>::default());

        self.loaded_sprite_sheet = Some(self.load_sprite_sheet(world));

        // //wait until the sprites are loaded
        // let one_second = time::Duration::from_secs(1);
        // thread::sleep(one_second);

        self.initialise_camera(world);
        self.initialize_field(world);
    }

    fn handle_event(&mut self, data: StateData<'_, GameData<'_,'_>>, event: StateEvent) -> SimpleTrans {
        if let StateEvent::Window(event) = &event {
            if is_close_requested(&event) {
                return Trans::Quit;
            };

                match get_mouse_button(&event) {


                Some((MouseButton::Left, ElementState::Pressed)) => {
                        // try_fetch returns a Option<Fetch<MyResource>>
                        let fetched = data.world.try_fetch::<InputHandler<StringBindings>>();
                        if let Some(fetched_resource) = fetched {
                            //dereference Fetch<MyResource> to access data
                            if let Some(mouse_position) = fetched_resource.mouse_position() {
                                //TODO: do something with the mouse input
                            } else {
                                println!("Mouse Position not available.");
                            }
                        } else {
                            println!("No InputHandler present in `World`");
                        }
                        /*
                            self.pause = !self.pause;
                            info!(
                                "Animation paused {}",
                                if self.pause {
                                    "enabled"
                                } else {
                                    "paused"
                                }
                            );
                        */
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
            game_field: Vec::new(),
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
        /*
        Nice to have, but right now we'll fix the game field to a certain size

        let (width, height) = {
            let dim = world.read_resource::<ScreenDimensions>();
            (dim.width(), dim.height())
        };
        */

        let (width, height) = (ARENA_WIDTH, ARENA_HEIGHT);

        let mut camera_transform = Transform::default();
        camera_transform.set_translation_xyz((width as f32) * 0.5, (height as f32) * 0.5, self.camera_z);
        //camera_transform.set_translation_xyz(0.0,0.0, self.camera_z);

        let camera = world
            .create_entity()
            .with(Camera::standard_2d(width, height))
            .with(camera_transform)
            // Define the view that the camera can see. It makes sense to keep the `near` value as
            // 0.0, as this means it starts seeing anything that is 0 units in front of it. The
            // `far` value is the distance the camera can see facing the origin.
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

    fn initialize_field(&mut self, world: &mut World){

        // Delete any existing entities TODO: do we need this?
        self.entities.drain(..).for_each(|entity| {
            world
                .delete_entity(entity)
                .expect("Failed to delete entity.")
        });

        self.game_field = Vec::new();

        let (sprite_offset_w, sprite_offset_h) = (16.0,16.0); //TODO: rather than hardcoding, we should load this from the spritesheet itself
        let mut common_transform = Transform::default();
        common_transform.set_translation_x(sprite_offset_w);
        common_transform.set_translation_y(sprite_offset_h);


        world.insert(HandleHandle{
            sprite_sheet_handle: Some(self.loaded_sprite_sheet.as_ref().unwrap().clone()),
        });

        for i in 0..GAMEFIELD_EXTENT.0 {
            for j in 0..GAMEFIELD_EXTENT.1 {

                let mut sprite_transform = Transform::default();
                let world_pos = ((i * 32) as f32, (j * 32) as f32);
                sprite_transform.set_translation_xyz(world_pos.0, world_pos.1, -1.0);

                sprite_transform.concat(&common_transform);

                let sprite_render = SpriteRender {
                    sprite_sheet: self.loaded_sprite_sheet.as_ref().unwrap().clone(),
                    sprite_number: spriteIds::UPLOADZONE,
                };


                let sprite_entity_builder = world
                    .create_entity()
                    .with(sprite_render)
                    .with(sprite_transform);

                //self.entities.push(entity_builder.build());

                let sprite_stack = GameTileSpriteStack {
                    sprite_stack: vec![sprite_entity_builder.build()]
                };

                let position = GameTilePosition{
                    grid_position: (i,j),
                    world_position: world_pos.clone(),
                    world_extent: (32.0,32.0)
                };

                let game_tile_builder = world.create_entity()
                    .with(position)
                    .with(sprite_stack)
                    .with( Walkable::new(true));


                self.game_field.push(game_tile_builder.build());
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

#[derive(SystemDesc)]
struct MainSystem {

}

impl<'a> System<'a> for MainSystem {

    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Camera>,
        Read<'a, InputHandler<StringBindings>>,
        Read<'a, ActiveCamera>,
        Read<'a,HandleHandle>,
        ReadExpect<'a, ScreenDimensions>,
        WriteStorage<'a,SpriteRender>,
        WriteStorage<'a,Transform>,
        WriteStorage<'a, GameTilePosition>,
        WriteStorage<'a, GameTileSpriteStack>,
        WriteStorage<'a, Walkable>,
    );

    fn run(&mut self, ( entities,
                        cameras,
                        input,
                        active_camera,
                        sprite_sheet_handle,
                        screen_dimensions,
                        mut sprites,
                        mut transforms,
                        mut game_tile_position,
                        mut game_tile_sprite_stack,
                        mut walkable,
                        ): Self::SystemData){

        // Get the mouse position if its available
        if input.button_is_down(Button::Mouse(MouseButton::Left)) {
            if let Some(mouse_position) = input.mouse_position() {
                // Get the active camera if it is spawned and ready
                let mut camera_join = (&cameras, &transforms).join();
                if let Some((camera, camera_transform)) = active_camera
                    .entity
                    .and_then(|a| camera_join.get(a, &entities))
                    .or_else(|| camera_join.next())
                {
                    // creates a point with the screen coordinates of the mouse pointer
                    let mouse_coordinate = Some(Point3::new(
                        mouse_position.0,
                        mouse_position.1,
                        camera_transform.translation().z,
                    ));
                    let screen_dimensions_vector =
                        Vector2::new(screen_dimensions.width(), screen_dimensions.height());
                    // creates a point with the _world_ coordinates of the mouse pointer
                    let mut world_coordinate = camera.projection().screen_to_world_point(
                        mouse_coordinate.expect("Dafuq!"),
                        screen_dimensions_vector,
                        camera_transform,
                    );

                    // Find any sprites which the mouse is currently inside
                    for (e, tile_position, tile_stack) in (&*entities, &game_tile_position, &mut game_tile_sprite_stack).join() {

                        if tile_position.is_inside((world_coordinate.x,world_coordinate.y)){
                            let mut common_transform = Transform::default();
                            //TODO: DO NOT USE HARDCODED OFFSET
                            common_transform.set_translation_x(16.0);
                            common_transform.set_translation_y(16.0);

                            let mut sprite_transform = Transform::default();
                            sprite_transform.set_translation_xyz(tile_position.world_position.0, tile_position.world_position.1, 0.0);

                            sprite_transform.concat(&common_transform);

                            let sprite_render = SpriteRender {
                                sprite_sheet: sprite_sheet_handle.sprite_sheet_handle.as_ref().unwrap().clone(),
                                sprite_number: spriteIds::SELECTSQUAREGREEN,
                            };

                            let sprite_entity_builder = entities
                                .build_entity()
                                .with(sprite_render,&mut sprites)
                                .with(sprite_transform,&mut transforms);

                           tile_stack.sprite_stack.push(sprite_entity_builder.build());
                        }
                    }
                }
            }
        }
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = path::PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")
        .expect("Could not find CARGO_MANIFEST_DIR env variable, pointing to cargo manifest"));

    let display_config_path = app_root.join(DISPLAY_PATH);
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
        )?
        .with(MainSystem{},"MainSystem", &["input_system"]);

    let mut game = Application::new(assets_dir, Spybotics::new(), game_data)?;
    game.run();

    Ok(())
}