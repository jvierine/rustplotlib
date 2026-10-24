//! Indexed 3-D meshes rendered by WebGL2 with a perspective camera and depth buffer.
//! World coordinates remain on the GPU; camera changes never re-upload geometry.
use crate::{context, error, shader, texture, uniform_i, uniform1, uniform2};
use wasm_bindgen::prelude::*;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram, WebGlTexture,
    WebGlVertexArrayObject,
};

fn vertex_data(xyz: &[f32], triangles: &[u32], values: &[f32]) -> Result<Vec<f32>, String> {
    let n = xyz.len() / 3;
    if xyz.len() % 3 != 0
        || n < 3
        || values.len() != n
        || triangles.is_empty()
        || triangles.len() % 3 != 0
        || !xyz.iter().chain(values).all(|v| v.is_finite())
        || triangles.iter().any(|&i| i as usize >= n)
    {
        return Err("Mesh requires finite XYZ/scalars and valid triangle indices".into());
    }
    let mut normals = vec![[0.; 3]; n];
    for t in triangles.chunks_exact(3) {
        let p = |i: u32| {
            [
                xyz[3 * i as usize],
                xyz[3 * i as usize + 1],
                xyz[3 * i as usize + 2],
            ]
        };
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in t {
            for k in 0..3 {
                normals[i as usize][k] += cross[k];
            }
        }
    }
    let mut out = Vec::with_capacity(n * 10);
    for i in 0..n {
        let norm = normals[i].iter().map(|v| v * v).sum::<f32>().sqrt();
        let normal = if norm > 1e-20 {
            normals[i].map(|v| v / norm)
        } else {
            [0., 0., 1.]
        };
        out.extend_from_slice(&xyz[3 * i..3 * i + 3]);
        out.extend_from_slice(&normal);
        out.extend_from_slice(&[values[i], 0., 0., 0.]);
    }
    Ok(out)
}

fn projected(
    p: [f32; 3],
    centre: [f32; 3],
    scale: [f32; 3],
    radius: f32,
    camera: [f32; 3],
    aspect: f32,
) -> [f32; 3] {
    let [a, b, d] = camera;
    let front = [a.cos() * b.cos(), a.sin() * b.cos(), b.sin()];
    let right = [-a.sin(), a.cos(), 0.];
    let up = [-a.cos() * b.sin(), -a.sin() * b.sin(), b.cos()];
    let q = std::array::from_fn::<_, 3, _>(|k| (p[k] - centre[k]) * scale[k] / radius);
    let dot = |v: [f32; 3]| (0..3).map(|k| q[k] * v[k]).sum::<f32>();
    let z = d - dot(front);
    [
        0.5 + 0.5 * dot(right) * 2.4142137 / (aspect * z),
        0.5 - 0.5 * dot(up) * 2.4142137 / z,
        z,
    ]
}

