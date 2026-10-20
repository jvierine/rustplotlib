//! GPU line/scatter primitives for scientific browser panels and geographic views.
//! Axis labels may be laid out in accessible HTML beside the canvas.
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram};

#[wasm_bindgen]
pub struct SeriesGlView {
    gl: Gl,
    canvas: HtmlCanvasElement,
    program: WebGlProgram,
    buffer: WebGlBuffer,
    dpr: f32,
    bounds: [f32; 4],
    viewport_css: [f32; 2],
}
#[wasm_bindgen]
impl SeriesGlView {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let gl = super::context(&canvas)?;
        let p = gl
            .create_program()
            .ok_or_else(|| super::error("Cannot allocate series program"))?;
        let vertex = super::shader(
            &gl,
            Gl::VERTEX_SHADER,
            r#"#version 300 es
precision highp float;
in vec2 position;
uniform vec4 bounds;
uniform float pointSize;
void main(){vec2 uv=(position-bounds.xy)/(bounds.zw-bounds.xy);gl_Position=vec4(uv*2.-1.,0.,1.);gl_PointSize=pointSize;}"#,
        )?;
        let fragment = super::shader(
            &gl,
            Gl::FRAGMENT_SHADER,
            r#"#version 300 es
precision highp float;
uniform vec4 color;
uniform bool roundPoint;
out vec4 rgba;
void main(){if(roundPoint&&length(gl_PointCoord-.5)>.5)discard;rgba=color;}"#,
        )?;
        gl.attach_shader(&p, &vertex);
        gl.attach_shader(&p, &fragment);
        gl.link_program(&p);
        gl.delete_shader(Some(&vertex));
        gl.delete_shader(Some(&fragment));
        if gl.get_program_parameter(&p, Gl::LINK_STATUS).as_bool() != Some(true) {
            return Err(super::error(
                gl.get_program_info_log(&p).unwrap_or_default(),
            ));
        }
        let buffer = gl
            .create_buffer()
            .ok_or_else(|| super::error("Cannot allocate series buffer"))?;
        Ok(Self {
            gl,
            canvas,
            program: p,
            buffer,
            dpr: 1.,
            bounds: [0., 0., 1., 1.],
            viewport_css: [1., 1.],
        })
    }
    /// Bounds are x0,y0,x1,y1. Coordinates can already be transformed to log10.
    pub fn begin(
        &mut self,
        width: f32,
        height: f32,
        dpr: f32,
        bounds: &[f32],
        background: &[f32],
    ) -> Result<(), JsValue> {
        if ![width, height, dpr]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            || bounds.len() != 4
            || !bounds.iter().all(|v| v.is_finite())
            || bounds[2] <= bounds[0]
            || bounds[3] <= bounds[1]
            || background.len() != 4
            || !background
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        {
            return Err(super::error(
                "Invalid scientific panel dimensions, bounds or colour",
            ));
        }
        self.dpr = dpr;
        self.bounds.copy_from_slice(bounds);
        self.viewport_css = [width, height];
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        let g = &self.gl;
        g.disable(Gl::SCISSOR_TEST);
        g.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        g.disable(Gl::DEPTH_TEST);
        g.enable(Gl::BLEND);
        g.blend_func(Gl::SRC_ALPHA, Gl::ONE_MINUS_SRC_ALPHA);
        g.clear_color(background[0], background[1], background[2], background[3]);
        g.clear(Gl::COLOR_BUFFER_BIT);
        g.use_program(Some(&self.program));
        g.uniform4fv_with_f32_array(
            g.get_uniform_location(&self.program, "bounds").as_ref(),
            bounds,
        );
        Ok(())
    }
    /// NaNs break lines. Marker radius and width are specified in CSS pixels.
    pub fn series(
        &self,
        xy: &[f32],
        color: &[f32],
        connected: bool,
        radius: f32,
    ) -> Result<(), JsValue> {
        if xy.len() % 2 != 0
            || color.len() != 4
            || !color
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            || !radius.is_finite()
            || radius < 0.
        {
            return Err(super::error("Invalid scientific series"));
        }
        let mut lines = Vec::new();
        let mut points = Vec::new();
        let mut last: Option<[f32; 2]> = None;
        for p in xy.chunks_exact(2) {
            if p.iter().all(|v| v.is_finite()) {
                points.extend_from_slice(p);
                if connected {
                    if let Some(q) = last {
                        lines.extend_from_slice(&q);
                        lines.extend_from_slice(p);
                    }
                }
                last = Some([p[0], p[1]]);
            } else {
                last = None;
            }
        }
        self.paint(&lines, color, Gl::LINES, false, 1.);
        if radius > 0. {
            self.paint(&points, color, Gl::POINTS, true, 2. * radius * self.dpr);
        }
        Ok(())
    }
    /// Independent x,y segment endpoints; nonfinite segments are skipped.
    pub fn segments(&self, xy: &[f32], color: &[f32]) -> Result<(), JsValue> {
        if xy.len() % 4 != 0
            || color.len() != 4
            || !color
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
        {
            return Err(super::error("Invalid segments"));
        }
        let values: Vec<f32> = xy
            .chunks_exact(4)
            .filter(|p| p.iter().all(|v| v.is_finite()))
            .flatten()
            .copied()
            .collect();
        self.paint(&values, color, Gl::LINES, false, 1.);
        Ok(())
    }
    pub fn renderer(&self) -> String {
        "rustplotlib · Rust/WASM · WebGL2 scientific series".into()
    }
    /// Restrict subsequent primitives to a CSS-pixel plot rectangle.
    pub fn viewport(&mut self, x: f32, y: f32, width: f32, height: f32) -> Result<(), JsValue> {
        if ![x, y, width, height].iter().all(|v| v.is_finite()) || width <= 0. || height <= 0. {
            return Err(super::error("Invalid plot viewport"));
        }
        let g = &self.gl;
        self.viewport_css = [width, height];
        g.enable(Gl::SCISSOR_TEST);
        g.viewport(
            (x * self.dpr) as i32,
            (y * self.dpr) as i32,
            (width * self.dpr) as i32,
            (height * self.dpr) as i32,
        );
        g.scissor(
            (x * self.dpr) as i32,
            (y * self.dpr) as i32,
            (width * self.dpr) as i32,
            (height * self.dpr) as i32,
        );
        Ok(())
    }
}
impl SeriesGlView {
    fn paint(&self, xy: &[f32], color: &[f32], mode: u32, round: bool, size: f32) {
        if xy.is_empty() {
            return;
        }
        // Portable CSS-width lines: WebGL implementations often clamp LINES
        // to one device pixel. Expand each segment into two triangles.
        let mut triangles = Vec::new();
        if mode == Gl::LINES {
            let sx = self.bounds[2] - self.bounds[0];
            let sy = self.bounds[3] - self.bounds[1];
            for p in xy.chunks_exact(4) {
                let dx = (p[2] - p[0]) / sx * self.viewport_css[0];
                let dy = (p[3] - p[1]) / sy * self.viewport_css[1];
                let length = dx.hypot(dy);
                if length <= 1e-10 {
                    continue;
                }
                let ox = -dy / length * 0.8 / self.viewport_css[0] * sx;
                let oy = dx / length * 0.8 / self.viewport_css[1] * sy;
                triangles.extend_from_slice(&[
                    p[0] + ox,
                    p[1] + oy,
                    p[0] - ox,
                    p[1] - oy,
                    p[2] + ox,
                    p[3] + oy,
                    p[2] + ox,
                    p[3] + oy,
                    p[0] - ox,
                    p[1] - oy,
                    p[2] - ox,
                    p[3] - oy,
                ]);
            }
        }
        let (xy, mode) = if mode == Gl::LINES {
            (triangles.as_slice(), Gl::TRIANGLES)
        } else {
            (xy, mode)
        };
        let g = &self.gl;
        g.use_program(Some(&self.program));
        g.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.buffer));
        let array = js_sys::Float32Array::from(xy);
        g.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &array, Gl::DYNAMIC_DRAW);
        let pos = g.get_attrib_location(&self.program, "position") as u32;
        g.enable_vertex_attrib_array(pos);
        g.vertex_attrib_pointer_with_i32(pos, 2, Gl::FLOAT, false, 0, 0);
        g.uniform4fv_with_f32_array(
            g.get_uniform_location(&self.program, "color").as_ref(),
            color,
        );
        g.uniform1i(
            g.get_uniform_location(&self.program, "roundPoint").as_ref(),
            round as i32,
        );
        g.uniform1f(
            g.get_uniform_location(&self.program, "pointSize").as_ref(),
            size,
        );
        g.draw_arrays(mode, 0, (xy.len() / 2) as i32);
    }
}
impl Drop for SeriesGlView {
    fn drop(&mut self) {
        self.gl.delete_buffer(Some(&self.buffer));
        self.gl.delete_program(Some(&self.program));
    }
}
