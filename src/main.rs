use std::time::Instant;

use glfw::{Context, WindowHint};

mod components;
mod quadtree;
mod shader;
mod systems;
mod utils;

use glow::HasContext;
use shader::Shader;
use systems as sys;

use components::*;
use quadtree::*;

use glam::Vec2;
use legion::*;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;

const POINT_COUNT: usize = 8;
const BUFFER_ACCESS_FLAGS: u32 =
    glow::MAP_WRITE_BIT | glow::MAP_READ_BIT | glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT;

// x, y, radius, red, green, blue
const FLOATS_PER_INSTANCE: usize = 6;
const INSTANCE_DATA_STRIDE: usize = std::mem::size_of::<f32>() * FLOATS_PER_INSTANCE;

const INITIAL_BUFFER_FLOAT_CAPACITY: usize = 1_000_000;

fn main() {
    let mut world = World::default();

    let mut resources = Resources::default();
    let mut schedule = Schedule::builder()
        .add_system(sys::update_positions_system())
        .add_system(sys::check_wall_collision_system())
        .build();

    resources.insert(InstanceCount(0));

    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

    glfw.window_hint(WindowHint::ContextVersionMinor(3));
    glfw.window_hint(WindowHint::ContextVersionMajor(3));
    glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, event) = glfw
        .create_window(WIDTH, HEIGHT, "Rendering text", glfw::WindowMode::Windowed)
        .unwrap();

    resources.insert(window.get_size());

    window.set_cursor_pos_polling(true);
    window.set_key_polling(true);
    window.set_mouse_button_polling(true);

    window.make_current();

    // gl::load_with(|s| window.get_proc_address(s));
    let gl =
        unsafe { glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _) };

    glfw.set_swap_interval(glfw::SwapInterval::None);
    unsafe { gl.viewport(0, 0, window.get_size().0, window.get_size().1) };
    window.set_size_polling(true);

    let (vao, vbo, ebo);
    let (vertices, indices) = utils::generate_circle(POINT_COUNT as _);

    unsafe {
        vao = gl.create_vertex_array().unwrap();
        vbo = gl.create_buffer().unwrap();
        ebo = gl.create_buffer().unwrap();

        gl.bind_vertex_array(Some(vao));

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));

        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            std::slice::from_raw_parts(
                vertices.as_ptr() as _,
                vertices.len() * std::mem::size_of::<f32>(),
            ),
            glow::STATIC_DRAW,
        );

        gl.buffer_data_u8_slice(
            glow::ELEMENT_ARRAY_BUFFER,
            std::slice::from_raw_parts(
                indices.as_ptr() as _,
                indices.len() * std::mem::size_of::<f32>(),
            ),
            glow::STATIC_READ,
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(
            0,
            2,
            glow::FLOAT,
            false,
            2 * std::mem::size_of::<f32>() as i32,
            0,
        );

        // vao still bound
    }

    // instancing
    let mut instance_vbo;
    let mut instance_data_ptr;
    let mut instance_data_offset = 0;
    let mut buffer_capacity = INITIAL_BUFFER_FLOAT_CAPACITY;

    unsafe {
        instance_vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_vbo));

        gl.buffer_storage(
            glow::ARRAY_BUFFER,
            (buffer_capacity * INSTANCE_DATA_STRIDE) as _,
            None,
            BUFFER_ACCESS_FLAGS,
        );

        instance_data_ptr = gl.map_buffer_range(
            glow::ARRAY_BUFFER,
            0,
            (buffer_capacity * INSTANCE_DATA_STRIDE) as _,
            BUFFER_ACCESS_FLAGS,
        ) as *mut f32;

        utils::setup_instance_attributes(&gl);
        resources.insert(InstanceDataPtr::new(instance_data_ptr));

        // unbind
        gl.bind_vertex_array(None);
    }

    let shader = Shader::from_str(&gl, include_str!("shader.glsl"), "vertex", "fragment")
        .expect("Failed to load shader");

    let orthographic_uniform = |(width, height)| unsafe {
        shader.use_shader();
        gl.uniform_matrix_4_f32_slice(
            Some(&shader.get_uniform_location("ortho").unwrap()),
            false,
            &glam::Mat4::orthographic_rh_gl(0.0, width as _, height as _, 0.0, -1.0, 1.0)
                .to_cols_array(),
        );
    };

    orthographic_uniform(window.get_size());

    let quad_capacity = 32;
    let mut mouse_down = false;
    let particle_radius: f32 = 10.0;

    let mut clock = Instant::now();
    while !window.should_close() {
        let dt = clock.elapsed().as_nanos() as f32 / 1e9;
        clock = Instant::now();

        println!(
            "FPS: {:.0}, {} particles",
            1.0 / dt,
            resources.get::<InstanceCount>().unwrap().0
        );

        glfw.poll_events();

        use glfw::WindowEvent;

        glfw::flush_messages(&event).for_each(|(_, event)| match event {
            WindowEvent::Key(glfw::Key::Escape, ..) | WindowEvent::Close => {
                window.set_should_close(true)
            }

            WindowEvent::Size(width, height) => {
                unsafe { gl.viewport(0, 0, width, height) };
                orthographic_uniform((width, height));
                resources.insert(window.get_size());
            }

            WindowEvent::MouseButton(glfw::MouseButtonLeft, glfw::Action::Press, _) => {
                mouse_down = true
            }

            WindowEvent::MouseButton(glfw::MouseButtonLeft, glfw::Action::Release, _) => {
                mouse_down = false
            }

            _ => {}
        });

        // let ptr = resources.get::<InstanceDataPtr>().unwrap().get_ptr();
        // let mut qt = quadtree::QuadTree::<usize>::new(
        //     quad_capacity,
        //     Rect {
        //         left: 0.,
        //         top: 0.,
        //         width: window.get_size().0 as _,
        //         height: window.get_size().1 as _,
        //     },
        // );

        // <&EntityIndex>::query().for_each(&world, |id| {
            // let [x, y, r, ..] = utils::get_entity(id.0, ptr);
            // qt.push((glam::vec2(*x, *y), *r, id.0));
        // });

        // resources.insert(qt);

        if mouse_down {
            for _ in 0..100 {
                if instance_data_offset / 3 < buffer_capacity {
                    let v_x: f32 = rand::random_range(-30.0..30.0);
                    let v_y: f32 = rand::random_range(-30.0..30.0);

                    let (x, y) = window.get_cursor_pos();
                    unsafe {
                        let r = rand::random_range(0.0..=1.0);
                        let g = rand::random_range(0.0..=1.0);
                        let b = rand::random_range(0.0..=1.0);

                        // copy position and radius data to GPU
                        let entity_data: [f32; FLOATS_PER_INSTANCE] =
                            [x as _, y as _, particle_radius, r, g, b];

                        std::ptr::copy_nonoverlapping(
                            entity_data.as_ptr(),
                            instance_data_ptr.add(instance_data_offset),
                            entity_data.len(),
                        )
                    };

                    instance_data_offset += FLOATS_PER_INSTANCE;

                    world.push((
                        // used as pointer offset in systems
                        EntityIndex(resources.get::<InstanceCount>().unwrap().0 as _),
                        Velocity(glam::vec2(v_x, v_y)),
                        Mass(particle_radius.powi(2)),
                    ));

                    resources.get_mut::<InstanceCount>().unwrap().0 += 1;
                } else {
                    let old_capacity = buffer_capacity;
                    buffer_capacity *= 2;

                    unsafe {
                        utils::reallocate_instance_vbo(
                            &gl,
                            buffer_capacity,
                            old_capacity,
                            &mut instance_data_ptr,
                            &mut instance_vbo,
                            vao,
                        )
                    };

                    // update the old pointer
                    resources.insert(InstanceDataPtr::new(instance_data_ptr));
                }
            }
        }

        resources.insert(DeltaTime(dt));
        schedule.execute(&mut world, &mut resources);

        unsafe {
            gl.clear_color(0.01, 0.01, 0.01, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT);

            shader.use_shader();

            gl.bind_vertex_array(Some(vao));
            gl.draw_elements_instanced(
                glow::TRIANGLES,
                indices.len() as _,
                glow::UNSIGNED_INT,
                0 as _,
                resources.get::<InstanceCount>().unwrap().0,
            );
        }

        window.swap_buffers();
    }
}