#[wasm_bindgen]
pub struct MeshView {
    gl: Gl,
    canvas: HtmlCanvasElement,
    p: WebGlProgram,
    vao: WebGlVertexArrayObject,
    lines_vao: WebGlVertexArrayObject,
    vertices: WebGlBuffer,
    indices: WebGlBuffer,
    lines: WebGlBuffer,
    colors: WebGlTexture,
    count: i32,
    line_count: i32,
    centre: [f32; 3],
    scale: [f32; 3],
    radius: f32,
    extent: [f32; 3],
    camera: [f32; 3],
}
#[wasm_bindgen]
impl MeshView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<MeshView, JsValue> {
        let gl = context(&canvas)?;
        let p = gl
            .create_program()
            .ok_or_else(|| error("Cannot allocate mesh program"))?;
        let vs = shader(&gl, Gl::VERTEX_SHADER, include_str!("mesh.vert"))?;
        let fs = shader(&gl, Gl::FRAGMENT_SHADER, include_str!("mesh.frag"))?;
        gl.attach_shader(&p, &vs);
        gl.attach_shader(&p, &fs);
        gl.link_program(&p);
        gl.delete_shader(Some(&vs));
        gl.delete_shader(Some(&fs));
        if gl.get_program_parameter(&p, Gl::LINK_STATUS).as_bool() != Some(true) {
            return Err(error(gl.get_program_info_log(&p).unwrap_or_default()));
        }
        let vao = gl
            .create_vertex_array()
            .ok_or_else(|| error("Cannot allocate mesh VAO"))?;
        let lines_vao = gl
            .create_vertex_array()
            .ok_or_else(|| error("Cannot allocate line VAO"))?;
        let vertices = gl
            .create_buffer()
            .ok_or_else(|| error("Cannot allocate mesh vertices"))?;
        let indices = gl
            .create_buffer()
            .ok_or_else(|| error("Cannot allocate mesh indices"))?;
        let lines = gl
            .create_buffer()
            .ok_or_else(|| error("Cannot allocate mesh lines"))?;
        let colors = texture(&gl, Gl::TEXTURE_2D)?;
        let view = Self {
            gl,
            canvas,
            p,
            vao,
            lines_vao,
            vertices,
            indices,
            lines,
            colors,
            count: 0,
            line_count: 0,
            centre: [0.; 3],
            scale: [1.; 3],
            radius: 1.,
            extent: [1.; 3],
            camera: [-0.8, 0.55, 3.2],
        };
        view.gl.use_program(Some(&view.p));
        uniform2(&view.gl, &view.p, "range", 0., 1.);
        view.set_colormap(&[0.12, 0.3, 0.45, 0.8, 0.9, 0.85])?;
        Ok(view)
    }
    /// XYZ positions and scalar values at vertices; three indices per triangle.
    pub fn set_mesh(
        &mut self,
        xyz: &[f32],
        triangles: &[u32],
        values: &[f32],
    ) -> Result<(), JsValue> {
        let data = vertex_data(xyz, triangles, values).map_err(error)?;
        self.gl.bind_vertex_array(Some(&self.vao));
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.vertices));
        let v = js_sys::Float32Array::from(data.as_slice());
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &v, Gl::STATIC_DRAW);
        self.attributes();
        self.gl
            .bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, Some(&self.indices));
        let i = js_sys::Uint32Array::from(triangles);
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ELEMENT_ARRAY_BUFFER, &i, Gl::STATIC_DRAW);
        self.count = triangles.len() as i32;
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for p in xyz.chunks_exact(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        self.set_bounds(lo[0], hi[0], lo[1], hi[1], lo[2], hi[2])?;
        Ok(())
    }
    /// Per-vertex RGB for physical surfaces and markers; all share the depth buffer.
    pub fn set_colored_mesh(
        &mut self,
        xyz: &[f32],
        triangles: &[u32],
        rgb: &[f32],
    ) -> Result<(), JsValue> {
        if rgb.len() != xyz.len() || !rgb.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v)) {
            return Err(error("Vertex colors must be finite RGB values in [0,1]"));
        }
        let values = vec![-1e30; xyz.len() / 3];
        let mut data = vertex_data(xyz, triangles, &values).map_err(error)?;
        for (v, c) in data.chunks_exact_mut(10).zip(rgb.chunks_exact(3)) {
            v[7..10].copy_from_slice(c);
        }
        self.set_mesh(xyz, triangles, &values)?;
        self.gl.bind_vertex_array(Some(&self.vao));
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.vertices));
        let v = js_sys::Float32Array::from(data.as_slice());
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &v, Gl::STATIC_DRAW);
        self.draw();
        Ok(())
    }
    /// Two XYZ/RGB vertices per segment. Lines share the mesh depth buffer.
    pub fn set_lines(&mut self, xyz_rgb: &[f32]) -> Result<(), JsValue> {
        if xyz_rgb.len() % 12 != 0 || !xyz_rgb.iter().all(|v| v.is_finite()) {
            return Err(error("Line segments require finite XYZ/RGB pairs"));
        }
        let mut data = Vec::new();
        for p in xyz_rgb.chunks_exact(6) {
            data.extend_from_slice(&p[..3]);
            data.extend_from_slice(&[0., 0., 1., 0.]);
            data.extend_from_slice(&p[3..]);
        }
        self.gl.bind_vertex_array(Some(&self.lines_vao));
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.lines));
        let a = js_sys::Float32Array::from(data.as_slice());
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &a, Gl::DYNAMIC_DRAW);
        self.attributes();
        self.line_count = (xyz_rgb.len() / 6) as i32;
        self.draw();
        Ok(())
    }
    /// Same units as XYZ; useful when multiple meshes must share a fixed view.
    pub fn set_bounds(
        &mut self,
        x0: f32,
        x1: f32,
        y0: f32,
        y1: f32,
        z0: f32,
        z1: f32,
    ) -> Result<(), JsValue> {
        if ![x0, x1, y0, y1, z0, z1].iter().all(|v| v.is_finite())
            || x1 <= x0
            || y1 <= y0
            || z1 <= z0
        {
            return Err(error("Mesh bounds must be finite and increasing"));
        }
        self.centre = [(x0 + x1) * 0.5, (y0 + y1) * 0.5, (z0 + z1) * 0.5];
        self.extent = [x1 - x0, y1 - y0, z1 - z0];
        self.update_radius();
        self.draw();
        Ok(())
    }
    /// Explicit axis scaling for graphs whose vertical coordinate is a quantity,
    /// rather than a third physical distance. Physical meshes keep [1,1,1].
    pub fn set_axis_scale(&mut self, x: f32, y: f32, z: f32) -> Result<(), JsValue> {
        if ![x, y, z].iter().all(|v| v.is_finite() && *v > 0.) {
            return Err(error("Invalid mesh axis scale"));
        }
        self.scale = [x, y, z];
        self.update_radius();
        self.draw();
        Ok(())
    }
    pub fn set_camera(&mut self, yaw: f32, pitch: f32, distance: f32) -> Result<(), JsValue> {
        if ![yaw, pitch, distance].iter().all(|v| v.is_finite())
            || !(1.2..=15.).contains(&distance)
            || pitch.abs() > 1.55
        {
            return Err(error("Invalid mesh camera"));
        }
        self.camera = [yaw, pitch, distance];
        self.draw();
        Ok(())
    }
    pub fn set_scale(&self, low: f32, high: f32) -> Result<(), JsValue> {
        if !low.is_finite() || !high.is_finite() || high <= low {
            return Err(error("Invalid mesh color range"));
        }
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "range", low, high);
        self.draw();
        Ok(())
    }
    pub fn set_colormap(&self, rgb: &[f32]) -> Result<(), JsValue> {
        if rgb.len() % 3 != 0
            || !(2..=4096).contains(&(rgb.len() / 3))
            || !rgb.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Err(error("Provide 2–4096 RGB color stops in [0,1]"));
        }
        let rgba: Vec<u8> = rgb
            .chunks_exact(3)
            .flat_map(|p| {
                [
                    (p[0] * 255.).round() as u8,
                    (p[1] * 255.).round() as u8,
                    (p[2] * 255.).round() as u8,
                    255,
                ]
            })
            .collect();
        let gl = &self.gl;
        gl.use_program(Some(&self.p));
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.colors));
        for k in [Gl::TEXTURE_MIN_FILTER, Gl::TEXTURE_MAG_FILTER] {
            gl.tex_parameteri(Gl::TEXTURE_2D, k, Gl::LINEAR as i32);
        }
        gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            0,
            Gl::RGBA as i32,
            (rgb.len() / 3) as i32,
            1,
            0,
            Gl::RGBA,
            Gl::UNSIGNED_BYTE,
            Some(&rgba),
        )?;
        uniform_i(gl, &self.p, "colormap", 0);
        uniform1(gl, &self.p, "colorCount", (rgb.len() / 3) as f32);
        self.draw();
        Ok(())
    }
    /// Normalized canvas x/y and positive camera depth. Uses the exact shader transform.
    pub fn project(&self, x: f32, y: f32, z: f32) -> Vec<f32> {
        projected(
            [x, y, z],
            self.centre,
            self.scale,
            self.radius,
            self.camera,
            self.canvas.width() as f32 / self.canvas.height().max(1) as f32,
        )
        .to_vec()
    }
    pub fn draw(&self) {
        let gl = &self.gl;
        gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        gl.use_program(Some(&self.p));
        gl.uniform3fv_with_f32_array(
            gl.get_uniform_location(&self.p, "centre").as_ref(),
            &self.centre,
        );
        gl.uniform3fv_with_f32_array(
            gl.get_uniform_location(&self.p, "axisScale").as_ref(),
            &self.scale,
        );
        gl.uniform3fv_with_f32_array(
            gl.get_uniform_location(&self.p, "camera").as_ref(),
            &self.camera,
        );
        uniform1(gl, &self.p, "radius", self.radius);
        uniform1(
            gl,
            &self.p,
            "aspect",
            self.canvas.width() as f32 / self.canvas.height().max(1) as f32,
        );
        gl.active_texture(Gl::TEXTURE0);
        gl.bind_texture(Gl::TEXTURE_2D, Some(&self.colors));
        gl.enable(Gl::DEPTH_TEST);
        gl.depth_func(Gl::LEQUAL);
        gl.disable(Gl::BLEND);
        gl.disable(Gl::CULL_FACE);
        gl.clear_color(1., 1., 1., 1.);
        gl.clear_depth(1.);
        gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        gl.bind_vertex_array(Some(&self.vao));
        uniform_i(gl, &self.p, "lineMode", 0);
        if self.count > 0 {
            gl.draw_elements_with_i32(Gl::TRIANGLES, self.count, Gl::UNSIGNED_INT, 0);
        }
        gl.bind_vertex_array(Some(&self.lines_vao));
        uniform_i(gl, &self.p, "lineMode", 1);
        if self.line_count > 0 {
            gl.draw_arrays(Gl::LINES, 0, self.line_count);
        }
        gl.bind_vertex_array(None);
    }
    pub fn renderer(&self) -> String {
        "Rust/WASM · WebGL2 indexed triangle mesh with depth testing".into()
    }
}
impl MeshView {
    fn update_radius(&mut self) {
        self.radius = self
            .extent
            .iter()
            .zip(self.scale)
            .map(|(e, s)| (e * s).powi(2))
            .sum::<f32>()
            .sqrt()
            * 0.5;
    }
    fn attributes(&self) {
        for (location, size, offset) in [(0, 3, 0), (1, 3, 12), (2, 1, 24), (3, 3, 28)] {
            self.gl.enable_vertex_attrib_array(location);
            self.gl
                .vertex_attrib_pointer_with_i32(location, size, Gl::FLOAT, false, 40, offset);
        }
    }
}
impl Drop for MeshView {
    fn drop(&mut self) {
        self.gl.delete_buffer(Some(&self.vertices));
        self.gl.delete_buffer(Some(&self.indices));
        self.gl.delete_buffer(Some(&self.lines));
        self.gl.delete_texture(Some(&self.colors));
        self.gl.delete_vertex_array(Some(&self.vao));
        self.gl.delete_vertex_array(Some(&self.lines_vao));
        self.gl.delete_program(Some(&self.p));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn surface_normals_and_masked_triangle_validation() {
        let p = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
        let d = vertex_data(&p, &[0, 1, 2], &[0., 1., 2.]).unwrap();
        for v in d.chunks_exact(10) {
            assert_eq!(&v[3..6], &[0., 0., 1.]);
        }
        assert!(vertex_data(&p, &[0, 1, 3], &[0., 1., 2.]).is_err());
        assert!(vertex_data(&p, &[0, 1, 2], &[0., f32::NAN, 2.]).is_err());
    }
    #[test]
    fn perspective_projects_nearer_points_larger_and_matches_camera_axes() {
        let c = [0.; 3];
        let s = [1.; 3];
        let cam = [0., 0., 3.];
        let far = projected([0., 1., 0.], c, s, 1., cam, 1.);
        let near = projected([1., 1., 0.], c, s, 1., cam, 1.);
        assert!(near[0] > 0.5 && near[0] > far[0] && near[2] < far[2]);
        let top = projected([0., 0., 1.], c, s, 1., cam, 1.);
        assert!(top[1] < 0.5);
        let centre = projected(c, c, s, 1., cam, 1.);
        assert_eq!(centre, [0.5, 0.5, 3.]);
    }
}
