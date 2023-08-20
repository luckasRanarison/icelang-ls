use tower_lsp::lsp_types::{Position, Range};

#[derive(Clone)]
pub enum VariableType {
    Any,
    Null,
    Boolean,
    Number,
    String,
    Object,
    Function,
}

#[derive(Clone)]
pub enum DeclarationKind {
    Variable(Option<VariableType>),
    Function(Vec<String>),
}

#[derive(Clone)]
pub struct Declaration {
    pub kind: DeclarationKind,
    pub end_pos: Position,
    pub scope: Option<Range>,
}
