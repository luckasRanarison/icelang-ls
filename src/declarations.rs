use std::{collections::HashMap, vec};

use tower_lsp::lsp_types::{Documentation, MarkupContent, MarkupKind, Position, Range};

use crate::{
    analyzer::IdentiferData,
    builtins::{BuiltinFn, BUILTIN_FUNCTION},
    utils::NIL_RANGE,
};

#[derive(Debug, Clone)]
pub enum VariableType {
    Any,
    Null,
    Boolean,
    Number,
    String,
    Range,
    Array,
    Object(Vec<String>),
    Function(Vec<String>),
}

impl ToString for VariableType {
    fn to_string(&self) -> String {
        match self {
            VariableType::Any => "any".to_owned(),
            VariableType::Null => "null".to_owned(),
            VariableType::Boolean => "boolean".to_owned(),
            VariableType::Number => "number".to_owned(),
            VariableType::String => "string".to_owned(),
            VariableType::Range => "range".to_owned(),
            VariableType::Array => "array".to_owned(),
            VariableType::Object(props) => format!("object {{ {} }}", props.join(", ")),
            VariableType::Function(args) => format!("function ({})", args.join(", ")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeclarationKind {
    Variable(VariableType),
    Function(Vec<String>),
}

impl Declaration {
    pub fn get_details(&self) -> String {
        if self.param {
            format!("parameter: {} -- any", &self.name)
        } else {
            match &self.kind {
                DeclarationKind::Variable(value) => {
                    format!("variable: {} -- {}", &self.name, value.to_string())
                }
                DeclarationKind::Function(args) => {
                    format!(
                        "function {}({}) {{}}{}",
                        &self.name,
                        args.join(", "),
                        match self.builtin {
                            true => " -- builtin function",
                            _ => "",
                        }
                    )
                }
            }
        }
    }
}

impl DeclarationKind {
    pub fn is_function(&self) -> bool {
        match self {
            DeclarationKind::Function(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub name: String,
    pub name_range: Range,
    pub kind: DeclarationKind,
    pub doc: Option<Documentation>,
    range: Range,
    scope: Option<Range>,
    used: bool,
    builtin: bool,
    param: bool,
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.scope == other.scope
    }
}

impl Declaration {
    pub fn new(
        name: String,
        kind: DeclarationKind,
        range: Range,
        name_range: Range,
        scope: Option<Range>,
        is_param: bool,
    ) -> Declaration {
        Self {
            name,
            kind,
            doc: None,
            range,
            name_range,
            scope,
            used: false,
            builtin: false,
            param: is_param,
        }
    }
}

impl From<&BuiltinFn> for Declaration {
    fn from(value: &BuiltinFn) -> Self {
        Declaration {
            name: value.name.clone(),
            kind: DeclarationKind::Function(value.args.clone()),
            doc: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: value.doc.clone(),
            })),
            range: *NIL_RANGE,
            name_range: *NIL_RANGE,
            scope: None,
            used: true,
            builtin: true,
            param: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclarationMap {
    map: HashMap<String, Vec<Declaration>>,
}

impl DeclarationMap {
    pub fn new() -> Self {
        let mut map = HashMap::new();

        for builtin_fn in BUILTIN_FUNCTION.iter() {
            let name = builtin_fn.name.clone();
            let declaration = Declaration::from(builtin_fn);

            map.insert(name.to_owned(), vec![declaration]);
        }

        Self { map }
    }

    pub fn insert(&mut self, value: Declaration) -> bool {
        let name = value.name.clone();

        if let Some(declarations) = self.map.get_mut(&name) {
            if declarations.contains(&value) {
                return false;
            }

            declarations.push(value)
        } else {
            self.map.insert(name.to_owned(), vec![value]);
        }

        true
    }

    pub fn is_declared_at(&mut self, identifer: &IdentiferData) -> bool {
        if let Some(declarations) = self.map.get_mut(&identifer.name) {
            for decl in declarations {
                if is_declaration_at(decl, identifer.range.end) {
                    if identifer.used {
                        decl.used = true;
                    }

                    return true;
                }
            }
        }

        false
    }

    pub fn get_declared_at(&self, position: Position) -> Vec<Declaration> {
        let mut result = Vec::new();

        for declarations in self.map.values() {
            let nearest = self.get_nearest(declarations, position);

            if let Some(nearest) = nearest {
                result.push(nearest);
            }
        }

        result
    }

    pub fn get_unused(&self) -> Vec<Declaration> {
        let mut unused = Vec::new();

        for declarations in self.map.values() {
            for decl in declarations {
                if !decl.used {
                    unused.push(decl.clone());
                }
            }
        }

        unused
    }

    fn get_nearest(
        &self,
        declarations: &Vec<Declaration>,
        position: Position,
    ) -> Option<Declaration> {
        let mut nearest: Option<Declaration> = None;

        for decl in declarations {
            if is_declaration_at(decl, position) {
                if let Some(value) = &nearest {
                    if decl.range.end > value.range.end {
                        nearest = Some(decl).cloned();
                    }
                } else {
                    nearest = Some(decl).cloned();
                }
            }
        }

        nearest
    }
}

fn is_declaration_at(decl: &Declaration, position: Position) -> bool {
    let condition = match decl.kind.is_function() {
        true => position < decl.range.start || position > decl.range.end,
        false => position > decl.range.end,
    };
    let inside_scope = match decl.scope {
        Some(scope) => position > scope.start && position < scope.end,
        None => true,
    };

    condition && inside_scope
}
