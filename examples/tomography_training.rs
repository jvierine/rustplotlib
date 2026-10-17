//! Plot recorded tomography RMS; an empty history is explicitly shown as empty.
use anyhow::Result;
use rustplotlib::*;
use serde_json::Value;
use std::{env, fs, path::PathBuf};
fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    let state: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let out = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("tomography_training"),
    );
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let fixed = state["fixed_training_history"]
        .as_array()
        .is_some_and(|v| !v.is_empty());
    let batch = fixed
        || state["batch_history"]
            .as_array()
            .is_some_and(|v| !v.is_empty());
    let history = state[if fixed {
        "fixed_training_history"
    } else if batch {
        "batch_history"
    } else {
        "history"
    }]
    .as_array()
    .cloned()
    .unwrap_or_default();
    let points: Vec<[f64; 2]> = history
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            m[if fixed { "rmse" } else { "train_rmse" }]
                .as_f64()
                .filter(|v| v.is_finite())
                .map(|r| {
                    [
                        m[if batch { "step" } else { "epoch" }]
                            .as_f64()
                            .unwrap_or(i as f64),
                        r,
                    ]
                })
        })
        .collect();
    let mut figure = Figure::new(IntendedUse::ArticleWide);
    figure.height_mm = 88.;
    let high = points.iter().map(|p| p[1]).fold(0., f64::max).max(1.) * 1.1;
    let xmax = if batch {
        points.last().map(|p| p[0]).unwrap_or(1.).max(1.)
    } else {
        3.
    };
    let mut axes = Axes::new(
        [0., xmax],
        [0., high],
        if batch { "Optimizer step" } else { "Epoch" },
        r"RMS residual [TECU]",
    );
    if !batch {
        axes.xticks = vec![0., 1., 2., 3.];
    }
    if points.is_empty() {
        axes.yticks = vec![];
    }
    let mut panel = Panel::new(Rect::new(23., 15., 138., 56.), axes);
    if !points.is_empty() {
        panel.push(Layer::Line(Line::new(points, "#245b78")));
    }
    figure.panels.push(panel);
    figure.labels.push(Label::new(
        if fixed {
            "Tomography: fixed training measurements"
        } else if batch {
            "Tomography training: minibatch RMS"
        } else {
            "Tomography training: epoch RMS"
        },
        92.,
        7.,
    ));
    if history.is_empty() {
        figure
            .labels
            .push(Label::new("No RMS evaluations yet", 92., 36.));
        figure.labels.push(Label::new(
            "Archive preparation; 0 epochs completed",
            92.,
            44.,
        ));
    }
    for extension in ["png", "pdf", "svg"] {
        figure.save(out.with_extension(extension))?;
    }
    Ok(())
}
