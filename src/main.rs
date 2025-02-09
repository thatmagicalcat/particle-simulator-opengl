use std::time::Instant;

use glfw::{Context, WindowHint};

mod components;
mod shader;
mod systems;

use components::*;
use systems as sys;

use shader::Shader;

use legion::*;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;

const POINT_COUNT: usize = 32;
const INSTANCE_DATA_STRIDE: usize = std::mem::size_of::<f32>() * 3;

// space for 50k particles
const INITIAL_BUFFER_SIZE: usize = std::mem::size_of::<f32>() * 3 * 50_000;

fn main() {
    let mut world = World::default();

    let mut resources = Resources::default();
    let mut schedule = Schedule::builder()
        .add_system(sys::update_positions_system())
        .add_system(sys::check_wall_collision_system())
        .build();

    world.push((
        EntityIndex(0),
        Velocity(glam::vec2(30.0, 30.0)),
        Mass(100.0),
    ));

    resources.insert(InstanceCount(1));

    let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();

    glfw.window_hint(WindowHint::ContextVersionMinor(3));
    glfw.window_hint(WindowHint::ContextVersionMajor(3));
    glfw.window_hint(WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

    let (mut window, event) = glfw
        .create_window(WIDTH, HEIGHT, "Rendering text", glfw::WindowMode::Windowed)
        .unwrap();

    resources.insert(window.get_size());

    window.set_key_polling(true);
    window.make_current();

    gl::load_with(|s| window.get_proc_address(s));

    glfw.set_swap_interval(glfw::SwapInterval::None);
    unsafe { gl::Viewport(0, 0, window.get_size().0, window.get_size().1) };
    window.set_size_polling(true);

    let (mut vao, mut vbo, mut ebo) = (0, 0, 0);

    let (vertices, indices) = generate_circle(POINT_COUNT as _);

    unsafe {
        gl::GenVertexArrays(1, &raw mut vao);
        gl::GenBuffers(1, &raw mut vbo);
        gl::GenBuffers(1, &raw mut ebo);

        gl::BindVertexArray(vao);

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);

        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>()) as isize,
            vertices.as_ptr() as _,
            gl::STATIC_DRAW,
        );

        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            (indices.len() * std::mem::size_of::<u32>()) as isize,
            indices.as_ptr() as _,
            gl::STATIC_READ,
        );

        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            2 * std::mem::size_of::<f32>() as i32,
            std::ptr::null(),
        );

        // still bound
    }

    // instancing
    let mut instance_vbo = 0;
    let instance_data = [100.0, 100.0, 100.0];
    let instance_data_ptr;

    unsafe {
        gl::GenBuffers(1, &raw mut instance_vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, instance_vbo);

        gl::BufferStorage(
            gl::ARRAY_BUFFER,
            INITIAL_BUFFER_SIZE as _,
            std::ptr::null(),
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
        );

        instance_data_ptr = gl::MapBufferRange(
            gl::ARRAY_BUFFER,
            0,
            INITIAL_BUFFER_SIZE as _,
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
        ) as *mut f32;

        std::ptr::copy_nonoverlapping(
            instance_data.as_ptr(),
            instance_data_ptr,
            instance_data.len(),
        );

        resources.insert(InstanceDataPtr::new(instance_data_ptr));

        let radius_offset = std::mem::size_of::<f32>() * 2;

        // position
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribDivisor(1, 1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            INSTANCE_DATA_STRIDE as _,
            0 as _,
        );

        // radius
        gl::EnableVertexAttribArray(2);
        gl::VertexAttribDivisor(2, 1);
        gl::VertexAttribPointer(
            2,
            1,
            gl::FLOAT,
            gl::FALSE,
            INSTANCE_DATA_STRIDE as _,
            radius_offset as _,
        );

        // unbind
        gl::BindVertexArray(0);
    }

    let shader = Shader::from_str(include_str!("shader.glsl"), "vertex", "fragment")
        .expect("Failed to load shader");

    let orthographic_uniform = |(width, height)| unsafe {
        shader.use_shader();
        gl::UniformMatrix4fv(
            shader.get_uniform_location("ortho").unwrap(),
            1,
            gl::FALSE,
            glam::Mat4::orthographic_rh_gl(0.0, width as _, height as _, 0.0, -1.0, 1.0)
                .to_cols_array()
                .as_ptr(),
        );
    };

    orthographic_uniform(window.get_size());

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
                unsafe { gl::Viewport(0, 0, width, height) };
                orthographic_uniform((width, height));
                resources.insert(window.get_size());
            }

            _ => {}
        });

        resources.insert(DeltaTime(dt));
        schedule.execute(&mut world, &mut resources);

        unsafe {
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            shader.use_shader();

            gl::BindVertexArray(vao);
            gl::DrawElementsInstanced(
                gl::TRIANGLES,
                indices.len() as _,
                gl::UNSIGNED_INT,
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
