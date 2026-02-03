/// DAG layout engine using a simplified Sugiyama algorithm.
///
/// Produces a layered graph layout suitable for rendering with Unicode
/// box-drawing characters in a terminal. The algorithm:
///
/// 1. Layer assignment: longest-path from sources (topological depth)
/// 2. Crossing minimization: barycenter heuristic (2 passes)
/// 3. Coordinate assignment: greedy left-to-right packing per layer
/// 4. Edge routing: vertical/horizontal segments with box-drawing chars

use std::collections::{HashMap, HashSet, VecDeque};

use workgraph::graph::{Status, Task, WorkGraph};

use super::app::TaskAgentInfo;

// ── Public types ────────────────────────────────────────────────────────

/// A positioned node in the layout, ready for rendering.
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub task_id: String,
    pub title: String,
    pub status: Status,
    pub assigned: Option<String>,
    pub critical: bool,
    pub active_agent_count: usize,
    pub active_agent_ids: Vec<String>,
    /// Layer index (0 = top/sources)
    pub layer: usize,
    /// Position within layer (0-based, left to right)
    pub order: usize,
    /// Character column of the left edge of the box
    pub x: usize,
    /// Character row of the top edge of the box
    pub y: usize,
    /// Width of the box in characters (including border)
    pub w: usize,
    /// Height of the box (always 3: top border, content, bottom border)
    pub h: usize,
}

/// An edge between two nodes.
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from_id: String,
    pub to_id: String,
    /// Segments to draw: list of (x, y) points forming a polyline
    pub segments: Vec<(usize, usize)>,
}

/// The complete layout result.
#[derive(Debug)]
pub struct DagLayout {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    /// Total width of the layout canvas in characters
    pub width: usize,
    /// Total height of the layout canvas in characters
    pub height: usize,
    /// Mapping from task_id to node index
    pub id_to_idx: HashMap<String, usize>,
}

// ── Configuration ───────────────────────────────────────────────────────

/// Minimum horizontal gap between node boxes in the same layer
const H_GAP: usize = 2;
/// Vertical gap between layers (rows between bottom of one layer and top of next)
const V_GAP: usize = 2;
/// Node box height (top border + content + bottom border)
const NODE_HEIGHT: usize = 3;
/// Minimum node box width (including borders)
const MIN_NODE_WIDTH: usize = 10;
/// Maximum node box width (including borders)
const MAX_NODE_WIDTH: usize = 40;
/// Left margin
const LEFT_MARGIN: usize = 1;
/// Top margin
const TOP_MARGIN: usize = 0;

// ── Layout computation ─────────────────────────────────────────────────

