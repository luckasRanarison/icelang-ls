use std::collections::HashMap;

use tower_lsp::lsp_types::{Position, Range};

// TODO: type check
#[derive(Debug, Clone)]
pub enum VariableType {
    Any,
    Null,
    Boolean,
    Number,
    String,
    Object,
    Function,
}

#[derive(Debug, Clone)]
pub enum DeclarationKind {
    Variable(Option<VariableType>),
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
    pub start: Position,
    pub scope: Option<Range>,
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.scope == other.scope
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

    pub fn is_declared(&mut self, name: &String, range: &Range) -> bool {
        if let Some(declarations) = self.map.get(name) {
            for decl in declarations {
                let is_function = decl.kind.is_function();
                let inside_scope = match decl.scope {
                    Some(scope) => range.end > scope.start && range.end < scope.end,
                    None => true,
                };

                if (range.end > decl.start || is_function) && inside_scope {
                    return true;
                } else {
                    return false;
                }
            }
        }

        false
    }

    pub fn get_declared_at(&self, position: Position) -> Vec<Declaration> {
        let mut result = Vec::new();

        for (_, declarations) in &self.map {
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
            let is_function = decl.kind.is_function();
            let inside_scope = match decl.scope {
                Some(scope) => position > scope.start && position < scope.end,
                None => true,
            };

            if (position > decl.start || is_function) && inside_scope {
                if let Some(value) = &nearest {
                    if decl.start > value.start {
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
