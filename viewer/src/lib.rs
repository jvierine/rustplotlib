mod viewport;
pub use viewport::PlotViewport;

mod series;
pub use series::SeriesView;
mod line_plot;
pub use line_plot::LinePlot;
// Browser GPU rendering; JavaScript only boots WASM and connects controls.
use wasm_bindgen::prelude::*;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlProgram, WebGlShader, WebGlTexture,
};

fn error(s: impl Into<String>) -> JsValue {
    JsValue::from_str(&s.into())
}
fn shader(gl: &Gl, kind: u32, source: &str) -> Result<WebGlShader, JsValue> {
    let s = gl
        .create_shader(kind)
        .ok_or_else(|| error("Cannot allocate shader"))?;
    gl.shader_source(&s, source);
    gl.compile_shader(&s);
    if gl.get_shader_parameter(&s, Gl::COMPILE_STATUS).as_bool() != Some(true) {
        return Err(error(gl.get_shader_info_log(&s).unwrap_or_default()));
    }
    Ok(s)
}
fn program(gl: &Gl, frag: &str) -> Result<WebGlProgram, JsValue> {
    let p = gl
        .create_program()
        .ok_or_else(|| error("Cannot allocate program"))?;
    let a = shader(gl, Gl::VERTEX_SHADER, include_str!("fullscreen.vert"))?;
    let b = shader(gl, Gl::FRAGMENT_SHADER, frag)?;
    gl.attach_shader(&p, &a);
    gl.attach_shader(&p, &b);
    gl.link_program(&p);
    gl.delete_shader(Some(&a));
    gl.delete_shader(Some(&b));
    if gl.get_program_parameter(&p, Gl::LINK_STATUS).as_bool() != Some(true) {
        return Err(error(gl.get_program_info_log(&p).unwrap_or_default()));
    }
    Ok(p)
}
fn context(canvas: &HtmlCanvasElement) -> Result<Gl, JsValue> {
    let options = js_sys::Object::new();
    js_sys::Reflect::set(&options, &"preserveDrawingBuffer".into(), &true.into())?;
    Ok(canvas
        .get_context_with_context_options("webgl2", &options)?
        .ok_or_else(|| error("WebGL2 is required"))?
        .dyn_into::<Gl>()?)
}
fn uniform1(gl: &Gl, p: &WebGlProgram, k: &str, v: f32) {
    gl.uniform1f(gl.get_uniform_location(p, k).as_ref(), v);
}
fn uniform_i(gl: &Gl, p: &WebGlProgram, k: &str, v: i32) {
    gl.uniform1i(gl.get_uniform_location(p, k).as_ref(), v);
}
fn uniform2(gl: &Gl, p: &WebGlProgram, k: &str, a: f32, b: f32) {
    gl.uniform2f(gl.get_uniform_location(p, k).as_ref(), a, b);
}
fn custom_colormap(gl: &Gl, p: &WebGlProgram, rgb: &[f32]) -> Result<(), JsValue> {
    if rgb.len() % 3 != 0
        || !(2..=32).contains(&(rgb.len() / 3))
        || !rgb.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))
    {
        return Err(error("Provide 2–32 RGB color stops in [0,1]"));
    }
    gl.use_program(Some(p));
    gl.uniform3fv_with_f32_array(gl.get_uniform_location(p, "colorStops[0]").as_ref(), rgb);
    uniform_i(gl, p, "colorCount", (rgb.len() / 3) as i32);
    Ok(())
}
fn texture(gl: &Gl, target: u32) -> Result<WebGlTexture, JsValue> {
    let t = gl
        .create_texture()
        .ok_or_else(|| error("Cannot allocate texture"))?;
    gl.bind_texture(target, Some(&t));
    for k in [Gl::TEXTURE_MIN_FILTER, Gl::TEXTURE_MAG_FILTER] {
        gl.tex_parameteri(target, k, Gl::NEAREST as i32);
    }
    for k in [Gl::TEXTURE_WRAP_S, Gl::TEXTURE_WRAP_T] {
        gl.tex_parameteri(target, k, Gl::CLAMP_TO_EDGE as i32);
    }
    if target == Gl::TEXTURE_3D {
        gl.tex_parameteri(target, Gl::TEXTURE_WRAP_R, Gl::CLAMP_TO_EDGE as i32);
    }
    Ok(t)
}
fn finish(gl: &Gl, p: &WebGlProgram, c: &HtmlCanvasElement) {
    gl.viewport(0, 0, c.width() as i32, c.height() as i32);
    gl.use_program(Some(p));
    gl.draw_arrays(Gl::TRIANGLES, 0, 3);
}

