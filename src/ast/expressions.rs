use std::collections::HashMap;

use crate::lexer::Token;

use super::{merge, Arenas, NameId, Span};
use cranelift_entity::{entity_impl, PrimaryMap};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExpressionId(u32);
entity_impl!(ExpressionId, "expression");

#[derive(Clone, Debug, Default)]
pub struct ExpressionData {
    expressions: PrimaryMap<ExpressionId, Expression>,
    expression_spans: HashMap<ExpressionId, Span>,
}

impl ExpressionData {
    pub fn alloc(&mut self, expression: Expression, span: Span) -> ExpressionId {
        let id = self.expressions.push(expression);
        self.expression_spans.insert(id, span);
        id
    }

    pub fn get_exp(&self, id: ExpressionId) -> &Expression {
        self.expressions.get(id).unwrap()
    }

    pub fn get_span(&self, id: ExpressionId) -> Span {
        self.expression_spans.get(&id).unwrap().clone()
    }

    pub fn expressions(&self) -> &PrimaryMap<ExpressionId, Expression> {
        &self.expressions
    }

    pub fn alloc_ident(&mut self, ident: NameId, span: Span) -> ExpressionId {
        let expr = Expression::Identifier(Identifier { ident });
        self.alloc(expr, span)
    }

    pub fn alloc_literal(&mut self, literal: Literal, span: Span) -> ExpressionId {
        self.alloc(Expression::Literal(literal), span)
    }

    pub fn alloc_call(
        &mut self,
        ident: NameId,
        args: Vec<ExpressionId>,
        span: Span,
    ) -> ExpressionId {
        let expr = Expression::Call(Call { ident, args });
        self.alloc(expr, span)
    }

    pub fn alloc_unary_op(&mut self, op: &Token, inner: ExpressionId, span: Span) -> ExpressionId {
        let expr = match op {
            Token::Invert => Expression::Invert(Invert { inner }),
            _ => todo!("More unary operator support"),
        };
        self.alloc(expr, span)
    }
}

macro_rules! gen_alloc_bin_op {
    ([$( $expr_type:ident ),*]) => {
        impl ExpressionData {
            pub fn alloc_bin_op(&mut self, op: BinaryOp, left: ExpressionId, right: ExpressionId) -> ExpressionId {
                let span = merge(&self.get_span(left), &self.get_span(right));
                let expr = match op {
                    $( BinaryOp::$expr_type => Expression::$expr_type($expr_type { left, right }), )+
                };
                self.alloc(expr, span)
            }
        }
    };
}

gen_alloc_bin_op!([
    Multiply,
    Divide,
    Modulo,
    Add,
    Subtract,
    BitShiftL,
    BitShiftR,
    ArithShiftR,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Equals,
    NotEquals,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr
]);