impl DagLayout {
    /// Compute a layered DAG layout from the work graph.
    pub fn compute(
        graph: &WorkGraph,
        critical_ids: &HashSet<String>,
        agent_map: &HashMap<String, TaskAgentInfo>,
    ) -> Self {
        let tasks: HashMap<String, &Task> = graph.tasks().map(|t| (t.id.clone(), t)).collect();

        if tasks.is_empty() {
            return Self {
                nodes: Vec::new(),
                edges: Vec::new(),
                width: 0,
                height: 0,
                id_to_idx: HashMap::new(),
            };
        }

        // Build adjacency: parent -> children (task blocks child)
        // blocked_by[child] = parents, so children[parent] = child
        let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut parents: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut all_ids: Vec<&str> = Vec::new();

        for task in tasks.values() {
            all_ids.push(&task.id);
            for blocker_id in &task.blocked_by {
                if tasks.contains_key(blocker_id) {
                    children
                        .entry(blocker_id.as_str())
                        .or_default()
                        .push(&task.id);
                    parents
                        .entry(task.id.as_str())
                        .or_default()
                        .push(blocker_id.as_str());
                }
            }
        }

        // 1. Layer assignment via longest-path from sources
        let layers = assign_layers(&all_ids, &children, &parents);

        // Group nodes by layer
        let max_layer = layers.values().copied().max().unwrap_or(0);
        let mut layer_groups: Vec<Vec<&str>> = vec![Vec::new(); max_layer + 1];
        for &id in &all_ids {
            let layer = layers.get(id).copied().unwrap_or(0);
            layer_groups[layer].push(id);
        }

        // 2. Initial ordering: sort within each layer by status priority then title
        for group in &mut layer_groups {
            group.sort_by(|a, b| {
                let ta = tasks.get(*a);
                let tb = tasks.get(*b);
                match (ta, tb) {
                    (Some(a), Some(b)) => sort_key_for_status(&a.status)
                        .cmp(&sort_key_for_status(&b.status))
                        .then(a.title.cmp(&b.title)),
                    _ => a.cmp(b),
                }
            });
        }

        // 3. Crossing minimization via barycenter heuristic
        minimize_crossings(&mut layer_groups, &children, &parents);

        // 4. Compute node widths (based on title + status indicator)
        let node_widths: HashMap<&str, usize> = all_ids
            .iter()
            .map(|&id| {
                let task = &tasks[id];
                let indicator = status_indicator_str(&task.status);
                // Box content: " indicator title "
                let content_width = indicator.len() + 1 + task.title.len() + 2;
                // Add 2 for box borders (│ on each side)
                let w = (content_width + 2).max(MIN_NODE_WIDTH).min(MAX_NODE_WIDTH);
                (id, w)
            })
            .collect();

        // 5. Coordinate assignment
        let (positioned_nodes, total_width, total_height) = assign_coordinates(
            &layer_groups,
            &node_widths,
            &tasks,
            critical_ids,
            agent_map,
            &layers,
        );

        // Build id_to_idx map
        let id_to_idx: HashMap<String, usize> = positioned_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.task_id.clone(), i))
            .collect();

        // 6. Route edges
        let edges = route_edges(&positioned_nodes, &id_to_idx, &tasks);

        Self {
            nodes: positioned_nodes,
            edges,
            width: total_width,
            height: total_height,
            id_to_idx,
        }
    }

    /// Find a node by task_id
    pub fn find_node(&self, task_id: &str) -> Option<&LayoutNode> {
        self.id_to_idx
            .get(task_id)
            .and_then(|&idx| self.nodes.get(idx))
    }
}

// ── Layer assignment (longest path from sources) ────────────────────────

fn assign_layers<'a>(
    all_ids: &[&'a str],
    children: &HashMap<&str, Vec<&'a str>>,
    parents: &HashMap<&str, Vec<&'a str>>,
) -> HashMap<&'a str, usize> {
    let mut layers: HashMap<&str, usize> = HashMap::new();
    let mut in_degree: HashMap<&str, usize> = HashMap::new();

    for &id in all_ids {
        let deg = parents.get(id).map(|p| p.len()).unwrap_or(0);
        in_degree.insert(id, deg);
    }

    // BFS from sources (nodes with in_degree 0)
    let mut queue: VecDeque<&str> = VecDeque::new();
    for &id in all_ids {
        if in_degree[id] == 0 {
            queue.push_back(id);
            layers.insert(id, 0);
        }
    }

    while let Some(node) = queue.pop_front() {
        let node_layer = layers[node];
        if let Some(kids) = children.get(node) {
            for &kid in kids {
                // Assign max depth (longest path)
                let new_layer = node_layer + 1;
                let current = layers.entry(kid).or_insert(0);
                if new_layer > *current {
                    *current = new_layer;
                }
                // Decrement in-degree; add to queue when all parents processed
                let deg = in_degree.get_mut(kid).unwrap();
                *deg = deg.saturating_sub(1);
                if *deg == 0 {
                    queue.push_back(kid);
                }
            }
        }
    }

    // Handle any unvisited nodes (cycles) - assign to layer 0
    for &id in all_ids {
        layers.entry(id).or_insert(0);
    }

    layers
}

// ── Crossing minimization (barycenter heuristic) ────────────────────────

