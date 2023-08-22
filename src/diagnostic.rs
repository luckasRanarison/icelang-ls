use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, Range};

#[derive(Debug, Clone)]
pub enum ErrorKind {
    SyntaxError,
    Unexpected,
    ExpectedExpr,
    ExpectedField,
    Missing(String),
    Redeclaration(String),
    Undeclared(String),
    ContinueOutside,
    BreakOutside,
    ReturnOutside,
}

impl ToString for ErrorKind {
    fn to_string(&self) -> String {
        match self {
            ErrorKind::SyntaxError => "Syntax error".to_owned(),
            ErrorKind::Unexpected => "Unexpected token".to_owned(),
            ErrorKind::ExpectedExpr => "Expected expression".to_owned(),
            ErrorKind::ExpectedField => "Expected field name".to_owned(),
            ErrorKind::Missing(str) => format!("Missing '{}'", str),
            ErrorKind::Redeclaration(str) => {
                format!("Redeclaring existing identifier '{}'", str)
            }
            ErrorKind::Undeclared(str) => format!("Undeclared identifier '{}'", str),
            ErrorKind::ContinueOutside => "continue outside of a loop".to_owned(),
            ErrorKind::BreakOutside => "break outside of a loop".to_owned(),
            ErrorKind::ReturnOutside => "return outside of a function".to_owned(),
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

pub enum HintKind {
    Unreachable,
}

impl ToString for HintKind {
    fn to_string(&self) -> String {
        match self {
            HintKind::Unreachable => "Unreachable code".to_string(),
        }
    }
}

pub fn hint(kind: HintKind, range: Range) -> Diagnostic {
    let tags = match kind {
        HintKind::Unreachable => vec![DiagnosticTag::UNNECESSARY],
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::HINT),
        source: Some("icelang_ls".to_owned()),
        message: kind.to_string(),
        tags: Some(tags),
        ..Default::default()
    }
}