/// Upload a rectangular field once, then change scales/views without re-upload.
#[wasm_bindgen]
pub struct FieldView {
    gl: Gl,
    canvas: HtmlCanvasElement,
    p: WebGlProgram,
    t: WebGlTexture,
    edge_x: WebGlTexture,
    edge_y: WebGlTexture,
    nx: i32,
    ny: i32,
    max_texture_size: u32,
}
#[wasm_bindgen]
impl FieldView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<FieldView, JsValue> {
        let gl = context(&canvas)?;
        // Query fixed device limits once: getParameter can synchronize the GPU.
        let max_texture_size = gl
            .get_parameter(Gl::MAX_TEXTURE_SIZE)?
            .as_f64()
            .unwrap_or(0.0) as u32;
        let p = program(&gl, include_str!("field.frag"))?;
        let t = texture(&gl, Gl::TEXTURE_2D)?;
        let edge_x = texture(&gl, Gl::TEXTURE_2D)?;
        let edge_y = texture(&gl, Gl::TEXTURE_2D)?;
        let defaults = js_sys::Float32Array::from(&[0.0f32, 1.0][..]);
        for edge in [&edge_x, &edge_y] {
            gl.bind_texture(Gl::TEXTURE_2D, Some(edge));
            gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                Gl::TEXTURE_2D,
                0,
                Gl::R32F as i32,
                2,
                1,
                0,
                Gl::RED,
                Gl::FLOAT,
                Some(&defaults),
            )?;
        }
        gl.use_program(Some(&p));
        uniform_i(&gl, &p, "edgesX", 1);
        uniform_i(&gl, &p, "edgesY", 2);
        uniform_i(&gl, &p, "rectilinear", 0);
        uniform2(&gl, &p, "range", -1.0, 1.0);
        uniform1(&gl, &p, "normWidth", 0.1);
        uniform_i(&gl, &p, "normMode", 0);
        uniform_i(&gl, &p, "palette", 0);
        uniform2(&gl, &p, "viewLow", 0.0, 0.0);
        uniform2(&gl, &p, "viewHigh", 1.0, 1.0);
        Ok(Self {
            gl,
            canvas,
            p,
            t,
            edge_x,
            edge_y,
            nx: 0,
            ny: 0,
            max_texture_size,
        })
    }
    pub fn set_data(&mut self, nx: u32, ny: u32, values: &[f32]) -> Result<(), JsValue> {
        if nx < 1 || ny < 1 || values.len() != nx as usize * ny as usize {
            return Err(error("Field dimensions do not match data"));
        }
        if nx > self.max_texture_size || ny > self.max_texture_size {
            return Err(error("Field exceeds GPU texture limit"));
        }
        self.gl.use_program(Some(&self.p));
        uniform_i(&self.gl, &self.p, "rectilinear", 0);
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&self.t));
        let arr = js_sys::Float32Array::from(values);
        if self.nx == nx as i32 && self.ny == ny as i32 {
            self.gl
                .tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
                    Gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    nx as i32,
                    ny as i32,
                    Gl::RED,
                    Gl::FLOAT,
                    Some(&arr),
                )?;
        } else {
            self.gl
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                Gl::TEXTURE_2D,
                0,
                Gl::R32F as i32,
                nx as i32,
                ny as i32,
                0,
                Gl::RED,
                Gl::FLOAT,
                Some(&arr),
            )?;
        }
        self.nx = nx as i32;
        self.ny = ny as i32;
        self.draw();
        Ok(())
    }
    pub fn set_rectilinear_data(
        &mut self,
        x_edges: &[f32],
        y_edges: &[f32],
        values: &[f32],
    ) -> Result<(), JsValue> {
        for edges in [x_edges, y_edges] {
            if edges.len() < 2
                || edges.iter().any(|x| !x.is_finite())
                || edges.windows(2).any(|w| w[1] <= w[0])
            {
                return Err(error("Cell edges must be finite and strictly increasing"));
            }
        }
        self.set_data(
            (x_edges.len() - 1) as u32,
            (y_edges.len() - 1) as u32,
            values,
        )?;
        let max = self
            .gl
            .get_parameter(Gl::MAX_TEXTURE_SIZE)?
            .as_f64()
            .unwrap_or(0.0) as usize;
        for (unit, t, edges) in [
            (Gl::TEXTURE1, &self.edge_x, x_edges),
            (Gl::TEXTURE2, &self.edge_y, y_edges),
        ] {
            if edges.len() > max {
                return Err(error("Cell edges exceed GPU texture limit"));
            }
            self.gl.active_texture(unit);
            self.gl.bind_texture(Gl::TEXTURE_2D, Some(t));
            let a = js_sys::Float32Array::from(edges);
            self.gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(Gl::TEXTURE_2D,0,Gl::R32F as i32,edges.len()as i32,1,0,Gl::RED,Gl::FLOAT,Some(&a))?;
        }
        uniform_i(&self.gl, &self.p, "rectilinear", 1);
        self.draw();
        Ok(())
    }
    /// Read a rendered pixel (bottom-left origin), useful for renderer checks.
    pub fn pixel(&self, x: i32, y: i32) -> Result<Vec<u8>, JsValue> {
        if x < 0 || y < 0 || x >= self.canvas.width() as i32 || y >= self.canvas.height() as i32 {
            return Err(error("Pixel outside canvas"));
        }
        self.draw();
        let mut rgba = vec![0u8; 4];
        self.gl.read_pixels_with_opt_u8_array(
            x,
            y,
            1,
            1,
            Gl::RGBA,
            Gl::UNSIGNED_BYTE,
            Some(&mut rgba),
        )?;
        Ok(rgba)
    }
    /// mode: 0 linear, 1 logarithmic, 2 signed asinh; palette: 0 diverging, 1 sequential, 2 ember.
    pub fn set_scale(
        &self,
        low: f32,
        high: f32,
        mode: i32,
        width: f32,
        palette: i32,
    ) -> Result<(), JsValue> {
        if !low.is_finite()
            || !high.is_finite()
            || high <= low
            || !(0..=2).contains(&mode)
            || !(0..=2).contains(&palette)
            || (mode == 1 && low <= 0.0)
            || (mode == 2 && (!width.is_finite() || width <= 0.0))
        {
            return Err(error("Invalid color scale"));
        }
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "range", low, high);
        uniform_i(&self.gl, &self.p, "normMode", mode);
        uniform1(&self.gl, &self.p, "normWidth", width);
        uniform_i(&self.gl, &self.p, "palette", palette);
        uniform_i(&self.gl, &self.p, "colorCount", 0);
        self.draw();
        Ok(())
    }
    pub fn set_colormap(&self, rgb: &[f32]) -> Result<(), JsValue> {
        custom_colormap(&self.gl, &self.p, rgb)?;
        self.draw();
        Ok(())
    }
    pub fn set_view(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> Result<(), JsValue> {
        if ![x0, y0, x1, y1].iter().all(|x| x.is_finite()) || x1 <= x0 || y1 <= y0 {
            return Err(error("Invalid view"));
        }
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "viewLow", x0, y0);
        uniform2(&self.gl, &self.p, "viewHigh", x1, y1);
        self.draw();
        Ok(())
    }
    pub fn draw(&self) {
        self.gl.use_program(Some(&self.p));
        self.gl.active_texture(Gl::TEXTURE1);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&self.edge_x));
        self.gl.active_texture(Gl::TEXTURE2);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&self.edge_y));
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&self.t));
        finish(&self.gl, &self.p, &self.canvas);
    }
    pub fn renderer(&self) -> String {
        "Rust/WASM · WebGL2 R32F field texture".into()
    }
}
impl Drop for FieldView {
    fn drop(&mut self) {
        self.gl.delete_texture(Some(&self.t));
        self.gl.delete_texture(Some(&self.edge_x));
        self.gl.delete_texture(Some(&self.edge_y));
        self.gl.delete_program(Some(&self.p));
    }
}