fn minimize_crossings<'a>(
    layer_groups: &mut [Vec<&'a str>],
    children: &HashMap<&str, Vec<&'a str>>,
    parents: &HashMap<&str, Vec<&'a str>>,
) {
    if layer_groups.len() <= 1 {
        return;
    }

    // Build position lookup for each layer
    let mut positions: Vec<HashMap<&str, usize>> = layer_groups
        .iter()
        .map(|group| {
            group
                .iter()
                .enumerate()
                .map(|(i, &id)| (id, i))
                .collect()
        })
        .collect();

    // Forward pass: order each layer based on parents in previous layer
    for layer_idx in 1..layer_groups.len() {
        let prev_positions = &positions[layer_idx - 1];
        let mut barycenters: Vec<(&str, f64)> = layer_groups[layer_idx]
            .iter()
            .map(|&id| {
                let bc = if let Some(pars) = parents.get(id) {
                    let parent_positions: Vec<f64> = pars
                        .iter()
                        .filter_map(|&p| prev_positions.get(p).map(|&pos| pos as f64))
                        .collect();
                    if parent_positions.is_empty() {
                        f64::MAX
                    } else {
                        parent_positions.iter().sum::<f64>() / parent_positions.len() as f64
                    }
                } else {
                    f64::MAX
                };
                (id, bc)
            })
            .collect();

        barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        layer_groups[layer_idx] = barycenters.into_iter().map(|(id, _)| id).collect();

        // Update positions for this layer
        positions[layer_idx] = layer_groups[layer_idx]
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();
    }

    // Backward pass: order each layer based on children in next layer
    for layer_idx in (0..layer_groups.len() - 1).rev() {
        let next_positions = &positions[layer_idx + 1];
        let mut barycenters: Vec<(&str, f64)> = layer_groups[layer_idx]
            .iter()
            .map(|&id| {
                let bc = if let Some(kids) = children.get(id) {
                    let child_positions: Vec<f64> = kids
                        .iter()
                        .filter_map(|&c| next_positions.get(c).map(|&pos| pos as f64))
                        .collect();
                    if child_positions.is_empty() {
                        f64::MAX
                    } else {
                        child_positions.iter().sum::<f64>() / child_positions.len() as f64
                    }
                } else {
                    f64::MAX
                };
                (id, bc)
            })
            .collect();

        barycenters.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        layer_groups[layer_idx] = barycenters.into_iter().map(|(id, _)| id).collect();

        positions[layer_idx] = layer_groups[layer_idx]
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();
    }
}

// ── Coordinate assignment ───────────────────────────────────────────────

fn assign_coordinates(
    layer_groups: &[Vec<&str>],
    node_widths: &HashMap<&str, usize>,
    tasks: &HashMap<String, &Task>,
    critical_ids: &HashSet<String>,
    agent_map: &HashMap<String, TaskAgentInfo>,
    layers: &HashMap<&str, usize>,
) -> (Vec<LayoutNode>, usize, usize) {
    let mut nodes = Vec::new();
    let mut total_width: usize = 0;
    let mut y = TOP_MARGIN;

    for (layer_idx, group) in layer_groups.iter().enumerate() {
        if group.is_empty() {
            continue;
        }

        // Compute total width of this layer
        let layer_width: usize = group
            .iter()
            .map(|&id| node_widths.get(id).copied().unwrap_or(MIN_NODE_WIDTH))
            .sum::<usize>()
            + H_GAP * group.len().saturating_sub(1)
            + LEFT_MARGIN;

        total_width = total_width.max(layer_width + LEFT_MARGIN);

        let mut x = LEFT_MARGIN;
        for (order, &id) in group.iter().enumerate() {
            let task = &tasks[id];
            let w = node_widths.get(id).copied().unwrap_or(MIN_NODE_WIDTH);
            let (agent_count, agent_ids) = agent_map
                .get(id)
                .map(|info| (info.count, info.agent_ids.clone()))
                .unwrap_or((0, Vec::new()));

            nodes.push(LayoutNode {
                task_id: id.to_string(),
                title: task.title.clone(),
                status: task.status.clone(),
                assigned: task.assigned.clone(),
                critical: critical_ids.contains(id),
                active_agent_count: agent_count,
                active_agent_ids: agent_ids,
                layer: layers.get(id).copied().unwrap_or(layer_idx),
                order,
                x,
                y,
                w,
                h: NODE_HEIGHT,
            });

            x += w + H_GAP;
        }

        y += NODE_HEIGHT + V_GAP;
    }

    let total_height = if y > V_GAP { y - V_GAP } else { y };
    (nodes, total_width, total_height)
}

