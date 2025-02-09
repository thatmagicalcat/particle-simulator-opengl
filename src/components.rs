#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceCount(pub i32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceDataPtr(usize);

impl InstanceDataPtr {
    pub fn new(ptr: *mut f32) -> Self {
        Self(ptr as _)
    }

    pub fn get_ptr(&self) -> *mut f32 {
        self.0 as _
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity(pub glam::Vec2);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntityIndex(pub usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mass(pub f32);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeltaTime(pub f32);