#[wasm_bindgen]
pub struct VolumeView {
    auto_draw: std::cell::Cell<bool>,
    gl: Gl,
    canvas: HtmlCanvasElement,
    p: WebGlProgram,
    t: WebGlTexture,
}
#[wasm_bindgen]
impl VolumeView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<VolumeView, JsValue> {
        let gl = context(&canvas)?;
        let p = program(&gl, include_str!("volume.frag"))?;
        let t = texture(&gl, Gl::TEXTURE_3D)?;
        gl.use_program(Some(&p));
        uniform2(&gl, &p, "camera", 0.45, 0.25);
        uniform_i(&gl, &p, "shellMode", 0);
        uniform1(&gl, &p, "density", 2.0);
        uniform1(&gl, &p, "cut", 0.04);
        uniform2(&gl, &p, "range", 0.0, 1.0);
        gl.uniform3f(
            gl.get_uniform_location(&p, "boxExtent").as_ref(),
            1.0,
            1.0,
            1.0,
        );
        Ok(Self {
            auto_draw: std::cell::Cell::new(true),
            gl,
            canvas,
            p,
            t,
        })
    }
    pub fn set_data(&self, nx: u32, ny: u32, nz: u32, values: &[f32]) -> Result<(), JsValue> {
        if [nx, ny, nz].iter().any(|x| *x < 2)
            || values.len() != nx as usize * ny as usize * nz as usize
        {
            return Err(error("Volume dimensions do not match data"));
        }
        let max = self
            .gl
            .get_parameter(Gl::MAX_3D_TEXTURE_SIZE)?
            .as_f64()
            .unwrap_or(0.0) as u32;
        if [nx, ny, nz].iter().any(|x| *x > max) {
            return Err(error("Volume exceeds GPU texture limit"));
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(error("Volume data must be finite; mask before upload"));
        }
        self.gl.bind_texture(Gl::TEXTURE_3D, Some(&self.t));
        let arr = js_sys::Float32Array::from(values);
        self.gl.tex_image_3d_with_opt_array_buffer_view(
            Gl::TEXTURE_3D,
            0,
            Gl::R32F as i32,
            nx as i32,
            ny as i32,
            nz as i32,
            0,
            Gl::RED,
            Gl::FLOAT,
            Some(&arr),
        )?;
        self.draw();
        Ok(())
    }
    pub fn set_camera(&self, yaw: f32, pitch: f32) {
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "camera", yaw, pitch.clamp(-1.45, 1.45));
        self.draw();
    }
    pub fn set_colormap(&self, rgb: &[f32]) -> Result<(), JsValue> {
        custom_colormap(&self.gl, &self.p, rgb)?;
        self.draw();
        Ok(())
    }
    /// Preserve physical box proportions instead of stretching data to a cube.
    pub fn set_box_extent(&self, x: f32, y: f32, z: f32) -> Result<(), JsValue> {
        if ![x, y, z].iter().all(|v| v.is_finite() && *v > 0.0) {
            return Err(error("Invalid physical box dimensions"));
        }
        let longest = x.max(y).max(z);
        self.gl.use_program(Some(&self.p));
        self.gl.uniform3f(
            self.gl.get_uniform_location(&self.p, "boxExtent").as_ref(),
            x / longest,
            y / longest,
            z / longest,
        );
        self.draw();
        Ok(())
    }
    /// Render a spherical atmosphere with a quadrant removed. Radii are in
    /// the unit texture cube (maximum radius 0.5); data remain Cartesian.
    pub fn set_spherical_shell(
        &self,
        inner: f32,
        outer: f32,
        enabled: bool,
    ) -> Result<(), JsValue> {
        if !inner.is_finite() || !outer.is_finite() || inner <= 0.0 || outer <= inner || outer > 0.5
        {
            return Err(error("Shell radii require 0 < inner < outer <= 0.5"));
        }
        self.gl.use_program(Some(&self.p));
        uniform_i(&self.gl, &self.p, "shellMode", i32::from(enabled));
        uniform2(&self.gl, &self.p, "shellRadii", inner, outer);
        self.draw();
        Ok(())
    }
    pub fn set_transfer(&self, low: f32, high: f32, density: f32, cut: f32) -> Result<(), JsValue> {
        if ![low, high, density, cut].iter().all(|v| v.is_finite())
            || high <= low
            || density < 0.0
            || !(0.0..=1.0).contains(&cut)
        {
            return Err(error("Invalid transfer function"));
        }
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "range", low, high);
        uniform1(&self.gl, &self.p, "density", density);
        uniform1(&self.gl, &self.p, "cut", cut);
        self.draw();
        Ok(())
    }
    /// Batch data, transfer and camera changes before one GPU draw.
    pub fn set_auto_draw(&self, enabled: bool) {
        self.auto_draw.set(enabled);
    }
    pub fn draw(&self) {
        if !self.auto_draw.get() {
            return;
        }
        self.gl.use_program(Some(&self.p));
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_3D, Some(&self.t));
        uniform1(
            &self.gl,
            &self.p,
            "aspect",
            self.canvas.width() as f32 / self.canvas.height() as f32,
        );
        finish(&self.gl, &self.p, &self.canvas);
    }
    pub fn renderer(&self) -> String {
        "Rust/WASM · WebGL2 R32F volume ray marcher".into()
    }
}
impl Drop for VolumeView {
    fn drop(&mut self) {
        self.gl.delete_texture(Some(&self.t));
        self.gl.delete_program(Some(&self.p));
    }
}

