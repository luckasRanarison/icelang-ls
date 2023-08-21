use std::{cmp::Ordering, collections::HashMap};

use tower_lsp::lsp_types::{Position, Range};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableType {
    Any,
    Null,
    Boolean,
    Number,
    String,
    Object,
    Function,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, Eq)]
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

impl PartialOrd for Declaration {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.start < other.start {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl Ord for Declaration {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.start < other.start {
            Ordering::Less
        } else {
            Ordering::Greater
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
            for decl in declarations {
                let is_function = decl.kind.is_function();
                let inside_scope = match decl.scope {
                    Some(scope) => position > scope.start && position < scope.end,
                    None => true,
                };

                if (position > decl.start || is_function) && inside_scope {
                    result.push(decl.clone());
                }
            }
        }

        result
    }

    // FIXME: get the nearest to avoid duplication
    // fn get_nearest(
    //     &self,
    //     declarations: &Vec<Declaration>,
    //     position: Position,
    // ) -> Option<Declaration> {
    //     declarations.iter().max().cloned()
    // }
}
