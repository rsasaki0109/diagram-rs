//! Dependency-free stage timing harness; see benchmarks/README.md for methodology.
use diagram_layout::layout;
use diagram_parser::parse;
use diagram_render::{Font, render, svg};
use std::{hint::black_box, time::Instant};
fn time(iterations: usize, mut action: impl FnMut()) -> f64 {
    for _ in 0..10 {
        action();
    }
    let mut samples = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        for _ in 0..iterations {
            action();
        }
        samples.push(start.elapsed().as_secs_f64() * 1e6 / iterations as f64);
    }
    samples.sort_by(f64::total_cmp);
    samples[2]
}
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--memory") {
        let n = std::env::args().nth(2).unwrap().parse().unwrap();
        black_box(render(&parse(&source(n)).unwrap()).unwrap());
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        let peak = status
            .lines()
            .find(|l| l.starts_with("VmHWM:"))
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap();
        println!("{peak}");
        return;
    }
    let font = Font::default();
    println!("nodes,edges,iterations,parse_us,layout_us,svg_us,end_to_end_us,svg_bytes");
    for (n, iterations) in [(8, 500), (64, 100), (256, 20)] {
        let source = source(n);
        let graph = parse(&source).unwrap();
        let geometry = layout(&graph, &font).unwrap();
        let parse_us = time(iterations, || {
            black_box(parse(black_box(&source)).unwrap());
        });
        let layout_us = time(iterations, || {
            black_box(layout(black_box(&graph), &font).unwrap());
        });
        let svg_us = time(iterations, || {
            black_box(svg(black_box(&graph), &geometry, &font).unwrap());
        });
        let total_us = time(iterations, || {
            black_box(render(&parse(black_box(&source)).unwrap()).unwrap());
        });
        println!(
            "{n},{},{iterations},{parse_us:.2},{layout_us:.2},{svg_us:.2},{total_us:.2},{}",
            graph.edges.len(),
            render(&graph).unwrap().len()
        );
    }
}

fn source(n: usize) -> String {
    let mut source = "flowchart LR\n".to_string();
    for i in 0..n {
        source.push_str(&format!("N{i}[Stage {i}]\n"));
        if i > 0 {
            source.push_str(&format!("N{}-->N{i}\n", i - 1));
        }
        if i > 3 && i % 4 == 0 {
            source.push_str(&format!("N{}-->N{i}\n", i - 3));
        }
    }
    source
}