// ── Edge routing ────────────────────────────────────────────────────────

fn route_edges(
    nodes: &[LayoutNode],
    id_to_idx: &HashMap<String, usize>,
    tasks: &HashMap<String, &Task>,
) -> Vec<LayoutEdge> {
    let mut edges = Vec::new();

    for node in nodes {
        let task = match tasks.get(&node.task_id) {
            Some(t) => t,
            None => continue,
        };

        for blocker_id in &task.blocked_by {
            let parent_idx = match id_to_idx.get(blocker_id) {
                Some(&i) => i,
                None => continue,
            };
            let parent = &nodes[parent_idx];

            // Route from bottom-center of parent to top-center of child
            let from_x = parent.x + parent.w / 2;
            let from_y = parent.y + parent.h - 1; // bottom border row

            let to_x = node.x + node.w / 2;
            let to_y = node.y; // top border row

            let mut segments = Vec::new();
            segments.push((from_x, from_y));

            // If not vertically aligned, route through a midpoint
            if from_x != to_x {
                let mid_y = from_y + (to_y - from_y) / 2;
                segments.push((from_x, mid_y));
                segments.push((to_x, mid_y));
            }

            segments.push((to_x, to_y));

            edges.push(LayoutEdge {
                from_id: blocker_id.clone(),
                to_id: node.task_id.clone(),
                segments,
            });
        }
    }

    edges
}

// ── Rendering to a character buffer ─────────────────────────────────────

/// A character cell in the render buffer.
#[derive(Clone)]
pub struct Cell {
    pub ch: char,
    pub style: CellStyle,
}

/// Style for a cell in the render buffer.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CellStyle {
    /// Default/empty
    Empty,
    /// Node box border
    Border,
    /// Node content text
    NodeText,
    /// Node content text for active agent
    ActiveAgent,
    /// Node content text for critical path
    Critical,
    /// Node content for done/abandoned
    Dimmed,
    /// Edge line
    Edge,
    /// Edge arrow
    Arrow,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::Empty,
        }
    }
}

/// Render the DAG layout into a 2D character buffer.
pub fn render_to_buffer(layout: &DagLayout) -> Vec<Vec<Cell>> {
    if layout.width == 0 || layout.height == 0 {
        return Vec::new();
    }

    let width = layout.width + 2; // some extra padding
    let height = layout.height + 1;
    let mut buf: Vec<Vec<Cell>> = vec![vec![Cell::default(); width]; height];

    // Draw edges first (so nodes are drawn on top)
    for edge in &layout.edges {
        draw_edge(&mut buf, &edge.segments, width, height);
    }

    // Draw nodes
    for node in &layout.nodes {
        draw_node(&mut buf, node, width, height);
    }

    buf
}

