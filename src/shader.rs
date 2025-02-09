#![allow(unused)]

macro_rules! uniform {
    [ $fn_name:tt, $rust_ty:ty, $uniform_fn:ident] => {
        pub fn $fn_name(&self, name: &str, value: $rust_ty) {
            let binding = std::ffi::CString::new(name)
                .expect(concat!("CString::new failed in Shader::set_", stringify!($fn_name)));

            unsafe {
                gl::$uniform_fn(
                    gl::GetUniformLocation(self.0, binding.as_ptr() as _),
                    value as _,
                )
            };
        }
    };
}

pub struct Shader(u32);

impl Shader {
    pub fn from_file(
        path: &str,
        vertex_section: &str,
        fragment_section: &str,
    ) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_str(&contents, vertex_section, fragment_section)
    }

    pub fn from_str(
        contents: &str,
        vertex_section: &str,
        fragment_section: &str,
    ) -> std::io::Result<Self> {
        let parsed = parse_shader(contents);

        let vert = parsed
            .get(vertex_section)
            .ok_or(std::io::ErrorKind::NotFound)?;

        let frag = parsed
            .get(fragment_section)
            .ok_or(std::io::ErrorKind::NotFound)?;

        let vert_shader = compile_shader(gl::VERTEX_SHADER, vert);
        let frag_shader = compile_shader(gl::FRAGMENT_SHADER, frag);

        let program = unsafe { gl::CreateProgram() };

        unsafe {
            gl::AttachShader(program, vert_shader);
            gl::AttachShader(program, frag_shader);
            gl::LinkProgram(program);
            gl::ValidateProgram(program);

            // cleanup
            gl::DeleteShader(vert_shader);
            gl::DeleteShader(frag_shader);
        };

        Ok(Self(program))
    }

    pub fn use_shader(&self) {
        unsafe { gl::UseProgram(self.0) };
    }

    pub fn get_id(&self) -> u32 {
        self.0
    }

    pub fn get_uniform_location(&self, uniform_name: &str) -> Option<i32> {
        let binding = std::ffi::CString::new(uniform_name).expect(concat!(
            "CString::new failed in `get_uniform_location`",
        ));

        let location = unsafe { gl::GetUniformLocation(self.0, binding.as_ptr() as _) };

        (location != -1).then_some(location)
    }

    uniform!(set_bool, bool, Uniform1i);
    uniform!(set_int, i32, Uniform1i);
    uniform!(set_float, f32, Uniform1f);
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.0) };
    }
}

fn parse_shader(input: &str) -> std::collections::HashMap<String, String> {
    let mut section: Option<String> = None;
    let mut code = String::new();
    let mut map = std::collections::HashMap::new();

    for line in input.lines() {
        if line.is_empty() {
            continue;
        }

        if line.trim().starts_with("--") {
            if section.is_some() {
                map.insert(section.take().unwrap(), code);
            }

            section = Some(line.trim_start_matches(['-', ' ']).to_string());
            code = String::new();
        } else {
            code += line;
            code += "\n";
        }
    }

    if let Some(section) = section {
        map.insert(section, code);
    }

    map
}

fn compile_shader(ty: u32, source: &str) -> u32 {
    let id = unsafe { gl::CreateShader(ty) };

    unsafe {
        gl::ShaderSource(
            id,
            1,
            &(source as *const str as *const i8),
            [source.len() as _].as_ptr(),
        );

        gl::CompileShader(id);

        let mut result: i32 = 0;
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &raw mut result);

        if result == gl::FALSE as _ {
            let mut length: i32 = 0;
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &raw mut length);
            let mut message = vec![0; length as _];
            gl::GetShaderInfoLog(id, length, &raw mut length, message.as_mut_ptr() as *mut i8);

            println!(
                "Failed to compile shader!\n{}",
                std::str::from_utf8(message.as_slice()).unwrap()
            );

            gl::DeleteShader(id);
            return 0;
        }
    }

    id
}
