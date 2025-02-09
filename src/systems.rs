use std::hint::unreachable_unchecked;

use super::*;

fn get_current_entity<'a>(index: usize, ptr: *mut f32) -> [&'a mut f32; 3] {
    unsafe {
        let [x, y, r] = std::slice::from_raw_parts_mut(ptr.add(index * FLOATS_PER_INSTANCE), 3)
        else {
            unreachable_unchecked()
        };

        [x, y, r]
    }
}

#[system(for_each)]
pub fn update_positions(
    EntityIndex(index): &EntityIndex,
    Velocity(vel): &Velocity,
    #[resource] ptr: &InstanceDataPtr,
    #[resource] DeltaTime(dt): &DeltaTime,
) {
    let dt = *dt;
    let [pos_x, pos_y, _] = get_current_entity(*index, ptr.get_ptr());

    *pos_x += vel.x * dt;
    *pos_y += vel.y * dt;
}

#[system(for_each)]
pub fn check_wall_collision(
    EntityIndex(index): &EntityIndex,
    vel: &mut Velocity,
    #[resource] size: &(i32, i32),
    #[resource] ptr: &InstanceDataPtr,
) {
    let [pos_x, pos_y, radius] = get_current_entity(*index, ptr.get_ptr());

    if *pos_x - *radius < 0.0 {
        vel.0.x *= -1.0;
        *pos_x = *radius;
    } else if *pos_x + *radius >= size.0 as f32 {
        vel.0.x *= -1.0;
        *pos_x = size.0 as f32 - *radius;
    }

    if *pos_y - *radius < 0.0 {
        vel.0.y *= -1.0;
        *pos_y = *radius;
    } else if *pos_y + *radius >= size.1 as f32 {
        vel.0.y *= -1.0;
        *pos_y = size.1 as f32 - *radius;
    }
}
