use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Every label is genuine LaTeX, including ordinary text and numeric ticks.
#[derive(Clone, Debug)]
pub struct Latex {
    pub cache: PathBuf,
    pub preamble: String,
    pub font_pt: f64,
}
#[derive(Clone, Debug)]
pub struct Glyph {
    pub body: String,
    pub view_box: [f64; 4],
    pub width_pt: f64,
    pub height_pt: f64,
}
impl Default for Latex {
    fn default() -> Self {
        Self {
            cache: PathBuf::from(".latex-cache"),
            preamble: r"\usepackage[T1]{fontenc}
\usepackage{amsmath,amssymb}
\usepackage{mathpazo}"
                .into(),
            font_pt: 10.0,
        }
    }
}
impl Latex {
    fn key(&self, s: &str) -> String {
        format!(
            "{:x}",
            Sha256::digest(format!("{}\n{}\n{}", self.preamble, self.font_pt, s))
        )
    }
    /// One LaTeX invocation for all uncached labels, not one subprocess per tick.
    pub fn typeset(
        &self,
        labels: impl IntoIterator<Item = String>,
    ) -> Result<BTreeMap<String, Glyph>> {
        ensure!(
            self.font_pt.is_finite() && self.font_pt > 0.0,
            "Invalid typesetting size"
        );
        fs::create_dir_all(&self.cache)?;
        let unique: std::collections::BTreeSet<_> = labels.into_iter().collect();
        let missing: Vec<_> = unique
            .iter()
            .filter(|s| !self.cache.join(format!("{}.svg", self.key(s))).exists())
            .cloned()
            .collect();
        if !missing.is_empty() {
            let id = format!("{:x}", Sha256::digest(missing.join("\n")));
            let work = self.cache.join(format!(
                "job-{}-{}-{}",
                std::process::id(),
                &id[..12],
                JOB_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&work)?;
            let mut source = format!(
                "\\documentclass{{article}}\n{}\n\\usepackage[active,tightpage]{{preview}}\n\\setlength\\PreviewBorder{{0pt}}\n\\begin{{document}}\n",
                self.preamble
            );
            for label in &missing {
                source.push_str(&format!(
                    "\\begin{{preview}}\\fontsize{{{}}}{{{}}}\\selectfont {}\\end{{preview}}\n",
                    self.font_pt,
                    self.font_pt * 1.2,
                    label
                ));
            }
            source.push_str("\\end{document}\n");
            fs::write(work.join("labels.tex"), source)?;
            let out = Command::new("latex")
                .args([
                    "-no-shell-escape",
                    "-interaction=nonstopmode",
                    "-halt-on-error",
                    "labels.tex",
                ])
                .current_dir(&work)
                .output()
                .context("LaTeX is required: install TeX Live or MacTeX")?;
            if !out.status.success() {
                bail!("LaTeX failed: {}", String::from_utf8_lossy(&out.stdout));
            }
            let out = Command::new("dvisvgm")
                .args([
                    "--no-fonts",
                    "--exact",
                    "--bbox=min",
                    "--page=1-",
                    "--output=label-%p.svg",
                    "labels.dvi",
                ])
                .current_dir(&work)
                .output()
                .context("dvisvgm is required")?;
            ensure!(
                out.status.success(),
                "dvisvgm failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            for (i, label) in missing.iter().enumerate() {
                let from = fs::read_dir(&work)?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .find(|p| {
                        p.file_stem()
                            .and_then(|x| x.to_str())
                            .and_then(|x| x.strip_prefix("label-"))
                            .and_then(|x| x.parse::<usize>().ok())
                            == Some(i + 1)
                    })
                    .context("Missing LaTeX SVG page")?;
                let svg = fs::read_to_string(&from)?;
                parse(&svg)?;
                let target = self.cache.join(format!("{}.svg", self.key(label)));
                fs::rename(from, target)?;
            }
            fs::remove_dir_all(work)?;
        }
        unique
            .into_iter()
            .map(|s| {
                let svg = fs::read_to_string(self.cache.join(format!("{}.svg", self.key(&s))))?;
                Ok((s, parse(&svg)?))
            })
            .collect()
    }
    pub fn cached_label_count(&self) -> usize {
        fs::read_dir(&self.cache)
            .map(|d| {
                d.flatten()
                    .filter(|e| e.path().extension().is_some_and(|x| x == "svg"))
                    .count()
            })
            .unwrap_or(0)
    }
}
fn attr(s: &str, key: &str) -> Result<String> {
    for q in ['\'', '"'] {
        let token = format!("{key}={q}");
        if let Some(start) = s.find(&token) {
            let v = &s[start + token.len()..];
            if let Some(end) = v.find(q) {
                return Ok(v[..end].into());
            }
        }
    }
    bail!("SVG attribute missing: {key}")
}
fn parse(svg: &str) -> Result<Glyph> {
    let start = svg.find("<svg").context("No SVG root")?;
    let end = svg[start..].find('>').unwrap() + start;
    let root = &svg[start..end];
    let v = attr(root, "viewBox")?
        .split_whitespace()
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        v.len() == 4 && v[2] > 0.0 && v[3] > 0.0,
        "Invalid LaTeX label bounds"
    );
    let stop = svg.rfind("</svg>").context("No SVG end")?;
    Ok(Glyph {
        body: svg[end + 1..stop].into(),
        view_box: [v[0], v[1], v[2], v[3]],
        width_pt: v[2],
        height_pt: v[3],
    })
}
/// Labels are trusted LaTeX input; `-no-shell-escape` is always used.
pub fn text(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "\\&".into(),
            '%' => "\\%".into(),
            '_' => "\\_".into(),
            '#' => "\\#".into(),
            '$' => "\\$".into(),
            '{' => "\\{".into(),
            '}' => "\\}".into(),
            '\\' => "\\textbackslash{}".into(),
            '^' => "\\textasciicircum{}".into(),
            '~' => "\\textasciitilde{}".into(),
            _ => c.to_string(),
        })
        .collect()
}
pub fn cache_path(path: impl AsRef<Path>) -> Latex {
    Latex {
        cache: path.as_ref().into(),
        ..Latex::default()
    }
}
