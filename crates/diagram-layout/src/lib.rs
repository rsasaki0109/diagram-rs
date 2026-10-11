//! Deterministic Sugiyama-style flowchart layout: cycle reversal, longest-path
//! ranks, dummy vertices, barycenter sweeps, coordinates, and orthogonal routing.
use diagram_core::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq)]
pub struct RoutedEdge {
    pub points: Vec<Point>,
    pub label: Rect,
    pub reversed_for_ranking: bool,
}
/// Geometry is in SVG coordinates; nodes and edges retain their IR indices.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub nodes: Vec<Rect>,
    pub edges: Vec<RoutedEdge>,
    pub groups: Vec<Rect>,
    pub size: Size,
    pub ranks: Vec<usize>,
    pub crossings_before: usize,
    pub crossings_after: usize,
}
#[derive(Clone)]
struct Vertex {
    w: f64,
    h: f64,
    group: Option<usize>,
}

/// Layout a validated graph using exactly the metrics used by the renderer.
pub fn layout(graph: &Graph, metrics: &impl TextMeasure) -> Result<Layout, Diagnostic> {
    layout_with_node_sizes(graph, metrics, &[])
}
/// Use measured card/table minimum sizes while sharing hierarchical graph routing.
/// Empty sizes uses the standard node sizing; otherwise supply one size per node.
pub fn layout_with_node_sizes(
    graph: &Graph,
    metrics: &impl TextMeasure,
    sizes: &[Size],
) -> Result<Layout, Diagnostic> {
    graph.validate()?;
    if (!sizes.is_empty() && sizes.len() != graph.nodes.len())
        || sizes.iter().any(|s| {
            !s.width.is_finite()
                || !s.height.is_finite()
                || !(1.0..=65536.0).contains(&s.width)
                || !(1.0..=65536.0).contains(&s.height)
        })
    {
        return Err(Diagnostic::new(
            "invalid node minimum sizes",
            Span {
                line: 1,
                column: 1,
                ..Span::default()
            },
        ));
    }
    let fail = |s: String| {
        Diagnostic::new(
            s,
            Span {
                line: 1,
                column: 1,
                ..Span::default()
            },
        )
    };
    let group_order = graph.group_order();
    let mut group_lane = vec![0; graph.groups.len()];
    for (i, &g) in group_order.iter().enumerate() {
        group_lane[g] = i + 1;
    }
    let group_key = |g: Option<usize>| g.map_or(0, |g| group_lane[g]);
    let horizontal = matches!(graph.direction, Direction::LR | Direction::RL);
    let n = graph.nodes.len();
    let mut vertices = Vec::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        let sz = metrics
            .measure(&node.label, node.style.font_size.unwrap_or(15.0))
            .map_err(fail)?;
        let (mut w, mut h) = (sz.width + 32.0, sz.height + 24.0);
        match node.shape {
            Shape::Initial | Shape::Final => {
                w = 22.0;
                h = 22.0;
            }
            Shape::Bar => {
                w = if horizontal { 10.0 } else { 60.0 };
                h = if horizontal { 60.0 } else { 10.0 };
            }
            Shape::Diamond => {
                w = (w * 1.45).max(90.0);
                h = (h * 1.8).max(72.0);
            }
            Shape::Circle => {
                let d = w.hypot(h).max(60.0);
                w = d;
                h = d;
            }
            Shape::Stadium => {
                h = h.max(44.0);
                w = (sz.width + h + 16.0).max(h);
            }
            Shape::Subroutine => {
                w = (w + 20.0).max(80.0);
                h = h.max(44.0);
            }
            Shape::Cylinder => {
                w = w.max(60.0);
                h = (h + 40.0).max(84.0);
            }
            _ => {
                w = w.max(60.0);
                h = h.max(44.0);
            }
        }
        if !sizes.is_empty() {
            w = w.max(sizes[index].width);
            h = h.max(sizes[index].height);
        }
        let stroke = node.style.stroke_width.unwrap_or(1.5);
        w += stroke;
        h += stroke;
        vertices.push(Vertex {
            w: if horizontal { h } else { w },
            h: if horizontal { w } else { h },
            group: node.group,
        });
    }
    let mut outgoing = vec![Vec::new(); n];
    for (i, e) in graph.edges.iter().enumerate() {
        if e.source != e.target {
            outgoing[e.source].push((e.target, i));
        }
    }
    let mut colors = vec![0; n];
    let mut reversed = vec![false; graph.edges.len()];
    fn visit(v: usize, out: &[Vec<(usize, usize)>], colors: &mut [u8], rev: &mut [bool]) {
        colors[v] = 1;
        for &(w, i) in &out[v] {
            if colors[w] == 1 {
                rev[i] = true;
            } else if colors[w] == 0 {
                visit(w, out, colors, rev);
            }
        }
        colors[v] = 2;
    }
    for v in 0..n {
        if colors[v] == 0 {
            visit(v, &outgoing, &mut colors, &mut reversed);
        }
    }
    let mut dag = vec![Vec::new(); n];
    let mut indegree = vec![0; n];
    for (i, e) in graph.edges.iter().enumerate() {
        if e.source == e.target {
            continue;
        }
        let (a, b) = if reversed[i] {
            (e.target, e.source)
        } else {
            (e.source, e.target)
        };
        dag[a].push((b, e.min_rank_span));
        indegree[b] += 1;
    }
    let mut queue: VecDeque<_> = (0..n).filter(|&v| indegree[v] == 0).collect();
    let mut ranks = vec![0; n];
    while let Some(v) = queue.pop_front() {
        for &(w, span) in &dag[v] {
            ranks[w] = ranks[w].max(ranks[v] + span);
            indegree[w] -= 1;
            if indegree[w] == 0 {
                queue.push_back(w);
            }
        }
    }
    let max_rank = *ranks.iter().max().unwrap_or(&0);
    let mut layers = vec![Vec::new(); max_rank + 1];
    for (v, &r) in ranks.iter().enumerate() {
        layers[r].push(v);
    }
    let mut paths = Vec::new();
    let mut segments = Vec::new();
    let mut v_ranks = ranks.clone();
    for (i, e) in graph.edges.iter().enumerate() {
        if e.source == e.target {
            paths.push(vec![e.source]);
            continue;
        }
        let (a, b) = if reversed[i] {
            (e.target, e.source)
        } else {
            (e.source, e.target)
        };
        let mut path = vec![a];
        for (r, layer) in layers
            .iter_mut()
            .enumerate()
            .take(ranks[b])
            .skip(ranks[a] + 1)
        {
            if vertices.len() >= 8192 {
                return Err(fail(
                    "expanded layout exceeds 8192 vertices; reduce long edges".into(),
                ));
            }
            let v = vertices.len();
            vertices.push(Vertex {
                w: 20.0,
                h: 0.0,
                group: if graph.nodes[a].group == graph.nodes[b].group {
                    graph.nodes[a].group
                } else {
                    None
                },
            });
            v_ranks.push(r);
            layer.push(v);
            path.push(v);
        }
        path.push(b);
        for pair in path.windows(2) {
            segments.push((pair[0], pair[1]));
        }
        paths.push(path);
    }
    let mut incoming = vec![Vec::new(); vertices.len()];
    let mut next = vec![Vec::new(); vertices.len()];
    for &(a, b) in &segments {
        incoming[b].push(a);
        next[a].push(b);
    }
    let before = crossings(&layers, &segments, &v_ranks);
    let mut best = layers.clone();
    let mut best_count = before;
    for sweep in 0..12 {
        let mut positions = vec![0.0; vertices.len()];
        for layer in &layers {
            for (i, &v) in layer.iter().enumerate() {
                positions[v] = i as f64;
            }
        }
        let order: Vec<_> = if sweep % 2 == 0 {
            (1..layers.len()).collect()
        } else {
            (0..layers.len().saturating_sub(1)).rev().collect()
        };
        for r in order {
            let adj = if sweep % 2 == 0 { &incoming } else { &next };
            layers[r].sort_by(|&a, &b| {
                let center = |v: usize| {
                    if adj[v].is_empty() {
                        positions[v]
                    } else {
                        adj[v].iter().map(|&u| positions[u]).sum::<f64>() / adj[v].len() as f64
                    }
                };
                let grouping = if graph.groups.is_empty() {
                    std::cmp::Ordering::Equal
                } else {
                    group_key(vertices[a].group).cmp(&group_key(vertices[b].group))
                };
                grouping
                    .then_with(|| center(a).total_cmp(&center(b)))
                    .then(a.cmp(&b))
            });
            for (i, &v) in layers[r].iter().enumerate() {
                positions[v] = i as f64;
            }
        }
        let count = crossings(&layers, &segments, &v_ranks);
        if count < best_count {
            best = layers.clone();
            best_count = count;
        }
    }
    // Keep the best ordering; sweeps are not guaranteed to improve every graph.
    layers = best;
    if !graph.groups.is_empty() {
        for layer in &mut layers {
            layer.sort_by_key(|&v| group_key(vertices[v].group));
        }
        best_count = crossings(&layers, &segments, &v_ranks);
    }
    let mut labels = Vec::new();
    let mut gap = graph.config.rank_spacing;
    for edge in &graph.edges {
        let sz = metrics
            .measure(&edge.label, edge.style.font_size.unwrap_or(13.0))
            .map_err(fail)?;
        if !edge.label.is_empty() {
            gap = gap.max(if horizontal {
                sz.width + 36.0
            } else {
                sz.height + 36.0
            });
        }
        labels.push(sz);
    }
    let mut lane_widths = vec![0.0_f64; graph.groups.len() + 1];
    let lane = |v: usize| group_key(vertices[v].group);
    let sum_width = |vs: &[usize]| {
        vs.iter().map(|&v| vertices[v].w).sum::<f64>()
            + graph.config.node_spacing * vs.len().saturating_sub(1) as f64
    };
    if !graph.groups.is_empty() {
        for layer in &layers {
            for (g, width) in lane_widths.iter_mut().enumerate() {
                let vs: Vec<_> = layer.iter().copied().filter(|&v| lane(v) == g).collect();
                *width = (*width).max(sum_width(&vs) + if g > 0 { 112.0 } else { 0.0 });
            }
        }
        for (g, group) in graph.groups.iter().enumerate() {
            let sz = metrics.measure(&group.label, 14.0).map_err(fail)?;
            lane_widths[group_lane[g]] = lane_widths[group_lane[g]].max(if horizontal {
                sz.height + 112.0
            } else {
                sz.width + 64.0
            });
        }
    }
    let max_width = if graph.groups.is_empty() {
        layers.iter().map(|v| sum_width(v)).fold(0.0, f64::max)
    } else {
        lane_widths.iter().sum::<f64>() + 40.0 * graph.groups.len() as f64
    };
    let mut coords = vec![Point::default(); vertices.len()];
    let mut cursor_y = 60.0;
    for layer in &layers {
        let height = layer.iter().map(|&v| vertices[v].h).fold(0.0, f64::max);
        if graph.groups.is_empty() {
            let mut x = 40.0 + (max_width - sum_width(layer)) / 2.0;
            for &v in layer {
                coords[v] = Point {
                    x: x + vertices[v].w / 2.0,
                    y: cursor_y + height / 2.0,
                };
                x += vertices[v].w + graph.config.node_spacing;
            }
        } else {
            let mut lane_x = 40.0;
            for (g, &width) in lane_widths.iter().enumerate() {
                let vs: Vec<_> = layer.iter().copied().filter(|&v| lane(v) == g).collect();
                let mut x = lane_x + (width - sum_width(&vs)) / 2.0;
                for &v in &vs {
                    coords[v] = Point {
                        x: x + vertices[v].w / 2.0,
                        y: cursor_y + height / 2.0,
                    };
                    x += vertices[v].w + graph.config.node_spacing;
                }
                lane_x += width + 40.0;
            }
        }
        cursor_y += height + gap;
    }
    let mut nodes: Vec<Rect> = (0..n)
        .map(|v| Rect {
            x: coords[v].x - vertices[v].w / 2.0,
            y: coords[v].y - vertices[v].h / 2.0,
            width: vertices[v].w,
            height: vertices[v].h,
        })
        .collect();
    let mut parallel = BTreeMap::<(usize, usize), Vec<usize>>::new();
    for (i, e) in graph.edges.iter().enumerate() {
        parallel
            .entry((e.source.min(e.target), e.source.max(e.target)))
            .or_default()
            .push(i);
    }
    let mut channels = vec![(0.0, 1_usize); graph.edges.len()];
    for indices in parallel.values() {
        let count = indices.len();
        for (order, &i) in indices.iter().enumerate() {
            channels[i] = (
                if count == 1 {
                    0.0
                } else {
                    order as f64 / (count - 1) as f64 * 2.0 - 1.0
                },
                count,
            );
        }
    }
    let mut edges = Vec::new();
    for (i, e) in graph.edges.iter().enumerate() {
        let mut points = Vec::new();
        if e.source == e.target {
            let r = nodes[e.source];
            let c = r.center();
            let x = r.x
                + r.width
                + graph.config.node_spacing
                    * (0.65
                        + if channels[i].1 > 1 {
                            (channels[i].0 + 1.0) * 0.08
                        } else {
                            0.0
                        });
            points = vec![
                Point { x: c.x, y: c.y },
                Point { x, y: c.y },
                Point { x, y: r.y - 24.0 },
                Point {
                    x: c.x,
                    y: r.y - 24.0,
                },
                Point { x: c.x, y: c.y },
            ];
        } else {
            let path = &paths[i];
            points.push(coords[path[0]]);
            for pair in path.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                // A channel halfway between occupied layer bands keeps horizontal
                // segments outside every node in both adjacent ranks.
                let bottom = layers[v_ranks[a]]
                    .iter()
                    .map(|&v| coords[v].y + vertices[v].h / 2.0)
                    .fold(0.0, f64::max);
                let top = layers[v_ranks[b]]
                    .iter()
                    .map(|&v| coords[v].y - vertices[v].h / 2.0)
                    .fold(f64::INFINITY, f64::min);
                let channel = (bottom + top) / 2.0
                    + channels[i].0 * ((top - bottom) / 2.0 - 14.0).clamp(0.0, 16.0);
                if channels[i].1 > 1 && path.len() == 2 && (coords[a].x - coords[b].x).abs() < 0.01
                {
                    let trunk = coords[a].x
                        + channels[i].0 * (vertices[a].w.min(vertices[b].w) / 4.0).min(20.0);
                    points.extend([
                        Point {
                            x: coords[a].x,
                            y: bottom + 10.0,
                        },
                        Point {
                            x: trunk,
                            y: bottom + 10.0,
                        },
                        Point {
                            x: trunk,
                            y: top - 10.0,
                        },
                        Point {
                            x: coords[b].x,
                            y: top - 10.0,
                        },
                        coords[b],
                    ]);
                    continue;
                }
                points.push(Point {
                    x: coords[a].x,
                    y: channel,
                });
                points.push(Point {
                    x: coords[b].x,
                    y: channel,
                });
                points.push(coords[b]);
            }
            if reversed[i] {
                points.reverse();
            }
        }
        points.dedup();
        let label_sz = labels[i];
        let mut label = Rect::default();
        if !e.label.is_empty() {
            let middle = points.len() / 2;
            let p = points[middle];
            label = Rect {
                x: p.x
                    - (if horizontal {
                        label_sz.height
                    } else {
                        label_sz.width
                    }) / 2.0,
                y: p.y
                    - (if horizontal {
                        label_sz.width
                    } else {
                        label_sz.height
                    }) / 2.0,
                width: if horizontal {
                    label_sz.height
                } else {
                    label_sz.width
                },
                height: if horizontal {
                    label_sz.width
                } else {
                    label_sz.height
                },
            };
        }
        edges.push(RoutedEdge {
            points,
            label,
            reversed_for_ranking: reversed[i],
        });
    }
    let mut groups = Vec::new();
    for g in 0..graph.groups.len() {
        let members: Vec<_> = graph
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, v)| graph.group_contains(v.group, g))
            .map(|(i, _)| nodes[i])
            .collect();
        let x = 40.0
            + lane_widths[..group_lane[g]].iter().sum::<f64>()
            + 40.0 * group_lane[g] as f64
            + 8.0;
        let y = members.iter().map(|r| r.y).fold(f64::INFINITY, f64::min) - 48.0;
        let bottom = members.iter().map(|r| r.y + r.height).fold(0.0, f64::max) + 48.0;
        let title = metrics
            .measure(&graph.groups[g].label, 14.0)
            .map_err(fail)?;
        groups.push(Rect {
            x,
            y,
            width: (lane_widths[group_lane[g]] - 16.0).max(if horizontal {
                title.height + 48.0
            } else {
                title.width + 32.0
            }),
            height: (bottom - y).max(if horizontal {
                title.width + 48.0
            } else {
                title.height + 64.0
            }),
        });
    }
    // Transform points, retaining the original text orientation and node shapes.
    let extent = cursor_y - gap + 60.0;
    let transform = |p: Point| match graph.direction {
        Direction::TB => p,
        Direction::BT => Point {
            x: p.x,
            y: extent - p.y,
        },
        Direction::LR => Point { x: p.y, y: p.x },
        Direction::RL => Point {
            x: extent - p.y,
            y: p.x,
        },
    };
    let transform_rect = |r: Rect| {
        let a = transform(Point { x: r.x, y: r.y });
        let b = transform(Point {
            x: r.x + r.width,
            y: r.y + r.height,
        });
        Rect {
            x: a.x.min(b.x),
            y: a.y.min(b.y),
            width: (b.x - a.x).abs(),
            height: (b.y - a.y).abs(),
        }
    };
    for r in &mut nodes {
        *r = transform_rect(*r);
    }
    for r in &mut groups {
        *r = transform_rect(*r);
    }
    // Expand parent frames around their child frames after axis transformation.
    for &g in group_order.iter().rev() {
        let Some(parent) = graph.groups[g].parent else {
            continue;
        };
        let child = groups[g];
        let p = &mut groups[parent];
        let x = p.x.min(child.x - 32.0);
        let y = p.y.min(child.y - 48.0);
        let right = (p.x + p.width).max(child.x + child.width + 32.0);
        let bottom = (p.y + p.height).max(child.y + child.height + 32.0);
        *p = Rect {
            x,
            y,
            width: right - x,
            height: bottom - y,
        };
    }
    for (i, edge) in edges.iter_mut().enumerate() {
        for p in &mut edge.points {
            *p = transform(*p);
        }
        edge.label = transform_rect(edge.label);
        let last = edge.points.len() - 1;
        edge.points[0] = boundary(
            nodes[graph.edges[i].source],
            graph.nodes[graph.edges[i].source].shape,
            edge.points[1],
        );
        edge.points[last] = boundary(
            nodes[graph.edges[i].target],
            graph.nodes[graph.edges[i].target].shape,
            edge.points[last - 1],
        );
    }
    // Place labels on routing channels and separate colliding label boxes. If no
    // safe position is available, report it instead of emitting clipped text.
    let mut occupied = nodes.clone();
    for (g, r) in graph.groups.iter().zip(&groups) {
        let sz = metrics.measure(&g.label, 14.0).map_err(fail)?;
        occupied.push(Rect {
            x: r.center().x - sz.width / 2.0 - 4.0,
            y: r.y + 17.0 - sz.height / 2.0 - 3.0,
            width: sz.width + 8.0,
            height: sz.height + 6.0,
        });
    }
    // Group headings are visible obstacles too. Detour locally around a heading
    // while keeping every resulting segment outside nodes and other headings.
    for (i, edge) in edges.iter_mut().enumerate() {
        for &title in &occupied[n..] {
            let hits: Vec<_> = edge
                .points
                .windows(2)
                .enumerate()
                .filter(|(_, p)| segment_hits(p[0], p[1], title))
                .map(|(j, _)| j)
                .collect();
            if hits.is_empty() {
                continue;
            }
            let first = hits[0];
            let last = hits[hits.len() - 1] + 1;
            let a = edge.points[first];
            let b = edge.points[last];
            let mut candidates = Vec::new();
            let outer = |v: f64, min: f64, max: f64| {
                if v < min {
                    v.max(min - 8.0)
                } else if v > max {
                    v.min(max + 8.0)
                } else {
                    v
                }
            };
            let ay = outer(a.y, title.y, title.y + title.height);
            let by = outer(b.y, title.y, title.y + title.height);
            for x in [title.x - 8.0, title.x + title.width + 8.0] {
                candidates.push(vec![
                    a,
                    Point { x: a.x, y: ay },
                    Point { x, y: ay },
                    Point { x, y: by },
                    Point { x: b.x, y: by },
                    b,
                ]);
            }
            let ax = outer(a.x, title.x, title.x + title.width);
            let bx = outer(b.x, title.x, title.x + title.width);
            for y in [title.y - 8.0, title.y + title.height + 8.0] {
                candidates.push(vec![
                    a,
                    Point { x: ax, y: a.y },
                    Point { x: ax, y },
                    Point { x: bx, y },
                    Point { x: bx, y: b.y },
                    b,
                ]);
            }
            let candidate = candidates.into_iter().find(|path| {
                path.windows(2)
                    .all(|pair| occupied.iter().all(|&r| !segment_hits(pair[0], pair[1], r)))
            });
            let Some(path) = candidate else {
                return Err(Diagnostic::new(
                    "cannot route around subgraph title; increase spacing",
                    graph.edges[i].span,
                ));
            };
            edge.points.splice(first..=last, path);
            edge.points.dedup();
        }
    }
    for (i, edge) in edges.iter_mut().enumerate() {
        if graph.edges[i].label.is_empty() {
            continue;
        }
        let mut candidates = Vec::new();
        for &point in edge
            .points
            .iter()
            .rev()
            .skip(1)
            .take(edge.points.len().saturating_sub(2))
        {
            candidates.push((gap, point));
        }
        for pair in edge.points.windows(2) {
            let mid = Point {
                x: (pair[0].x + pair[1].x) / 2.0,
                y: (pair[0].y + pair[1].y) / 2.0,
            };
            let length = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
            candidates.push((length, mid));
        }
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
        let sz = labels[i];
        let mut placed = None;
        'search: for (_, mid) in candidates {
            for (dx, dy) in [
                (0.0, 0.0),
                (0.0, -sz.height - 8.0),
                (0.0, sz.height + 8.0),
                (sz.width / 2.0 + 12.0, 0.0),
                (-sz.width / 2.0 - 12.0, 0.0),
            ] {
                let r = Rect {
                    x: mid.x + dx - sz.width / 2.0 - 5.0,
                    y: mid.y + dy - sz.height / 2.0 - 3.0,
                    width: sz.width + 10.0,
                    height: sz.height + 6.0,
                };
                if occupied.iter().all(|&o| !r.overlaps(o)) {
                    placed = Some(r);
                    break 'search;
                }
            }
        }
        edge.label = placed.ok_or_else(|| {
            Diagnostic::new(
                "edge labels cannot be placed without overlap; increase spacing",
                graph.edges[i].span,
            )
        })?;
        occupied.push(edge.label);
    }
    let mut min_x = 0.0_f64;
    let mut min_y = 0.0_f64;
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for r in nodes.iter().chain(groups.iter()).chain(
        edges
            .iter()
            .filter(|e| e.label.width > 0.0)
            .map(|e| &e.label),
    ) {
        min_x = min_x.min(r.x);
        min_y = min_y.min(r.y);
        max_x = max_x.max(r.x + r.width);
        max_y = max_y.max(r.y + r.height);
    }
    for e in &edges {
        for p in &e.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    let (dx, dy) = (24.0 - min_x, 24.0 - min_y);
    for r in nodes.iter_mut().chain(groups.iter_mut()) {
        r.x += dx;
        r.y += dy;
    }
    for e in &mut edges {
        for p in &mut e.points {
            p.x += dx;
            p.y += dy;
        }
        if e.label.width > 0.0 {
            e.label.x += dx;
            e.label.y += dy;
        }
    }
    Ok(Layout {
        nodes,
        edges,
        groups,
        size: Size {
            width: max_x - min_x + 48.0,
            height: max_y - min_y + 48.0,
        },
        ranks,
        crossings_before: before,
        crossings_after: best_count,
    })
}
fn crossings(layers: &[Vec<usize>], segments: &[(usize, usize)], ranks: &[usize]) -> usize {
    let mut pos = vec![0; ranks.len()];
    for layer in layers {
        for (i, &v) in layer.iter().enumerate() {
            pos[v] = i;
        }
    }
    let mut total = 0;
    for r in 0..layers.len().saturating_sub(1) {
        let mut pairs: Vec<_> = segments
            .iter()
            .filter(|&&(a, _)| ranks[a] == r)
            .map(|&(a, b)| (pos[a], pos[b]))
            .collect();
        pairs.sort_unstable();
        let mut tree = vec![0; layers[r + 1].len() + 2];
        let mut seen = 0;
        let mut i = 0;
        while i < pairs.len() {
            let mut j = i + 1;
            while j < pairs.len() && pairs[j].0 == pairs[i].0 {
                j += 1;
            }
            for &(_, p) in &pairs[i..j] {
                let mut k = p + 1;
                let mut prefix = 0;
                while k > 0 {
                    prefix += tree[k];
                    k &= k - 1;
                }
                total += seen - prefix;
            }
            for &(_, p) in &pairs[i..j] {
                let mut k = p + 1;
                while k < tree.len() {
                    tree[k] += 1;
                    k += k & k.wrapping_neg();
                }
                seen += 1;
            }
            i = j;
        }
    }
    total
}
/// Intersection of a ray from a node center with its visible geometry.
fn boundary(r: Rect, shape: Shape, toward: Point) -> Point {
    let c = r.center();
    let dx = toward.x - c.x;
    let dy = toward.y - c.y;
    let (rx, ry) = (r.width / 2.0, r.height / 2.0);
    let scale = match shape {
        Shape::Circle | Shape::Initial | Shape::Final => {
            1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt()
        }
        Shape::Diamond => 1.0 / (dx.abs() / rx + dy.abs() / ry),
        _ => 1.0 / (dx.abs() / rx).max(dy.abs() / ry),
    };
    if scale.is_finite() {
        Point {
            x: c.x + dx * scale,
            y: c.y + dy * scale,
        }
    } else {
        c
    }
}

// Intersections with an open rectangle; touching a port/boundary is permitted.
fn segment_hits(a: Point, b: Point, r: Rect) -> bool {
    let eps = 0.001;
    if (a.x - b.x).abs() < eps {
        a.x > r.x + eps
            && a.x < r.x + r.width - eps
            && a.y.min(b.y) < r.y + r.height - eps
            && a.y.max(b.y) > r.y + eps
    } else {
        a.y > r.y + eps
            && a.y < r.y + r.height - eps
            && a.x.min(b.x) < r.x + r.width - eps
            && a.x.max(b.x) > r.x + eps
    }
}

pub mod gantt;
pub mod sequence;
