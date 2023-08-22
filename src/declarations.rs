use std::{collections::HashMap, vec};

use tower_lsp::lsp_types::{Position, Range};

// TODO: type check
#[derive(Debug, Clone)]
pub enum VariableType {
    Any,
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object(Vec<String>),
    Function(Vec<String>),
}

#[derive(Debug, Clone)]
pub enum DeclarationKind {
    Variable(VariableType),
    Function(Vec<String>),
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
    pub kind: DeclarationKind,
    pub range: Range,
    pub scope: Option<Range>,
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
        scope: Option<Range>,
    ) -> Declaration {
        Self {
            name,
            kind,
            range,
            scope,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclarationMap {
    map: HashMap<String, Vec<Declaration>>,
}

impl DeclarationMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: Declaration) -> bool {
        let name = value.name.clone();

        if let Some(declarations) = self.map.get_mut(&name) {
            let contains = declarations.contains(&value);

            if contains {
                return false;
            } else {
                declarations.push(value)
            }
        } else {
            self.map.insert(name.to_owned(), vec![value]);
        }

        true
    }

    pub fn is_declared_at(&mut self, name: &str, position: Position) -> bool {
        if let Some(declarations) = self.map.get(name) {
            for decl in declarations {
                if is_declaration_at(decl, position) {
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