pub trait ContextEq<Context> {
    fn context_eq(&self, other: &Self, context: &Context) -> bool;
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expression {
    Identifier(Identifier),
    Literal(Literal),
    Call(Call),

    // Unary Expressions
    Invert(Invert),

    // Arithmetic Operations
    Multiply(Multiply),
    Divide(Divide),
    Modulo(Modulo),
    Add(Add),
    Subtract(Subtract),

    // Shifting Operations
    BitShiftL(BitShiftL),
    BitShiftR(BitShiftR),
    ArithShiftR(ArithShiftR),

    // Comparisons
    LessThan(LessThan),
    LessThanEqual(LessThanEqual),
    GreaterThan(GreaterThan),
    GreaterThanEqual(GreaterThanEqual),
    Equals(Equals),
    NotEquals(NotEquals),

    // Bitwise Operations
    BitAnd(BitAnd),
    BitXor(BitXor),
    BitOr(BitOr),

    // Logical Operations
    LogicalAnd(LogicalAnd),
    LogicalOr(LogicalOr),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinaryOp {
    // Arithmetic Operations
    Multiply,
    Divide,
    Modulo,
    Add,
    Subtract,

    // Shifting Operations
    BitShiftL,
    BitShiftR,
    ArithShiftR,

    // Comparisons
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Equals,
    NotEquals,

    // Bitwise Operations
    BitOr,
    BitXor,
    BitAnd,

    // Logical Operations
    LogicalOr,
    LogicalAnd,
}

impl ContextEq<Arenas> for ExpressionId {
    fn context_eq(&self, other: &Self, context: &Arenas) -> bool {
        if context.expr().get_span(*self) != context.expr().get_span(*other) {
            return false;
        }
        let self_expr = context.expr().get_exp(*self);
        let other_expr = context.expr().get_exp(*other);
        self_expr.context_eq(other_expr, context)
    }
}

macro_rules! gen_expression_context_eq {
    ([$( $expr_type:ident ),*]) => {
        impl ContextEq<Arenas> for Expression {
            fn context_eq(&self, other: &Self, context: &Arenas) -> bool {
                match (self, other) {
                    $((Expression::$expr_type(left), Expression::$expr_type(right)) => left.context_eq(right, context),)*
                    _ => false
                }
            }
        }
    }
}

gen_expression_context_eq!([
    Identifier,
    Literal,
    Call,
    Invert,
    Multiply,
    Divide,
    Modulo,
    Add,
    Subtract,
    BitShiftL,
    BitShiftR,
    ArithShiftR,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    Equals,
    NotEquals,
    BitAnd,
    BitXor,
    BitOr,
    LogicalAnd,
    LogicalOr
]);

#[derive(Debug, PartialEq, Clone)]
pub struct Identifier {
    pub ident: NameId,
}

impl ContextEq<Arenas> for Identifier {
    fn context_eq(&self, other: &Self, _context: &Arenas) -> bool {
        self.ident == other.ident
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    Integer(u64),
    Float(f64),
}

impl ContextEq<Arenas> for Literal {
    fn context_eq(&self, other: &Self, _context: &Arenas) -> bool {
        self == other
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Call {
    pub ident: NameId,
    pub args: Vec<ExpressionId>,
}

impl ContextEq<Arenas> for Call {
    fn context_eq(&self, other: &Self, context: &Arenas) -> bool {
        let ident_eq = self.ident.context_eq(&other.ident, context);
        let args_eq = self
            .args
            .iter()
            .zip(other.args.iter())
            .map(|(l, r)| l.context_eq(r, context))
            .all(|v| v);

        ident_eq && args_eq
    }
}

// Unary Operators

macro_rules! unary_context_eq {
    ($type_name:ident) => {
        impl ContextEq<Arenas> for $type_name {
            fn context_eq(&self, other: &Self, context: &Arenas) -> bool {
                let self_inner = context.expr().get_exp(self.inner);
                let other_inner = context.expr().get_exp(other.inner);
                self_inner.context_eq(other_inner, context)
            }
        }
    };
}

#[derive(Debug, PartialEq, Clone)]
pub struct Invert {
    pub inner: ExpressionId,
}

unary_context_eq!(Invert);

// Binary Operators

macro_rules! binary_context_eq {
    ($type_name:ident) => {
        impl ContextEq<Arenas> for $type_name {
            fn context_eq(&self, other: &Self, context: &Arenas) -> bool {
                let self_left = context.expr().get_exp(self.left);
                let other_left = context.expr().get_exp(other.left);
                let left_eq = self_left.context_eq(other_left, context);

                let self_right = context.expr().get_exp(self.right);
                let other_right = context.expr().get_exp(other.right);
                let right_eq = self_right.context_eq(other_right, context);

                left_eq && right_eq
            }
        }
    };
}

#[derive(Debug, PartialEq, Clone)]
pub struct Multiply {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(Multiply);

#[derive(Debug, PartialEq, Clone)]
pub struct Divide {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(Divide);

#[derive(Debug, PartialEq, Clone)]
pub struct Modulo {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(Modulo);

#[derive(Debug, PartialEq, Clone)]
pub struct Add {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(Add);

#[derive(Debug, PartialEq, Clone)]
pub struct Subtract {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(Subtract);

#[derive(Debug, PartialEq, Clone)]
pub struct BitShiftL {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(BitShiftL);

#[derive(Debug, PartialEq, Clone)]
pub struct BitShiftR {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(BitShiftR);

#[derive(Debug, PartialEq, Clone)]
pub struct ArithShiftR {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(ArithShiftR);

#[derive(Debug, PartialEq, Clone)]
pub struct LessThan {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(LessThan);

#[derive(Debug, PartialEq, Clone)]
pub struct LessThanEqual {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(LessThanEqual);

#[derive(Debug, PartialEq, Clone)]
pub struct GreaterThan {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(GreaterThan);

#[derive(Debug, PartialEq, Clone)]
pub struct GreaterThanEqual {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(GreaterThanEqual);

#[derive(Debug, PartialEq, Clone)]
pub struct Equals {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(Equals);

#[derive(Debug, PartialEq, Clone)]
pub struct NotEquals {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(NotEquals);

#[derive(Debug, PartialEq, Clone)]
pub struct BitAnd {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(BitAnd);

#[derive(Debug, PartialEq, Clone)]
pub struct BitXor {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(BitXor);

#[derive(Debug, PartialEq, Clone)]
pub struct BitOr {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(BitOr);

#[derive(Debug, PartialEq, Clone)]
pub struct LogicalAnd {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(LogicalAnd);

#[derive(Debug, PartialEq, Clone)]
pub struct LogicalOr {
    pub left: ExpressionId,
    pub right: ExpressionId,
}

binary_context_eq!(LogicalOr);
