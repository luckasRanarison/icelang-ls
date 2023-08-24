use tree_sitter::Node;

#[derive(PartialEq, Eq)]
pub enum NodeType {
    StmtBlock,
    StmtVarDecl,
    StmtFuncDecl,
    StmtLoop,
    StmtWhile,
    StmtFor,
    StmtContinue,
    StmtBreak,
    StmtReturn,
    StmtExpression,

    ExprLiteral,
    ExprGroup,
    ExprIdentifier,
    ExprArray,
    ExprObject,
    ExprUnary,
    ExprBinary,
    ExprIndex,
    ExprField,
    ExprIf,
    ExprMatch,
    ExprCall,
    ExprLambda,

    Args,
    Prop,
    Iterator,

    Error,
    Unnamed,
}

impl From<&Node<'_>> for NodeType {
    fn from(value: &Node) -> Self {
        match value.kind() {
            "stmt_block" => NodeType::StmtBlock,
            "stmt_var_decl" => NodeType::StmtVarDecl,
            "stmt_func_decl" => NodeType::StmtFuncDecl,
            "stmt_loop" => NodeType::StmtLoop,
            "stmt_while" => NodeType::StmtWhile,
            "stmt_for" => NodeType::StmtFor,
            "stmt_continue" => NodeType::StmtContinue,
            "stmt_break" => NodeType::StmtBreak,
            "stmt_return" => NodeType::StmtReturn,
            "stmt_expression" => NodeType::StmtReturn,

            "expr_literal" => NodeType::ExprLiteral,
            "expr_group" => NodeType::ExprGroup,
            "expr_identifier" => NodeType::ExprIdentifier,
            "expr_array" => NodeType::ExprArray,
            "expr_object" => NodeType::ExprObject,
            "expr_unary" => NodeType::ExprUnary,
            "expr_binary" => NodeType::ExprBinary,
            "expr_index" => NodeType::ExprIndex,
            "expr_field" => NodeType::ExprField,
            "expr_if" => NodeType::ExprIf,
            "expr_match" => NodeType::ExprMatch,
            "expr_call" => NodeType::ExprCall,
            "expr_lambda" => NodeType::ExprLambda,

            "args" => NodeType::Args,
            "prop" => NodeType::Prop,
            "iterator" => NodeType::Iterator,

            "ERROR" => NodeType::Error,
            _ => NodeType::Unnamed,
        }
    }
}

pub const LOOP_NODE: [NodeType; 3] = [NodeType::StmtFor, NodeType::StmtWhile, NodeType::StmtLoop];
pub const FUNCTION_NODE: [NodeType; 2] = [NodeType::StmtFuncDecl, NodeType::ExprLambda];
