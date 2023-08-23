use tower_lsp::lsp_types::{Position, Range};
use tree_sitter::{Node, Point};

pub fn point_to_position(point: Point) -> Position {
    Position::new(point.row as u32, point.column as u32)
}

pub fn get_node_range(node: &Node) -> Range {
    let start = point_to_position(node.start_position());
    let end = point_to_position(node.end_position());

    Range::new(start, end)
}

pub fn tsrange_to_lsprange(range: tree_sitter::Range) -> Range {
    let start = point_to_position(range.start_point);
    let end = point_to_position(range.end_point);
    Range::new(start, end)
}