fn draw_node(buf: &mut [Vec<Cell>], node: &LayoutNode, buf_width: usize, buf_height: usize) {
    let x = node.x;
    let y = node.y;
    let w = node.w;
    let h = node.h;

    if y + h > buf_height || x + w > buf_width {
        return;
    }

    let style = if node.active_agent_count > 0 {
        CellStyle::ActiveAgent
    } else if node.critical {
        CellStyle::Critical
    } else if matches!(node.status, Status::Done | Status::Abandoned) {
        CellStyle::Dimmed
    } else {
        CellStyle::NodeText
    };

    // Top border: ┌───┐
    set_cell(buf, x, y, '┌', CellStyle::Border);
    for cx in (x + 1)..(x + w - 1) {
        set_cell(buf, cx, y, '─', CellStyle::Border);
    }
    set_cell(buf, x + w - 1, y, '┐', CellStyle::Border);

    // Middle row(s): │ content │
    let content_y = y + 1;
    set_cell(buf, x, content_y, '│', CellStyle::Border);
    set_cell(buf, x + w - 1, content_y, '│', CellStyle::Border);

    // Content: status indicator + title, truncated to fit
    let indicator = status_indicator_str(&node.status);
    let max_content = w.saturating_sub(3); // 1 for left padding, 2 for borders
    let full_content = format!("{} {}", indicator, node.title);
    let content: String = if full_content.len() > max_content {
        format!("{}…", &full_content[..max_content.saturating_sub(1)])
    } else {
        full_content
    };

    // Write content starting at x+1 (after left border), padded with spaces
    for (i, ch) in content.chars().enumerate() {
        let cx = x + 1 + i;
        if cx < x + w - 1 {
            set_cell(buf, cx, content_y, ch, style);
        }
    }
    // Fill remaining with spaces
    for cx in (x + 1 + content.len())..(x + w - 1) {
        set_cell(buf, cx, content_y, ' ', style);
    }

    // Bottom border: └───┘
    let bottom_y = y + h - 1;
    set_cell(buf, x, bottom_y, '└', CellStyle::Border);
    for cx in (x + 1)..(x + w - 1) {
        set_cell(buf, cx, bottom_y, '─', CellStyle::Border);
    }
    set_cell(buf, x + w - 1, bottom_y, '┘', CellStyle::Border);
}

fn draw_edge(buf: &mut [Vec<Cell>], segments: &[(usize, usize)], buf_width: usize, buf_height: usize) {
    if segments.len() < 2 {
        return;
    }

    for i in 0..segments.len() - 1 {
        let (x1, y1) = segments[i];
        let (x2, y2) = segments[i + 1];

        if x1 == x2 {
            // Vertical segment
            let min_y = y1.min(y2);
            let max_y = y1.max(y2);
            for cy in min_y..=max_y {
                if cy < buf_height && x1 < buf_width {
                    let existing = buf[cy][x1].ch;
                    let ch = match existing {
                        '─' | '┼' => '┼',
                        '┐' | '┘' | '┌' | '└' | '│' | '├' | '┤' | '┬' | '┴' => existing,
                        _ => '│',
                    };
                    set_cell_if_empty_or_edge(buf, x1, cy, ch, CellStyle::Edge);
                }
            }
        } else if y1 == y2 {
            // Horizontal segment
            let min_x = x1.min(x2);
            let max_x = x1.max(x2);
            for cx in min_x..=max_x {
                if y1 < buf_height && cx < buf_width {
                    let existing = buf[y1][cx].ch;
                    let ch = match existing {
                        '│' | '┼' => '┼',
                        '┐' | '┘' | '┌' | '└' | '├' | '┤' | '┬' | '┴' => existing,
                        _ => '─',
                    };
                    set_cell_if_empty_or_edge(buf, cx, y1, ch, CellStyle::Edge);
                }
            }
        }

        // Draw corners at segment junctions
        if i + 1 < segments.len() - 1 {
            let (nx, ny) = segments[i + 1];
            if i + 2 < segments.len() {
                let (nnx, _nny) = segments[i + 2];
                // We're at a corner point (x2, y2) between two segments
                let corner = if x1 == x2 && y2 == ny && nx != x2 {
                    // Vertical then horizontal
                    if y1 < y2 {
                        // Going down then...
                        if nnx > x2 { '└' } else { '┘' }
                    } else {
                        // Going up then...
                        if nnx > x2 { '┌' } else { '┐' }
                    }
                } else if y1 == y2 && x2 == nx && ny != y2 {
                    // Horizontal then vertical
                    if x1 < x2 {
                        // Going right then...
                        if ny > y2 { '┐' } else { '┘' }
                    } else {
                        // Going left then...
                        if ny > y2 { '┌' } else { '└' }
                    }
                } else {
                    continue;
                };

                if x2 < buf_width && y2 < buf_height {
                    set_cell_if_empty_or_edge(buf, x2, y2, corner, CellStyle::Edge);
                }
            }
        }
    }

    // Draw arrow at the end (▼ pointing into the target node)
    if let Some(&(x, y)) = segments.last() {
        if y > 0 && y - 1 < buf_height && x < buf_width {
            // Place arrow one row above the target node top border
            let arrow_y = y.saturating_sub(1);
            if buf[arrow_y][x].style == CellStyle::Empty || buf[arrow_y][x].style == CellStyle::Edge {
                set_cell(buf, x, arrow_y, '▼', CellStyle::Arrow);
            }
        }
    }
}

