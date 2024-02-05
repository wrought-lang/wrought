use crate::ast::expressions::ExpressionId;
use crate::ast::{
    self,
    expressions::{BinaryOp, ExpressionData},
    M,
};
use crate::ast::{merge, NameId};
use crate::lexer::Token;
use crate::parser::{ParseInput, ParserError};

pub fn parse_expression(
    input: &mut ParseInput,
    data: &mut ExpressionData,
) -> Result<ExpressionId, ParserError> {
    pratt_parse(input, data, 0)
}

/// Pratt parsing of expressions based on
/// https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html
fn pratt_parse(
    input: &mut ParseInput,
    data: &mut ExpressionData,
    min_bp: u8,
) -> Result<ExpressionId, ParserError> {
    let mut lhs = parse_leaf(input, data)?;
    loop {
        let checkpoint = input.checkpoint();
        if let Some(bin_op) = try_parse_bin_op(input) {
            let (l_bp, r_bp) = infix_binding_power(bin_op.value);

            if l_bp < min_bp {
                input.restore(checkpoint);
                break;
            }

            let rhs = pratt_parse(input, data, r_bp)?;
            lhs = data.alloc_bin_op(bin_op.as_ref(), lhs, rhs);
        } else {
            break;
        }
    }
    Ok(lhs)
}

fn parse_leaf(
    input: &mut ParseInput,
    data: &mut ExpressionData,
) -> Result<ExpressionId, ParserError> {
    if input.peek()?.token == Token::LParen {
        return parse_parenthetical(input, data);
    }
    if matches!(input.peekn(0), Some(Token::Identifier(_)))
        && matches!(input.peekn(1), Some(Token::LParen))
    {
        return parse_call(input, data);
    }
    if matches!(input.peek()?.token, Token::Identifier(_)) {
        return parse_ident_expr(input, data);
    }
    parse_literal(input, data)
}

fn parse_parenthetical(
    input: &mut ParseInput,
    data: &mut ExpressionData,
) -> Result<ExpressionId, ParserError> {
    let _left = input.assert_next(Token::LParen, "Left parenthesis '('")?;
    let inner = parse_expression(input, data)?;
    let _right = input.assert_next(Token::RParen, "Right parenthesis ')'")?;
    Ok(inner)
}

/// Parse an identifier
pub fn parse_ident_expr(
    input: &mut ParseInput,
    data: &mut ExpressionData,
) -> Result<ExpressionId, ParserError> {
    let checkpoint = input.checkpoint();
    let next = input.next()?;
    let span = next.span.clone();
    match next.token.clone() {
        Token::Identifier(ident) => Ok(data.alloc_ident(ident, span)),
        _ => {
            input.restore(checkpoint);
            Err(input.unexpected_token("Expected identifier"))
        }
    }
}

fn parse_literal(
    input: &mut ParseInput,
    data: &mut ExpressionData,
) -> Result<ExpressionId, ParserError> {
    let next = input.next()?;
    let span = next.span.clone();
    let literal = match &next.token {
        Token::StringLiteral(_value) => return Err(input.unsupported_error("StringLiteral")),
        Token::DecIntLiteral(value) => ast::Literal::Integer(*value),
        Token::DecFloatLiteral(value) => ast::Literal::Float(*value),
        Token::BinLiteral(value) => ast::Literal::Integer(*value),
        Token::HexLiteral(value) => ast::Literal::Integer(*value),
        _ => return Err(input.unexpected_token("Parse Literal")),
    };
    Ok(data.alloc_literal(literal, span))
}

fn parse_call(
    input: &mut ParseInput,
    data: &mut ExpressionData,
) -> Result<ExpressionId, ParserError> {
    let first = input.next()?;

    let ident = match first.token.clone() {
        Token::Identifier(ident) => ident,
        _ => return Err(input.unexpected_token("Parse expression identifier")),
    };
    let ident = M::new(ident, first.span.clone());
    let name_id = NameId::new();

    let lparen = input.assert_next(Token::LParen, "Function arguments")?;

    let mut args = Vec::new();
    let rparen = loop {
        args.push(parse_expression(input, data)?);

        let token = input.next()?;
        match token.token {
            Token::Comma => continue,
            Token::RParen => break token.span.clone(),
            _ => return Err(input.unexpected_token("Argument list")),
        }
    };

    let span = merge(&lparen, &rparen);

    Ok(data.alloc_call(ident, name_id, args, span))
}

fn try_parse_bin_op(input: &mut ParseInput) -> Option<M<BinaryOp>> {
    let next = input.peek().ok()?;
    let span = next.span.clone();
    let op = match &next.token {
        Token::LogicalOr => BinaryOp::LogicalOr,
        Token::LogicalAnd => BinaryOp::LogicalAnd,

        Token::BitOr => BinaryOp::BitOr,

        Token::BitXor => BinaryOp::BitXor,

        Token::BitAnd => BinaryOp::BitAnd,

        Token::EQ => BinaryOp::Equals,
        Token::NEQ => BinaryOp::NotEquals,

        Token::LT => BinaryOp::LessThan,
        Token::LTE => BinaryOp::LessThanEqual,
        Token::GT => BinaryOp::GreaterThan,
        Token::GTE => BinaryOp::GreaterThanEqual,

        Token::BitShiftL => BinaryOp::BitShiftL,
        Token::BitShiftR => BinaryOp::BitShiftR,
        Token::ArithShiftR => BinaryOp::ArithShiftR,

        Token::Add => BinaryOp::Add,
        Token::Sub => BinaryOp::Subtract,

        Token::Mult => BinaryOp::Multiply,
        Token::Div => BinaryOp::Divide,
        Token::Mod => BinaryOp::Modulo,

        _ => return None,
    };
    let _ = input.next();
    Some(M::new(op, span))
}

fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::LogicalOr => (10, 1),
        BinaryOp::LogicalAnd => (20, 21),

        BinaryOp::BitOr => (30, 31),

        BinaryOp::BitXor => (40, 41),

        BinaryOp::BitAnd => (50, 51),

        BinaryOp::Equals | BinaryOp::NotEquals => (60, 61),

        BinaryOp::LessThan | BinaryOp::LessThanEqual | BinaryOp::GreaterThan | BinaryOp::GreaterThanEqual => (70, 71),

        BinaryOp::BitShiftL | BinaryOp::BitShiftR | BinaryOp::ArithShiftR => (80, 81),

        BinaryOp::Add | BinaryOp::Subtract => (90, 91),

        BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => (100, 101),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::tests::{make_input, make_span};

    use crate::ast::expressions::Literal;

    #[test]
    fn parsing_supports_dec_integer() {
        let mut data = ExpressionData::default();
        let cases = [
            ("0", 0, make_span(0, 1)),
            ("1", 1, make_span(0, 1)),
            ("32", 32, make_span(0, 2)),
            ("129", 129, make_span(0, 3)),
        ];
        for (source, value, span) in cases {
            let expected_expression = data.alloc_literal(Literal::Integer(value), span.clone());

            let found_literal = parse_literal(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_literal, expected_expression));
            let found_leaf = parse_leaf(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_leaf, expected_expression));
            let found_expression = parse_expression(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_expression, expected_expression));
        }
    }

    #[test]
    fn parsing_supports_idents() {
        let mut data = ExpressionData::default();
        let cases = [
            ("foo", make_span(0, 3)),
            ("foobar", make_span(0, 6)),
            ("asdf", make_span(0, 4)),
            ("asdf2", make_span(0, 5)),
        ];
        for (source, span) in cases {
            let expected_expression = data.alloc_ident(source.to_owned(), span.clone());
            let found_ident = parse_ident_expr(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_ident, expected_expression));

            let found_leaf = parse_leaf(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_leaf, expected_expression));

            let found_expression = parse_expression(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_expression, expected_expression));
        }
    }

    #[test]
    fn parsing_supports_parenthesized_idents() {
        let mut data = ExpressionData::default();
        // parenthesized, raw, raw-span
        let cases = [
            ("(foo)", "foo", make_span(1, 3)),
            ("(foobar)", "foobar", make_span(1, 6)),
            ("(asdf)", "asdf", make_span(1, 4)),
            ("(asdf2)", "asdf2", make_span(1, 5)),
        ];
        for (source, ident, span) in cases {
            let expected_expression = data.alloc_ident(ident.to_owned(), span.clone());
            let found_expression = parse_parenthetical(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_expression, expected_expression));
            let found_expression = parse_leaf(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_expression, expected_expression));
            let found_expression = parse_expression(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(found_expression, expected_expression));
        }
    }

    #[test]
    fn parse_expression_respects_precedence() {
        let mut data = ExpressionData::default();

        macro_rules! lit {
            ($val:expr => ($span_l:expr, $span_r:expr)) => {{
                let expr = $val;
                let span = make_span($span_l, $span_r);
                data.alloc_literal(Literal::Integer(expr), span)
            }};
        }

        macro_rules! bin {
            ($left:expr, $op:expr, $right:expr) => {{
                let lhs = $left;
                let rhs = $right;
                data.alloc_bin_op(&$op, lhs, rhs)
            }};
        }

        let source0 = "0 + 1 * 2";
        let expected0 = bin!(
            lit!(0 => (0, 1)),
            BinaryOp::Add,
            bin!(
                lit!(1 => (4, 1)),
                BinaryOp::Multiply,
                lit!(2 => (8, 1))
            )
        );

        let source1 = "0 * 1 + 2";
        let expected1 = bin!(
            bin!(
                lit!(0 => (0, 1)),
                BinaryOp::Multiply,
                lit!(1 => (4, 1))
            ),
            BinaryOp::Add,
            lit!(2 => (8, 1))
        );

        let cases = [(source0, expected0), (source1, expected1)];

        for (source, expected) in cases {
            let result = parse_expression(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(result, expected));
        }
    }

    #[test]
    fn parse_expression_respects_associativity() {
        let mut data = ExpressionData::default();

        macro_rules! lit {
            ($val:expr => ($span_l:expr, $span_r:expr)) => {{
                let expr = $val;
                let span = make_span($span_l, $span_r);
                data.alloc_literal(Literal::Integer(expr), span)
            }};
        }

        macro_rules! bin {
            ($left:expr, $op:expr, $right:expr) => {{
                let lhs = $left;
                let rhs = $right;
                data.alloc_bin_op(&$op, lhs, rhs)
            }};
        }

        let source0 = "0 + 1 + 2";
        let expected0 = bin!(
            bin!(
                lit!(0 => (0, 1)),
                BinaryOp::Add,
                lit!(1 => (4, 1))
            ),
            BinaryOp::Add,
            lit!(2 => (8, 1))
        );

        let source1 = "0 * 1 * 2";
        let expected1 = bin!(
            bin!(
                lit!(0 => (0, 1)),
                BinaryOp::Multiply,
                lit!(1 => (4, 1))
            ),
            BinaryOp::Multiply,
            lit!(2 => (8, 1))
        );

        let cases = [(source0, expected0), (source1, expected1)];

        for (source, expected) in cases {
            let result = parse_expression(&mut make_input(source), &mut data).unwrap();
            assert!(data.eq(result, expected));
        }
    }
}
