#![allow(unused)]

use glow::HasContext;

pub struct Shader<'a>(&'a glow::Context, glow::NativeProgram);

impl<'a> Shader<'a> {
    pub fn from_file(
        gl: &'a glow::Context,
        path: &str,
        vertex_section: &str,
        fragment_section: &str,
    ) -> std::io::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_str(gl, &contents, vertex_section, fragment_section)
    }

    pub fn from_str(
        gl: &'a glow::Context,
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

        let vert_shader = compile_shader(gl, glow::VERTEX_SHADER, vert);
        let frag_shader = compile_shader(gl, glow::FRAGMENT_SHADER, frag);

        let program = unsafe { gl.create_program().unwrap() };

        unsafe {
            gl.attach_shader(program, vert_shader);
            gl.attach_shader(program, frag_shader);
            gl.link_program(program);
            gl.validate_program(program);

            // cleanup
            gl.delete_shader(vert_shader);
            gl.delete_shader(frag_shader);
        };

        Ok(Self(gl, program))
    }

    pub fn use_shader(&self) {
        unsafe { self.0.use_program(Some(self.1)) };
    }

    pub fn get_shader(&self) -> glow::NativeProgram {
        self.1
    }

    pub fn get_uniform_location(&self, uniform_name: &str) -> Option<glow::UniformLocation> {
        unsafe { self.0.get_uniform_location(self.1, uniform_name) }
    }
}

impl<'a> Drop for Shader<'a> {
    fn drop(&mut self) {
        unsafe { self.0.delete_program(self.1) };
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

fn compile_shader(gl: &glow::Context, ty: u32, source: &str) -> glow::NativeShader {
    let id = unsafe { gl.create_shader(ty).unwrap() };

    unsafe {
        gl.shader_source(id, source);
        gl.compile_shader(id);

        let mut result: i32 = 0;
        let info = gl.get_shader_info_log(id);

        if !info.is_empty() {
            eprintln!("[Info] `compile_shader(...)`: {info}");
        }
    }

    id
}