fn set_cell(buf: &mut [Vec<Cell>], x: usize, y: usize, ch: char, style: CellStyle) {
    if y < buf.len() && x < buf[0].len() {
        buf[y][x] = Cell { ch, style };
    }
}

fn set_cell_if_empty_or_edge(buf: &mut [Vec<Cell>], x: usize, y: usize, ch: char, style: CellStyle) {
    if y < buf.len() && x < buf[0].len() {
        let existing = &buf[y][x];
        if existing.style == CellStyle::Empty || existing.style == CellStyle::Edge {
            buf[y][x] = Cell { ch, style };
        }
    }
}

fn status_indicator_str(status: &Status) -> &'static str {
    match status {
        Status::Done => "✓",
        Status::InProgress => "~",
        Status::Open => "○",
        Status::Failed => "!",
        Status::Blocked => "B",
        Status::PendingReview => "?",
        Status::Abandoned => "-",
    }
}

fn sort_key_for_status(status: &Status) -> u8 {
    match status {
        Status::InProgress => 0,
        Status::Open => 1,
        Status::PendingReview => 2,
        Status::Failed => 3,
        Status::Blocked => 4,
        Status::Done => 5,
        Status::Abandoned => 6,
    }
}

// ── Centering helper ────────────────────────────────────────────────────

/// After initial layout, center each layer horizontally relative to the widest layer.
pub fn center_layers(layout: &mut DagLayout) {
    if layout.nodes.is_empty() {
        return;
    }

    // Group nodes by layer
    let max_layer = layout.nodes.iter().map(|n| n.layer).max().unwrap_or(0);
    let mut layer_extents: Vec<(usize, usize)> = vec![(usize::MAX, 0); max_layer + 1];

    for node in &layout.nodes {
        let layer = node.layer;
        layer_extents[layer].0 = layer_extents[layer].0.min(node.x);
        layer_extents[layer].1 = layer_extents[layer].1.max(node.x + node.w);
    }

    // Find widest layer
    let max_width = layer_extents
        .iter()
        .filter(|(min, _)| *min != usize::MAX)
        .map(|(min, max)| max - min)
        .max()
        .unwrap_or(0);

    // Center each layer
    for node in &mut layout.nodes {
        let layer = node.layer;
        let (min_x, max_x) = layer_extents[layer];
        if min_x == usize::MAX {
            continue;
        }
        let layer_width = max_x - min_x;
        let offset = (max_width - layer_width) / 2;
        // Shift relative to the current layer's minimum x
        node.x = node.x - min_x + offset + LEFT_MARGIN;
    }

    // Recompute total width
    layout.width = layout
        .nodes
        .iter()
        .map(|n| n.x + n.w)
        .max()
        .unwrap_or(0)
        + LEFT_MARGIN;

    // Re-route edges after centering
    let tasks_map: HashMap<String, ()> = layout.nodes.iter().map(|n| (n.task_id.clone(), ())).collect();
    let _ = tasks_map; // We'll rebuild edges below

    // Rebuild id_to_idx after potential reordering
    layout.id_to_idx = layout
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.task_id.clone(), i))
        .collect();
}

/// Re-route all edges based on current node positions.
pub fn reroute_edges(layout: &mut DagLayout, graph: &WorkGraph) {
    let tasks: HashMap<String, &Task> = graph.tasks().map(|t| (t.id.clone(), t)).collect();
    layout.edges = route_edges(&layout.nodes, &layout.id_to_idx, &tasks);
}
