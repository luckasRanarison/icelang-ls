use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

#[derive(Debug, Clone)]
pub enum ErrorKind {
    SyntaxError,
    Unexpected,
    ExpectedExpr,
    ExpectedField,
    Missing(String),
    Redeclaration(String),
    Undeclared(String),
}

impl ToString for ErrorKind {
    fn to_string(&self) -> String {
        match self {
            ErrorKind::SyntaxError => "Syntax error".to_owned(),
            ErrorKind::Unexpected => "Unexpected token".to_owned(),
            ErrorKind::ExpectedExpr => "Expected expression".to_owned(),
            ErrorKind::ExpectedField => "Expected field name".to_owned(),
            ErrorKind::Missing(str) => format!("Missing '{}'", str),
            ErrorKind::Redeclaration(str) => format!("Redeclaring existing identifier '{}'", str),
            ErrorKind::Undeclared(str) => format!("Undeclared identifier '{}'", str),
        }
    }
}

pub fn error(kind: ErrorKind, range: Range) -> Diagnostic {
    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("icelang_ls".to_owned()),
        message: kind.to_string(),
        ..Default::default()
    }
}
