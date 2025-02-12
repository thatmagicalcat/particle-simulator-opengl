use super::*;

pub unsafe fn reallocate_instance_vbo(
    gl: &glow::Context,
    buffer_capacity: usize,
    old_capacity: usize,
    instance_data_ptr: &mut *mut f32,
    instance_vbo: &mut glow::NativeBuffer,
    vao: glow::NativeVertexArray,
) {
    let new_vbo = gl.create_buffer().unwrap();

    gl.bind_buffer(glow::ARRAY_BUFFER, Some(new_vbo));
    gl.buffer_storage(
        glow::ARRAY_BUFFER,
        (buffer_capacity * INSTANCE_DATA_STRIDE) as _,
        None,
        BUFFER_ACCESS_FLAGS,
    );

    let new_ptr = gl.map_buffer_range(
        glow::ARRAY_BUFFER,
        0,
        (buffer_capacity * INSTANCE_DATA_STRIDE) as _,
        BUFFER_ACCESS_FLAGS,
    ) as *mut f32;

    std::ptr::copy_nonoverlapping(
        *instance_data_ptr,
        new_ptr,
        old_capacity * FLOATS_PER_INSTANCE,
    );

    gl.bind_vertex_array(Some(vao));

    gl.bind_buffer(glow::ARRAY_BUFFER, Some(*instance_vbo));
    gl.unmap_buffer(glow::ARRAY_BUFFER);
    gl.delete_buffer(*instance_vbo);

    gl.bind_buffer(glow::ARRAY_BUFFER, Some(new_vbo));

    setup_instance_attributes(gl);

    *instance_data_ptr = new_ptr;
    *instance_vbo = new_vbo;

    // unbind
    // gl.bind_vertex_array(None);
    // gl.bind_buffer(glow::ARRAY_BUFFER, None);
}

pub unsafe fn setup_instance_attributes(gl: &glow::Context) {
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
        (std::mem::size_of::<f32>() * 2) as _,
    );

    // color
    gl.enable_vertex_attrib_array(3);
    gl.vertex_attrib_divisor(3, 1);
    gl.vertex_attrib_pointer_f32(
        3,
        3,
        glow::FLOAT,
        false,
        INSTANCE_DATA_STRIDE as _,
        (std::mem::size_of::<f32>() * 3) as _,
    );
}

pub fn generate_circle(point_count: u32) -> (Vec<f32>, Vec<u32>) {
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

pub fn get_entity<'a>(index: usize, ptr: *mut f32) -> [&'a mut f32; FLOATS_PER_INSTANCE] {
    unsafe {
        let [x, y, r, red, green, blue] = std::slice::from_raw_parts_mut(
            ptr.add(index * FLOATS_PER_INSTANCE),
            FLOATS_PER_INSTANCE,
        ) else {
            std::hint::unreachable_unchecked()
        };

        [x, y, r, red, green, blue]
    }
}

pub fn process_collision(v1: Vec2, v2: Vec2, s1: Vec2, s2: Vec2, m1: f32, m2: f32) -> (Vec2, Vec2) {
    (
        v1 - (2.0 * m2) / (m1 + m2)
            * ((v1 - v2).dot(s1 - s2) / (s1 - s2).length_squared())
            * (s1 - s2),
        v2 - (2.0 * m1) / (m1 + m1)
            * ((v2 - v1).dot(s2 - s1) / (s2 - s1).length_squared())
            * (s2 - s1),
    )
}
