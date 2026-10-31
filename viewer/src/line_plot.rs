//! WebGL2 scientific line/scatter layer. Axes can be composed with SeriesView.
use super::{Gl, context, error, shader};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, WebGlBuffer, WebGlProgram};
#[wasm_bindgen]
pub struct LinePlot {
    gl: Gl,
    canvas: HtmlCanvasElement,
    program: WebGlProgram,
    buffer: WebGlBuffer,
    bounds: [f64; 4],
    dpr: f64,
    margins: [f64; 4],
}
#[wasm_bindgen]
impl LinePlot {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas: HtmlCanvasElement) -> Result<LinePlot, JsValue> {
        let gl = context(&canvas)?;
        let program = gl
            .create_program()
            .ok_or_else(|| error("No WebGL program"))?;
        let vs = shader(
            &gl,
            Gl::VERTEX_SHADER,
            "#version 300 es\nprecision highp float;in vec2 position;uniform float point_size;void main(){gl_Position=vec4(position,0.,1.);gl_PointSize=point_size;}",
        )?;
        let fs = shader(
            &gl,
            Gl::FRAGMENT_SHADER,
            "#version 300 es\nprecision highp float;uniform vec4 rgba;out vec4 color;void main(){color=rgba;}",
        )?;
        gl.attach_shader(&program, &vs);
        gl.attach_shader(&program, &fs);
        gl.link_program(&program);
        if gl
            .get_program_parameter(&program, Gl::LINK_STATUS)
            .as_bool()
            != Some(true)
        {
            return Err(error(gl.get_program_info_log(&program).unwrap_or_default()));
        }
        gl.delete_shader(Some(&vs));
        gl.delete_shader(Some(&fs));
        let buffer = gl.create_buffer().ok_or_else(|| error("No WebGL buffer"))?;
        Ok(Self {
            gl,
            canvas,
            program,
            buffer,
            bounds: [0., 1., 0., 1.],
            dpr: 1.,
            margins: [55., 20., 20., 35.],
        })
    }
    pub fn set_margins(
        &mut self,
        left: f64,
        right: f64,
        top: f64,
        bottom: f64,
    ) -> Result<(), JsValue> {
        let margins = [left, right, top, bottom];
        if !margins.iter().all(|v| v.is_finite() && *v >= 0.) {
            return Err(error("Invalid margins"));
        }
        self.margins = margins;
        Ok(())
    }
    pub fn begin(
        &mut self,
        width: f64,
        height: f64,
        dpr: f64,
        x0: f64,
        x1: f64,
        y0: f64,
        y1: f64,
    ) -> Result<(), JsValue> {
        if ![width, height, dpr, x0, x1, y0, y1]
            .iter()
            .all(|x| x.is_finite())
            || width < 100.
            || height < 100.
            || dpr <= 0.
            || x1 <= x0
            || y1 <= y0
        {
            return Err(error("Invalid line-plot bounds"));
        }
        self.bounds = [x0, x1, y0, y1];
        self.dpr = dpr;
        self.gl.disable(Gl::SCISSOR_TEST);
        self.canvas.set_width((width * dpr).round() as u32);
        self.canvas.set_height((height * dpr).round() as u32);
        self.gl.viewport(
            0,
            0,
            self.canvas.width() as i32,
            self.canvas.height() as i32,
        );
        self.gl.clear_color(0., 0., 0., 0.);
        self.gl.clear(Gl::COLOR_BUFFER_BIT);
        let [l, r, t, b] = self.margins;
        self.gl.enable(Gl::SCISSOR_TEST);
        self.gl.scissor(
            (l * dpr) as i32,
            (b * dpr) as i32,
            ((width - l - r) * dpr) as i32,
            ((height - t - b) * dpr) as i32,
        );
        Ok(())
    }
    /// Coordinates transformed in f64 before GPU upload, retaining close-up precision.
    pub fn series(
        &self,
        xy: &[f64],
        red: f32,
        green: f32,
        blue: f32,
        points: bool,
        size: f32,
        dashed: bool,
    ) -> Result<(), JsValue> {
        if xy.len() % 2 != 0
            || ![red, green, blue, size].iter().all(|v| v.is_finite())
            || size <= 0.
        {
            return Err(error("Invalid line data"));
        }
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        let dpr = self.dpr;
        let [l, r, t, b] = self.margins;
        let project = |p: &[f64]| {
            [
                (2. * (l * dpr
                    + (p[0] - self.bounds[0]) / (self.bounds[1] - self.bounds[0])
                        * (w - (l + r) * dpr))
                    / w
                    - 1.) as f32,
                (2. * (b * dpr
                    + (p[1] - self.bounds[2]) / (self.bounds[3] - self.bounds[2])
                        * (h - (t + b) * dpr))
                    / h
                    - 1.) as f32,
            ]
        };
        let mut data = Vec::new();
        if points {
            for p in xy.chunks_exact(2) {
                if p.iter().all(|v| v.is_finite()) {
                    data.extend(project(p));
                }
            }
        } else {
            for (i, p) in xy
                .chunks_exact(2)
                .collect::<Vec<_>>()
                .windows(2)
                .enumerate()
            {
                if p.iter().all(|q| q.iter().all(|v| v.is_finite()))
                    && (!dashed || (i / 3) % 2 == 0)
                {
                    let a = project(p[0]);
                    let b = project(p[1]);
                    let dx = (b[0] - a[0]) * w as f32;
                    let dy = (b[1] - a[1]) * h as f32;
                    let length = dx.hypot(dy);
                    if length == 0. {
                        continue;
                    }
                    let nx = -dy / length * size * dpr as f32 / w as f32;
                    let ny = dx / length * size * dpr as f32 / h as f32;
                    for q in [
                        [a[0] + nx, a[1] + ny],
                        [a[0] - nx, a[1] - ny],
                        [b[0] + nx, b[1] + ny],
                        [b[0] + nx, b[1] + ny],
                        [a[0] - nx, a[1] - ny],
                        [b[0] - nx, b[1] - ny],
                    ] {
                        data.extend(q);
                    }
                }
            }
        }
        let gl = &self.gl;
        gl.use_program(Some(&self.program));
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.buffer));
        let a = js_sys::Float32Array::from(data.as_slice());
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &a, Gl::DYNAMIC_DRAW);
        let loc = gl.get_attrib_location(&self.program, "position") as u32;
        gl.enable_vertex_attrib_array(loc);
        gl.vertex_attrib_pointer_with_i32(loc, 2, Gl::FLOAT, false, 8, 0);
        gl.uniform4f(
            gl.get_uniform_location(&self.program, "rgba").as_ref(),
            red,
            green,
            blue,
            1.,
        );
        gl.uniform1f(
            gl.get_uniform_location(&self.program, "point_size")
                .as_ref(),
            size * dpr as f32,
        );
        gl.draw_arrays(
            if points { Gl::POINTS } else { Gl::TRIANGLES },
            0,
            (data.len() / 2) as i32,
        );
        Ok(())
    }
    /// Filled interval, encoded as consecutive (x, lower, upper) triples.
    pub fn band(
        &self,
        xlohi: &[f64],
        red: f32,
        green: f32,
        blue: f32,
        alpha: f32,
    ) -> Result<(), JsValue> {
        if xlohi.len() % 3 != 0
            || ![red, green, blue, alpha].iter().all(|v| v.is_finite())
            || !(0. ..=1.).contains(&alpha)
        {
            return Err(error("Invalid band data"));
        }
        let w = self.canvas.width() as f64;
        let h = self.canvas.height() as f64;
        let d = self.dpr;
        let [l, r, t, b] = self.margins;
        let project = |x: f64, y: f64| {
            [
                (2. * (l * d
                    + (x - self.bounds[0]) / (self.bounds[1] - self.bounds[0]) * (w - (l + r) * d))
                    / w
                    - 1.) as f32,
                (2. * (b * d
                    + (y - self.bounds[2]) / (self.bounds[3] - self.bounds[2]) * (h - (t + b) * d))
                    / h
                    - 1.) as f32,
            ]
        };
        let mut vertices = Vec::new();
        for p in xlohi.chunks_exact(3).collect::<Vec<_>>().windows(2) {
            if !p
                .iter()
                .all(|q| q.iter().all(|v| v.is_finite()) && q[1] <= q[2])
            {
                continue;
            }
            let a = project(p[0][0], p[0][1]);
            let b = project(p[0][0], p[0][2]);
            let c = project(p[1][0], p[1][1]);
            let d = project(p[1][0], p[1][2]);
            for v in [a, c, b, b, c, d] {
                vertices.extend(v);
            }
        }
        let gl = &self.gl;
        gl.use_program(Some(&self.program));
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.buffer));
        let a = js_sys::Float32Array::from(vertices.as_slice());
        gl.buffer_data_with_array_buffer_view(Gl::ARRAY_BUFFER, &a, Gl::DYNAMIC_DRAW);
        let loc = gl.get_attrib_location(&self.program, "position") as u32;
        gl.enable_vertex_attrib_array(loc);
        gl.vertex_attrib_pointer_with_i32(loc, 2, Gl::FLOAT, false, 8, 0);
        gl.uniform4f(
            gl.get_uniform_location(&self.program, "rgba").as_ref(),
            red,
            green,
            blue,
            alpha,
        );
        gl.enable(Gl::BLEND);
        gl.blend_func_separate(
            Gl::SRC_ALPHA,
            Gl::ONE_MINUS_SRC_ALPHA,
            Gl::ONE,
            Gl::ONE_MINUS_SRC_ALPHA,
        );
        gl.draw_arrays(Gl::TRIANGLES, 0, (vertices.len() / 2) as i32);
        gl.disable(Gl::BLEND);
        Ok(())
    }
    pub fn renderer(&self) -> String {
        "Rustplotlib · Rust/Wasm · WebGL2".into()
    }
}