/// Colored 3-D line geometry, e.g. traced vector fields. Coordinates stay in
/// physical units; the camera centre and extent use those same units.
#[wasm_bindgen]
pub struct PathView {
    gl: Gl,
    canvas: HtmlCanvasElement,
    p: WebGlProgram,
    paths: web_sys::WebGlBuffer,
    earth: web_sys::WebGlBuffer,
    count: i32,
    earth_count: i32,
}
#[wasm_bindgen]
impl PathView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<PathView, JsValue> {
        let gl = context(&canvas)?;
        let p = gl
            .create_program()
            .ok_or_else(|| error("Cannot allocate path program"))?;
        let a = shader(&gl, Gl::VERTEX_SHADER, include_str!("path.vert"))?;
        let b = shader(&gl, Gl::FRAGMENT_SHADER, include_str!("path.frag"))?;
        gl.attach_shader(&p, &a);
        gl.attach_shader(&p, &b);
        gl.link_program(&p);
        if gl.get_program_parameter(&p, Gl::LINK_STATUS).as_bool() != Some(true) {
            return Err(error(gl.get_program_info_log(&p).unwrap_or_default()));
        }
        gl.delete_shader(Some(&a));
        gl.delete_shader(Some(&b));
        let paths = gl
            .create_buffer()
            .ok_or_else(|| error("Cannot allocate paths"))?;
        let earth = gl
            .create_buffer()
            .ok_or_else(|| error("Cannot allocate Earth"))?;
        let mut sphere = Vec::new();
        let point = |i: usize, j: usize| {
            let a = std::f32::consts::TAU * i as f32 / 64.;
            let b = std::f32::consts::PI * (j as f32 / 32. - 0.5);
            [b.cos() * a.cos(), b.cos() * a.sin(), b.sin()]
        };
        for j in 0..32 {
            for i in 0..64 {
                for q in [
                    point(i, j),
                    point(i + 1, j),
                    point(i + 1, j + 1),
                    point(i, j),
                    point(i + 1, j + 1),
                    point(i, j + 1),
                ] {
                    let light = (0.35 + 0.65 * (q[0] * 0.6 + q[1] * 0.1 + q[2] * 0.7).max(0.))
                        .clamp(0.0, 1.0);
                    sphere.extend_from_slice(&q);
                    sphere.extend_from_slice(&[0.22 * light, 0.45 * light, 0.58 * light]);
                }
            }
        }
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&earth));
        let arr = js_sys::Float32Array::from(sphere.as_slice());
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &arr, Gl::STATIC_DRAW);
        gl.use_program(Some(&p));
        uniform2(&gl, &p, "camera", -0.9, 0.45);
        uniform1(&gl, &p, "extent", 34.);
        gl.uniform3f(gl.get_uniform_location(&p, "centre").as_ref(), -7., 0., 0.);
        Ok(Self {
            gl,
            canvas,
            p,
            paths,
            earth,
            count: 0,
            earth_count: (sphere.len() / 6) as i32,
        })
    }
    pub fn set_paths(&mut self, xyz_rgb: &[f32]) -> Result<(), JsValue> {
        if xyz_rgb.len() % 12 != 0 || !xyz_rgb.iter().all(|x| x.is_finite()) {
            return Err(error("Line segments require two XYZ/RGB vertices each"));
        }
        self.gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.paths));
        let arr = js_sys::Float32Array::from(xyz_rgb);
        self.gl
            .buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &arr, Gl::DYNAMIC_DRAW);
        self.count = (xyz_rgb.len() / 6) as i32;
        self.draw();
        Ok(())
    }
    pub fn set_camera(&self, yaw: f32, pitch: f32, extent: f32) -> Result<(), JsValue> {
        if ![yaw, pitch, extent].iter().all(|v| v.is_finite()) || extent <= 0.0 {
            return Err(error("Invalid path camera"));
        }
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "camera", yaw, pitch);
        uniform1(&self.gl, &self.p, "extent", extent);
        self.draw();
        Ok(())
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
        uniform1(
            gl,
            &self.p,
            "aspect",
            self.canvas.width() as f32 / self.canvas.height() as f32,
        );
        gl.enable(Gl::DEPTH_TEST);
        gl.clear_color(1., 1., 1., 1.);
        gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        for (buffer, count, mode) in [
            (&self.earth, self.earth_count, Gl::TRIANGLES),
            (&self.paths, self.count, Gl::LINES),
        ] {
            gl.bind_buffer(Gl::ARRAY_BUFFER, Some(buffer));
            for (name, offset) in [("position", 0), ("rgb", 12)] {
                let a = gl.get_attrib_location(&self.p, name) as u32;
                gl.enable_vertex_attrib_array(a);
                gl.vertex_attrib_pointer_with_i32(a, 3, Gl::FLOAT, false, 24, offset);
            }
            gl.draw_arrays(mode, 0, count);
        }
        gl.disable(Gl::DEPTH_TEST);
    }
}

