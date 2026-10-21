//! Place contour labels in physical coordinates using measured LaTeX bounds.
use crate::{Figure, Label, Rect};
use anyhow::{Result, ensure};

/// Projection-aware graticule text. Candidates are supplied in preference
/// order after projecting geographic anchors into figure millimetres.
pub struct GraticuleLabel {
    pub latex: String,
    pub candidates: Vec<[f64; 2]>,
    pub required: bool,
}

pub fn clear_label_position(
    candidates: &[[f64; 2]],
    size: [f64; 2],
    bounds: Rect,
    obstacles: &[Rect],
) -> Result<[f64; 2]> {
    let mut best: Option<(f64, [f64; 2])> = None;
    for &p in candidates {
        let r = Rect::new(p[0] - size[0] / 2., p[1] - size[1] / 2., size[0], size[1]);
        if !p.iter().all(|v| v.is_finite())
            || r.x < bounds.x
            || r.y < bounds.y
            || r.x + r.width > bounds.x + bounds.width
            || r.y + r.height > bounds.y + bounds.height
        {
            continue;
        }
        let mut clearance = f64::INFINITY;
        let mut overlaps = false;
        for o in obstacles {
            let dx = (o.x - r.x - r.width).max(r.x - o.x - o.width).max(0.);
            let dy = (o.y - r.y - r.height).max(r.y - o.y - o.height).max(0.);
            if dx == 0. && dy == 0. {
                overlaps = true;
                break;
            }
            clearance = clearance.min(dx.hypot(dy));
        }
        if !overlaps && best.as_ref().is_none_or(|(score, _)| clearance > *score) {
            best = Some((clearance, p));
        }
    }
    ensure!(
        best.is_some(),
        "No uncluttered contour-label position; supply more candidates or omit the label"
    );
    Ok(best.unwrap().1)
}

