use std::time::Instant;

use glfw::{Context, WindowHint};

mod components;
mod shader;
mod systems;

use components::*;
use glow::HasContext;
use shader::Shader;
use systems as sys;

use legion::*;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;

const POINT_COUNT: usize = 8;

const FLOATS_PER_INSTANCE: usize = 3;
const INSTANCE_DATA_STRIDE: usize = std::mem::size_of::<f32>() * FLOATS_PER_INSTANCE;

// space for 50k particles
const INITIAL_BUFFER_SIZE: usize = std::mem::size_of::<f32>() * 3 * 50_000;

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

    glfw.set_swap_interval(glfw::SwapInterval::Adaptive);
    unsafe { gl.viewport(0, 0, window.get_size().0, window.get_size().1) };
    window.set_size_polling(true);

    let (vao, vbo, ebo);

    let (vertices, indices) = generate_circle(POINT_COUNT as _);

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
            std::slice::from_raw_parts(indices.as_ptr() as _, indices.len() * std::mem::size_of::<f32>()),
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

        // still bound
    }

    // instancing
    let instance_vbo;
    let instance_data_ptr;
    let mut instance_data_offset = 0;

    unsafe {
        instance_vbo = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(instance_vbo));

        gl.buffer_storage(
            glow::ARRAY_BUFFER,
            INITIAL_BUFFER_SIZE as _,
            None,
            glow::MAP_WRITE_BIT | glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT,
        );

        instance_data_ptr = gl.map_buffer_range(
            glow::ARRAY_BUFFER,
            0,
            INITIAL_BUFFER_SIZE as _,
            glow::MAP_WRITE_BIT | glow::MAP_PERSISTENT_BIT | glow::MAP_COHERENT_BIT,
        ) as *mut f32;

        resources.insert(InstanceDataPtr::new(instance_data_ptr));

        let radius_offset = std::mem::size_of::<f32>() * 2;

        // position
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_divisor(1, 1);
        gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, INSTANCE_DATA_STRIDE as _, 0);

        // radius
        gl.enable_vertex_attrib_array(2);
        gl.vertex_attrib_divisor(2, 1);
        gl.vertex_attrib_pointer_f32(
            2,
            1,
            glow::FLOAT,
            false,
            INSTANCE_DATA_STRIDE as _,
            radius_offset as _,
        );

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

    unsafe {
        std::ptr::copy_nonoverlapping(
            [100.0, 100.0, 20.0, 300.0, 300.0, 30.0].as_ptr(),
            instance_data_ptr,
            6,
        )
    };

    let mut mouse_down = false;
    let particle_radius: f32 = 3.0;

    let mut clock = Instant::now();
    while !window.should_close() {
        let dt = clock.elapsed().as_nanos() as f32 / 1e9;
        clock = Instant::now();

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

        if mouse_down && instance_data_offset < 50_000 {
            let v_x: f32 = rand::random_range(-30.0..30.0);
            let v_y: f32 = rand::random_range(-30.0..30.0);

            let (x, y) = window.get_cursor_pos();

            unsafe {
                std::ptr::copy_nonoverlapping(
                    [x as _, y as _, particle_radius].as_ptr(),
                    instance_data_ptr.add(instance_data_offset),
                    3,
                );

                instance_data_offset += 3;
            };

            world.push((
                EntityIndex(resources.get::<InstanceCount>().unwrap().0 as _),
                Velocity(glam::vec2(v_x, v_y)),
                Mass(particle_radius.powi(2)),
            ));

            resources.get_mut::<InstanceCount>().unwrap().0 += 1;
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

fn generate_circle(point_count: u32) -> (Vec<f32>, Vec<u32>) {
    let mut vertices: Vec<f32> = vec![];

    let angle = 2.0 * std::f32::consts::PI / point_count as f32;

    vertices.extend([0.0, 0.0]);
    vertices.extend(
        (0..point_count)
            .map(|i| angle * i as f32)
            .flat_map(|theta| [theta.cos(), theta.sin()]),
    );

    let mut indices: Vec<u32> = Vec::from_iter((0..point_count).flat_map(|i| [0, i, i + 1]));
    indices.extend([0, point_count, 1]);

    (vertices, indices)
}