/// Geographic RGBA layers on a rotatable globe. Input rows run south to north
/// and columns run west to east from -180 to 180 degrees.
#[wasm_bindgen]
pub struct GlobeView {
    gl: Gl,
    canvas: HtmlCanvasElement,
    p: WebGlProgram,
    t: WebGlTexture,
}
#[wasm_bindgen]
impl GlobeView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<GlobeView, JsValue> {
        let gl = context(&canvas)?;
        let p = program(&gl, include_str!("globe.frag"))?;
        let t = texture(&gl, Gl::TEXTURE_2D)?;
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
        gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::REPEAT as i32);
        gl.use_program(Some(&p));
        uniform2(&gl, &p, "camera", -0.9, 0.9);
        Ok(Self { gl, canvas, p, t })
    }
    pub fn set_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> Result<(), JsValue> {
        if width == 0 || height == 0 || rgba.len() != width as usize * height as usize * 4 {
            return Err(error("Invalid globe image dimensions"));
        }
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&self.t));
        self.gl
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                Gl::TEXTURE_2D,
                0,
                Gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                Gl::RGBA,
                Gl::UNSIGNED_BYTE,
                Some(rgba),
            )?;
        self.draw();
        Ok(())
    }
    pub fn set_camera(&self, yaw: f32, pitch: f32) -> Result<(), JsValue> {
        if !yaw.is_finite() || !pitch.is_finite() {
            return Err(error("Invalid globe camera"));
        }
        self.gl.use_program(Some(&self.p));
        uniform2(&self.gl, &self.p, "camera", yaw, pitch);
        self.draw();
        Ok(())
    }
    pub fn draw(&self) {
        self.gl.use_program(Some(&self.p));
        self.gl.active_texture(Gl::TEXTURE0);
        self.gl.bind_texture(Gl::TEXTURE_2D, Some(&self.t));
        uniform1(
            &self.gl,
            &self.p,
            "aspect",
            self.canvas.width() as f32 / self.canvas.height() as f32,
        );
        finish(&self.gl, &self.p, &self.canvas);
    }
}

mod mesh;
pub use mesh::MeshView;