impl Figure {
    /// Attach text to a data-coordinate anchor. Default placement avoids visible
    /// geometry and existing text using measured LaTeX bounds. No white backing
    /// is used to conceal collisions. Returns the inserted label index.
    pub fn annotate(
        &mut self,
        panel: &crate::Panel,
        latex: impl Into<String>,
        anchor: [f64; 2],
    ) -> Result<usize> {
        let latex = latex.into();
        let legend = crate::Legend::new(vec![crate::LegendEntry::circle("annotation", "black")]);
        let mask = self.legend_occupancy(panel, &legend)?;
        let w = (panel.rect.width * 4.).ceil() as usize;
        let mut obstacles = Vec::new();
        // Compact horizontal runs preserve all occupied quarter-mm cells.
        for (y, row) in mask.chunks(w).enumerate() {
            let mut x = 0;
            while x < row.len() {
                if !row[x] {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < row.len() && row[x] {
                    x += 1;
                }
                obstacles.push(Rect::new(
                    panel.rect.x + start as f64 / 4. - 0.5,
                    panel.rect.y + y as f64 / 4. - 0.5,
                    (x - start) as f64 / 4. + 1.,
                    1.25,
                ));
            }
        }
        let glyphs = self.latex.typeset(
            self.labels
                .iter()
                .map(|l| l.latex.clone())
                .chain(std::iter::once(latex.clone())),
        )?;
        for l in &self.labels {
            let g = &glyphs[&l.latex];
            let angle = l.rotation.to_radians();
            let gw = g.width_pt * 25.4 / 72.27;
            let gh = g.height_pt * 25.4 / 72.27;
            let rw = gw * angle.cos().abs() + gh * angle.sin().abs();
            let rh = gh * angle.cos().abs() + gw * angle.sin().abs();
            obstacles.push(Rect::new(
                l.x - rw / 2. - 0.7,
                l.y - rh / 2. - 0.7,
                rw + 1.4,
                rh + 1.4,
            ));
        }
        let g = &glyphs[&latex];
        let size = [
            g.width_pt * 25.4 / 72.27 + 0.8,
            g.height_pt * 25.4 / 72.27 + 0.8,
        ];
        let origin = panel.data_to_figure(anchor);
        let mut selected = None;
        for gap in [1.5, 3., 5., 8., 12.] {
            let dx = size[0] / 2. + gap;
            let dy = size[1] / 2. + gap;
            for delta in [
                [dx, 0.],
                [-dx, 0.],
                [0., -dy],
                [0., dy],
                [dx, -dy],
                [-dx, -dy],
                [dx, dy],
                [-dx, dy],
            ] {
                let candidate = [origin[0] + delta[0], origin[1] + delta[1]];
                if clear_label_position(&[candidate], size, panel.rect, &obstacles).is_ok() {
                    selected = Some(candidate);
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let xy = selected.ok_or_else(|| {
            anyhow::anyhow!(
                "No clear annotation position for {latex}; enlarge the panel or reduce clutter"
            )
        })?;
        let index = self.labels.len();
        let mut label = Label::new(latex, xy[0], xy[1]);
        let gap_x = ((xy[0] - origin[0]).abs() - size[0] / 2.).max(0.);
        let gap_y = ((xy[1] - origin[1]).abs() - size[1] / 2.).max(0.);
        if gap_x.hypot(gap_y) > 3. {
            label.leader = Some(origin);
        }
        self.labels.push(label);
        Ok(index)
    }

    /// Validate fixed annotations against the current projected geometry.
    /// Call for every animation frame after changing the camera or data. This
    /// measures actual LaTeX bounds, including rotation and padding, and fails
    /// rather than hiding a collision behind an opaque label background.
    /// `annotate` remains the automatic placement API for an individual frame.
    pub fn check_annotations(&self, panel: &crate::Panel, labels: &[Label]) -> Result<()> {
        let legend = crate::Legend::new(vec![crate::LegendEntry::circle("annotation", "black")]);
        let mask = self.legend_occupancy(panel, &legend)?;
        let w = (panel.rect.width * 4.).ceil() as usize;
        let glyphs = self.latex.typeset(labels.iter().map(|l| l.latex.clone()))?;
        let mut boxes: Vec<Rect> = Vec::new();
        for label in labels {
            let g = &glyphs[&label.latex];
            let angle = label.rotation.to_radians();
            let gw = g.width_pt * 25.4 / 72.27;
            let gh = g.height_pt * 25.4 / 72.27;
            let padding = label.padding_mm.max(0.) + 0.5;
            let rw = gw * angle.cos().abs() + gh * angle.sin().abs() + 2. * padding;
            let rh = gh * angle.cos().abs() + gw * angle.sin().abs() + 2. * padding;
            let r = Rect::new(label.x - rw / 2., label.y - rh / 2., rw, rh);
            ensure!(
                boxes.iter().all(|b| r.x + r.width < b.x
                    || b.x + b.width < r.x
                    || r.y + r.height < b.y
                    || b.y + b.height < r.y),
                "Annotation {} overlaps another annotation",
                label.latex
            );
            for (i, occupied) in mask.iter().enumerate() {
                if !occupied {
                    continue;
                }
                let x = panel.rect.x + (i % w) as f64 / 4.;
                let y = panel.rect.y + (i / w) as f64 / 4.;
                ensure!(
                    x + 0.25 < r.x || x > r.x + r.width || y + 0.25 < r.y || y > r.y + r.height,
                    "Annotation {} overlaps projected geometry; move it or use annotate",
                    label.latex
                );
            }
            boxes.push(r);
        }
        Ok(())
    }

    /// Place an upright label outside a convex map boundary. Its entire measured
    /// glyph rectangle clears the boundary tangent by `clearance_mm`.
    pub fn boundary_label(
        &self,
        latex: impl Into<String>,
        anchor: [f64; 2],
        outward: [f64; 2],
        clearance_mm: f64,
    ) -> Result<Label> {
        let latex = latex.into();
        let norm = outward[0].hypot(outward[1]);
        ensure!(
            norm.is_finite() && norm > 0. && clearance_mm >= 0. && clearance_mm.is_finite(),
            "Invalid boundary-label normal/clearance"
        );
        let glyphs = self.latex.typeset([latex.clone()])?;
        let g = &glyphs[&latex];
        let n = [outward[0] / norm, outward[1] / norm];
        let support = (n[0].abs() * g.width_pt + n[1].abs() * g.height_pt) * 25.4 / 72.27 / 2.;
        let distance = support + clearance_mm;
        Ok(Label::new(
            latex,
            anchor[0] + n[0] * distance,
            anchor[1] + n[1] * distance,
        ))
    }

    /// Label a projected graticule using real glyph bounds. Required coordinates
    /// are placed first; optional coordinates are thinned if they would collide.
    /// Labels stay upright and centred on their geographic anchors, never slid
    /// sideways to imply a different longitude. A backing interrupts grid lines.
    pub fn graticule_labels(
        &self,
        ticks: &[GraticuleLabel],
        obstacles: &[Rect],
    ) -> Result<Vec<Label>> {
        let glyphs = self.latex.typeset(
            self.labels
                .iter()
                .map(|l| l.latex.clone())
                .chain(ticks.iter().map(|t| t.latex.clone())),
        )?;
        let mut occupied = obstacles.to_vec();
        for l in &self.labels {
            let g = &glyphs[&l.latex];
            let a = l.rotation.to_radians();
            let w = g.width_pt * 25.4 / 72.27;
            let h = g.height_pt * 25.4 / 72.27;
            let rw = w * a.cos().abs() + h * a.sin().abs();
            let rh = h * a.cos().abs() + w * a.sin().abs();
            occupied.push(Rect::new(
                l.x - rw / 2. - 0.7,
                l.y - rh / 2. - 0.7,
                rw + 1.4,
                rh + 1.4,
            ));
        }
        let mut ordered: Vec<_> = ticks.iter().collect();
        ordered.sort_by_key(|tick| !tick.required);
        let mut labels = Vec::new();
        for tick in ordered {
            if labels.iter().any(|l: &Label| l.latex == tick.latex) {
                continue;
            }
            let g = &glyphs[&tick.latex];
            let w = g.width_pt * 25.4 / 72.27 + 1.6;
            let h = g.height_pt * 25.4 / 72.27 + 1.6;
            let candidate = tick.candidates.iter().copied().find(|p| {
                p.iter().all(|v| v.is_finite())
                    && occupied.iter().all(|o| {
                        p[0] + w / 2. <= o.x
                            || p[0] - w / 2. >= o.x + o.width
                            || p[1] + h / 2. <= o.y
                            || p[1] - h / 2. >= o.y + o.height
                    })
            });
            if let Some(p) = candidate {
                let mut label = Label::new(tick.latex.clone(), p[0], p[1]);
                label.background = Some("white".into());
                label.padding_mm = 0.4;
                occupied.push(Rect::new(p[0] - w / 2., p[1] - h / 2., w, h));
                labels.push(label);
            } else {
                ensure!(
                    !tick.required,
                    "Required graticule coordinate has no clear anchor: {}",
                    tick.latex
                );
            }
        }
        Ok(labels)
    }

    /// Set plain text along a contour in figure millimetres. Glyphs follow the
    /// local tangent, remain upright, and interrupt the line with white backing.
    /// Selects an on-curve span clear of measurements and existing annotations.
    pub fn curved_contour_label(
        &self,
        text: &str,
        font_pt: f64,
        path: &[[f64; 2]],
        bounds: Rect,
        obstacles: &[Rect],
    ) -> Result<Vec<Label>> {
        ensure!(
            font_pt.is_finite() && font_pt > 0.,
            "Invalid contour font size"
        );
        let chars: Vec<_> = text
            .chars()
            .map(|c| {
                if c == ' ' {
                    None
                } else {
                    Some(format!(
                        "{{\\fontsize{{{font_pt}}}{{{}}}\\selectfont\\strut {c}}}",
                        font_pt * 1.2
                    ))
                }
            })
            .collect();
        let glyphs = self.latex.typeset(
            chars
                .iter()
                .flatten()
                .cloned()
                .chain(self.labels.iter().map(|l| l.latex.clone())),
        )?;
        let widths: Vec<_> = chars
            .iter()
            .map(|c| {
                c.as_ref().map_or(font_pt * 0.25 * 25.4 / 72.27, |c| {
                    glyphs[c].width_pt * 25.4 / 72.27
                })
            })
            .collect();
        let total: f64 = widths.iter().sum();
        let mut occupied = obstacles.to_vec();
        for l in &self.labels {
            let g = &glyphs[&l.latex];
            let a = l.rotation.to_radians();
            let w = g.width_pt * 25.4 / 72.27;
            let h = g.height_pt * 25.4 / 72.27;
            let rw = w * a.cos().abs() + h * a.sin().abs();
            let rh = h * a.cos().abs() + w * a.sin().abs();
            occupied.push(Rect::new(
                l.x - rw / 2. - 0.6,
                l.y - rh / 2. - 0.6,
                rw + 1.2,
                rh + 1.2,
            ));
        }
        let distance = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        let mut best: Option<(f64, Vec<Label>)> = None;
        // Large jumps identify disjoint contour components, not label spans.
        let mut components = vec![Vec::new()];
        for &p in path {
            let last = components.last_mut().unwrap();
            if !p.iter().all(|v| v.is_finite()) {
                components.push(Vec::new());
                continue;
            }
            if last.last().is_some_and(|&q| distance(p, q) > 10.) {
                components.push(vec![p]);
            } else {
                last.push(p);
            }
        }
        for mut points in components {
            if points.len() < 2 {
                continue;
            }
            // Choose reading direction per candidate, not globally on a loop.
            let lengths: Vec<_> = std::iter::once(0.)
                .chain(points.windows(2).scan(0., |s, p| {
                    *s += distance(p[0], p[1]);
                    Some(*s)
                }))
                .collect();
            let at = |s: f64| -> [f64; 2] {
                let i = lengths
                    .partition_point(|&v| v < s)
                    .clamp(1, lengths.len() - 1);
                let f =
                    ((s - lengths[i - 1]) / (lengths[i] - lengths[i - 1]).max(1e-12)).clamp(0., 1.);
                [
                    points[i - 1][0] + f * (points[i][0] - points[i - 1][0]),
                    points[i - 1][1] + f * (points[i][1] - points[i - 1][1]),
                ]
            };
            for &center in &lengths {
                if center < total / 2. + 1. || center + total / 2. + 1. > *lengths.last().unwrap() {
                    continue;
                }
                let left = at(center - total / 2.);
                let right = at(center + total / 2.);
                let direction = if right[0] >= left[0] { 1. } else { -1. };
                let mut offset = -total / 2.;
                let mut labels = vec![];
                let mut clearance = f64::INFINITY;
                let mut valid = true;
                let mut last_angle: Option<f64> = None;
                for (c, &w) in chars.iter().zip(&widths) {
                    let s = center + direction * (offset + w / 2.);
                    offset += w;
                    let Some(c) = c else {
                        continue;
                    };
                    let p = at(s);
                    let a = at(s - direction * 0.8);
                    let b = at(s + direction * 0.8);
                    let angle = (b[1] - a[1]).atan2(b[0] - a[0]);
                    if angle.abs() > 1.25 || last_angle.is_some_and(|old| (old - angle).abs() > 0.3)
                    {
                        valid = false;
                        break;
                    }
                    last_angle = Some(angle);
                    let h = glyphs[c].height_pt * 25.4 / 72.27;
                    let rw = w * angle.cos().abs() + h * angle.sin().abs() + 0.8;
                    let rh = h * angle.cos().abs() + w * angle.sin().abs() + 0.8;
                    let r = Rect::new(p[0] - rw / 2., p[1] - rh / 2., rw, rh);
                    if r.x < bounds.x
                        || r.y < bounds.y
                        || r.x + rw > bounds.x + bounds.width
                        || r.y + rh > bounds.y + bounds.height
                    {
                        valid = false;
                        break;
                    }
                    for o in &occupied {
                        let dx = (o.x - r.x - rw).max(r.x - o.x - o.width).max(0.);
                        let dy = (o.y - r.y - rh).max(r.y - o.y - o.height).max(0.);
                        if dx == 0. && dy == 0. {
                            valid = false;
                            break;
                        }
                        clearance = clearance.min(dx.hypot(dy));
                    }
                    if !valid {
                        break;
                    }
                    let mut label = Label::new(c.clone(), p[0], p[1]);
                    label.rotation = angle.to_degrees();
                    label.background = Some("white".into());
                    label.padding_mm = 0.35;
                    labels.push(label);
                }
                if valid && best.as_ref().is_none_or(|(score, _)| clearance > *score) {
                    best = Some((clearance, labels));
                }
            }
            points.clear();
        }
        best.map(|(_, labels)| labels)
            .ok_or_else(|| anyhow::anyhow!("No clear, gently curved contour-label span for {text}"))
    }

    /// Candidates and obstacles are in figure millimetres. Returned labels have
    /// an opaque backing sized from actual LaTeX glyphs to interrupt the contour.
    pub fn contour_label(
        &self,
        text: impl Into<String>,
        candidates: &[[f64; 2]],
        bounds: Rect,
        obstacles: &[Rect],
    ) -> Result<Label> {
        let text = text.into();
        let mut all = self
            .labels
            .iter()
            .map(|l| l.latex.clone())
            .collect::<Vec<_>>();
        all.push(text.clone());
        let glyphs = self.latex.typeset(all)?;
        let mut occupied = obstacles.to_vec();
        for l in &self.labels {
            let g = &glyphs[&l.latex];
            let a = l.rotation.to_radians();
            let w = g.width_pt * 25.4 / 72.27;
            let h = g.height_pt * 25.4 / 72.27;
            let (w, h) = (
                w * a.cos().abs() + h * a.sin().abs(),
                h * a.cos().abs() + w * a.sin().abs(),
            );
            occupied.push(Rect::new(
                l.x - w / 2. - 0.7,
                l.y - h / 2. - 0.7,
                w + 1.4,
                h + 1.4,
            ));
        }
        let g = &glyphs[&text];
        let size = [
            g.width_pt * 25.4 / 72.27 + 1.4,
            g.height_pt * 25.4 / 72.27 + 1.4,
        ];
        let direct = clear_label_position(candidates, size, bounds, &occupied);
        let mut result = direct.as_ref().ok().copied();
        if result.is_none() {
            for radius in [3., 6., 10., 16.] {
                let nearby: Vec<_> = candidates
                    .iter()
                    .flat_map(|p| {
                        (0..8).map(move |i| {
                            let angle = i as f64 * std::f64::consts::TAU / 8.;
                            [p[0] + radius * angle.cos(), p[1] + radius * angle.sin()]
                        })
                    })
                    .collect();
                if let Ok(p) = clear_label_position(&nearby, size, bounds, &occupied) {
                    result = Some(p);
                    break;
                }
            }
        }
        let p =
            result.ok_or_else(|| anyhow::anyhow!("No clear position on or near the contour"))?;
        let mut label = Label::new(text, p[0], p[1]);
        label.background = Some("white".into());
        if direct.is_err() {
            label.leader = candidates
                .iter()
                .filter(|q| q.iter().all(|v| v.is_finite()))
                .min_by(|a, b| {
                    (a[0] - p[0])
                        .hypot(a[1] - p[1])
                        .total_cmp(&(b[0] - p[0]).hypot(b[1] - p[1]))
                })
                .copied();
        }
        Ok(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_annotation_is_checked_at_every_animation_frame() {
        use crate::{Axes, IntendedUse, Layer, Line, Panel};
        let f = Figure::new(IntendedUse::ArticleWide);
        let label = Label::new("$r_\\perp$", 50., 40.);
        // Endpoints clear the label; the middle frame crosses it.
        for (y, clear) in [(0.2, true), (0.5, false), (0.8, true)] {
            let mut p = Panel::new(
                Rect::new(10., 10., 80., 60.),
                Axes::new([0., 1.], [0., 1.], "", ""),
            );
            p.push(Layer::Line(Line::new(vec![[0., y], [1., y]], "black")));
            assert_eq!(
                f.check_annotations(&p, std::slice::from_ref(&label))
                    .is_ok(),
                clear
            );
        }
    }

    #[test]
    fn projected_surface_and_rotated_annotation_collisions_are_rejected() {
        use crate::{Axes, ColorMap, IntendedUse, Layer, Normalization, Panel, Scale, Surface};
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let sc =
            f.scale(Scale::new([0., 1.], Normalization::Linear, ColorMap::sequential()).unwrap());
        for yaw in [-1.0, 0.0, 1.0] {
            let mut surface =
                Surface::new(vec![0., 1.], vec![0., 1.], vec![0.5; 4], [0., 1.], sc).unwrap();
            surface.yaw = yaw;
            let mut p = Panel::new(
                Rect::new(10., 10., 80., 60.),
                Axes::new([0., 1.], [0., 1.], "", ""),
            );
            p.push(Layer::Surface(surface));
            let mut label = Label::new("$r_\\perp$", 50., 40.);
            label.rotation = 35.;
            label.background = Some("white".into());
            assert!(f.check_annotations(&p, &[label]).is_err());
            assert!(
                f.check_annotations(&p, &[Label::new("$z$", 5., 5.)])
                    .is_ok()
            );
        }
    }

    #[test]
    fn boundary_label_clears_entire_glyph_box() {
        let f = Figure::new(crate::IntendedUse::ArticleWide);
        let n = [-0.6, -0.8];
        let label = f.boundary_label("$75^\\circ$", [40., 20.], n, 1.).unwrap();
        let glyphs = f.latex.typeset([label.latex.clone()]).unwrap();
        let g = &glyphs[&label.latex];
        let support = (n[0].abs() * g.width_pt + n[1].abs() * g.height_pt) * 25.4 / 72.27 / 2.;
        let distance = (label.x - 40.) * n[0] + (label.y - 20.) * n[1];
        assert!((distance - support - 1.).abs() < 1e-10);
    }
    #[test]
    fn graticule_preserves_required_coordinates_and_thins_collisions() {
        let figure = Figure::new(crate::IntendedUse::ArticleWide);
        let tick = |s: &str, x, required| GraticuleLabel {
            latex: s.into(),
            candidates: vec![[x, 20.]],
            required,
        };
        let labels = figure
            .graticule_labels(
                &[
                    tick("$300^\\circ$", 20., false),
                    tick("$270^\\circ$", 20., true),
                    tick("$0^\\circ$", 40., true),
                    tick("$180^\\circ$", 60., true),
                    tick("$0^\\circ$", 40., false),
                ],
                &[],
            )
            .unwrap();
        assert_eq!(labels.len(), 3);
        assert_eq!(labels[0].latex, "$270^\\circ$");
        assert!(
            labels
                .iter()
                .all(|l| l.rotation == 0. && l.background.is_some())
        );
        assert!(
            figure
                .graticule_labels(
                    &[tick("$0^\\circ$", 40., true)],
                    &[Rect::new(0., 0., 100., 100.)]
                )
                .is_err()
        );
    }
    #[test]
    fn graticule_backing_clears_neighbouring_parallel() {
        let figure = Figure::new(crate::IntendedUse::ArticleWide);
        let parallel = Rect::new(0., 18., 100., 0.2);
        let ticks = [GraticuleLabel {
            latex: "$270^\\circ$".into(),
            required: true,
            candidates: vec![[50., 19.], [50., 24.]],
        }];
        let labels = figure.graticule_labels(&ticks, &[parallel]).unwrap();
        assert_eq!(labels[0].y, 24.);
        let glyphs = figure.latex.typeset([labels[0].latex.clone()]).unwrap();
        let half_backing =
            glyphs[&labels[0].latex].height_pt * 25.4 / 72.27 / 2. + labels[0].padding_mm;
        assert!(labels[0].y - half_backing > parallel.y + parallel.height);
    }
    #[test]
    fn curved_glyphs_follow_tangent_and_remain_on_path() {
        let figure = Figure::new(crate::IntendedUse::ArticleWide);
        let path: Vec<_> = (0..201)
            .map(|i| {
                let a = -0.6 + i as f64 * 0.006;
                [50. + 30. * a.sin(), 50. - 30. * a.cos()]
            })
            .collect();
        let labels = figure
            .curved_contour_label("3 h", 7., &path, Rect::new(0., 0., 100., 100.), &[])
            .unwrap();
        assert_eq!(labels.len(), 2);
        for label in &labels {
            let radius = (label.x - 50.).hypot(label.y - 50.);
            assert!((radius - 30.).abs() < 0.001);
            assert!(label.rotation.abs() < 80.);
            assert!(label.leader.is_none());
        }
        assert!((labels[0].rotation - labels[1].rotation).abs() > 1.);
        assert!(
            figure
                .curved_contour_label(
                    "3 h",
                    7.,
                    &path,
                    Rect::new(0., 0., 100., 100.),
                    &[Rect::new(0., 0., 100., 100.)]
                )
                .is_err()
        );
    }
    #[test]
    fn anchored_annotations_avoid_geometry_and_other_labels_by_default() {
        use crate::{Axes, IntendedUse, Layer, Line, Panel};
        let mut f = Figure::new(IntendedUse::ArticleWide);
        let mut p = Panel::new(
            Rect::new(10., 10., 70., 50.),
            Axes::new([0., 1.], [0., 1.], "", ""),
        );
        p.major_grid("#dddddd", 0.4);
        p.push(Layer::Line(Line::new(vec![[0.5, 0.5], [1., 1.]], "blue")));
        let first = f.annotate(&p, "Station", [0.5, 0.5]).unwrap();
        let second = f.annotate(&p, "Receiver", [0.5, 0.5]).unwrap();
        let glyphs = f
            .latex
            .typeset(f.labels.iter().map(|l| l.latex.clone()))
            .unwrap();
        let boxes: Vec<_> = [first, second]
            .iter()
            .map(|&i| {
                let l = &f.labels[i];
                let g = &glyphs[&l.latex];
                assert!(l.background.is_none());
                let w = g.width_pt * 25.4 / 72.27;
                let h = g.height_pt * 25.4 / 72.27;
                Rect::new(l.x - w / 2., l.y - h / 2., w, h)
            })
            .collect();
        assert!(
            boxes[0].x + boxes[0].width < boxes[1].x
                || boxes[1].x + boxes[1].width < boxes[0].x
                || boxes[0].y + boxes[0].height < boxes[1].y
                || boxes[1].y + boxes[1].height < boxes[0].y
        );
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let q = p.data_to_figure([0.5 + 0.5 * t, 0.5 + 0.5 * t]);
            assert!(boxes.iter().all(|r| q[0] < r.x
                || q[0] > r.x + r.width
                || q[1] < r.y
                || q[1] > r.y + r.height));
        }
        let mut full = Line::new(
            vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]],
            "black",
        );
        full.fill = Some("black".into());
        p.push(Layer::Line(full));
        assert!(f.annotate(&p, "No space", [0.5, 0.5]).is_err());
    }

    #[test]
    fn avoids_measurements_and_rejects_crowded_positions() {
        let bounds = Rect::new(0., 0., 50., 30.);
        let obstacles = vec![Rect::new(0., 0., 20., 30.)];
        assert_eq!(
            clear_label_position(&[[10., 15.], [40., 15.]], [8., 3.], bounds, &obstacles).unwrap(),
            [40., 15.]
        );
        assert!(clear_label_position(&[[10., 15.]], [8., 3.], bounds, &obstacles).is_err());
    }
}
