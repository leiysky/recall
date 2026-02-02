// Copyright 2026 Recall Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::Context;
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Table {
    Doc,
    Chunk,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderBy {
    Score,
    Field(FieldRef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RqlQuery {
    pub fields: Vec<SelectField>,
    pub table: Table,
    pub using_semantic: Option<String>,
    pub using_lexical: Option<String>,
    pub filter: Option<FilterExpr>,
    pub order_by: Option<(OrderBy, OrderDir)>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectField {
    All,
    Score,
    Field(FieldRef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldRef {
    pub table: Option<Table>,
    pub name: String,
}

impl FieldRef {
    pub fn parse(input: &str) -> Self {
        if let Some((prefix, name)) = input.split_once('.') {
            let table = match prefix.to_lowercase().as_str() {
                "doc" => Some(Table::Doc),
                "chunk" => Some(Table::Chunk),
                _ => None,
            };
            Self {
                table,
                name: name.to_string(),
            }
        } else {
            Self {
                table: None,
                name: input.to_string(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterExpr {
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
    Not(Box<FilterExpr>),
    Predicate(Predicate),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    Cmp {
        field: FieldRef,
        op: CmpOp,
        value: Value,
    },
    In {
        field: FieldRef,
        values: Vec<Value>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Like,
    Glob,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    String(String),
    Number(f64),
    Comma,
    LParen,
    RParen,
    Star,
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Semicolon,
    Keyword(Keyword),
}

#[derive(Debug, Clone, PartialEq)]
enum Keyword {
    Select,
    From,
    Using,
    Semantic,
    Lexical,
    Filter,
    Order,
    By,
    Limit,
    Offset,
    Asc,
    Desc,
    And,
    Or,
    Not,
    In,
    Like,
    Glob,
}

pub fn parse_rql(input: &str) -> Result<RqlQuery> {
    let tokens = lex(input)?;
    let mut p = Parser::new(tokens);
    p.parse_rql()
}

pub fn parse_filter(input: &str) -> Result<FilterExpr> {
    let tokens = lex(input)?;
    let mut p = Parser::new(tokens);
    p.parse_filter_expr()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_rql(&mut self) -> Result<RqlQuery> {
        if self.peek_keyword(Keyword::Select) {
            self.parse_rql_select_first()
        } else if self.peek_keyword(Keyword::From) {
            self.parse_rql_from_first()
        } else {
            anyhow::bail!("expected SELECT or FROM")
        }
    }

    fn parse_rql_select_first(&mut self) -> Result<RqlQuery> {
        self.expect_keyword(Keyword::Select)?;
        let fields = self.parse_select_fields()?;
        self.expect_keyword(Keyword::From)?;
        let table = self.parse_table()?;
        let (using_semantic, using_lexical) = self.parse_using_clause()?;
        let filter = self.parse_filter_clause()?;
        let order_by = self.parse_order_clause()?;
        let (limit, offset) = self.parse_limit_clause()?;
        self.consume_semicolon();
        Ok(RqlQuery {
            fields,
            table,
            using_semantic,
            using_lexical,
            filter,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_rql_from_first(&mut self) -> Result<RqlQuery> {
        self.expect_keyword(Keyword::From)?;
        let table = self.parse_table()?;
        let (using_semantic, using_lexical) = self.parse_using_clause()?;
        let filter = self.parse_filter_clause()?;
        let order_by = self.parse_order_clause()?;
        let (limit, offset) = self.parse_limit_clause()?;
        self.expect_keyword(Keyword::Select)?;
        let fields = self.parse_select_fields()?;
        self.consume_semicolon();
        Ok(RqlQuery {
            fields,
            table,
            using_semantic,
            using_lexical,
            filter,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_using_clause(&mut self) -> Result<(Option<String>, Option<String>)> {
        let mut using_semantic = None;
        let mut using_lexical = None;
        if self.peek_keyword(Keyword::Using) {
            self.next();
            loop {
                if self.peek_keyword(Keyword::Semantic) {
                    self.next();
                    self.expect(Token::LParen)?;
                    let text = self.expect_string()?;
                    self.expect(Token::RParen)?;
                    using_semantic = Some(text);
                } else if self.peek_keyword(Keyword::Lexical) {
                    self.next();
                    self.expect(Token::LParen)?;
                    let text = self.expect_string()?;
                    self.expect(Token::RParen)?;
                    using_lexical = Some(text);
                } else {
                    break;
                }
                if self.peek(Token::Comma) {
                    self.next();
                } else {
                    break;
                }
            }
        }
        Ok((using_semantic, using_lexical))
    }

    fn parse_filter_clause(&mut self) -> Result<Option<FilterExpr>> {
        if self.peek_keyword(Keyword::Filter) {
            self.next();
            Ok(Some(self.parse_filter_expr()?))
        } else {
            Ok(None)
        }
    }

    fn parse_order_clause(&mut self) -> Result<Option<(OrderBy, OrderDir)>> {
        if self.peek_keyword(Keyword::Order) {
            self.next();
            self.expect_keyword(Keyword::By)?;
            let order = if self.peek_ident("score") {
                self.next();
                OrderBy::Score
            } else {
                let field = self.expect_ident()?;
                OrderBy::Field(FieldRef::parse(&field))
            };
            let dir = if self.peek_keyword(Keyword::Asc) {
                self.next();
                OrderDir::Asc
            } else if self.peek_keyword(Keyword::Desc) {
                self.next();
                OrderDir::Desc
            } else {
                OrderDir::Desc
            };
            Ok(Some((order, dir)))
        } else {
            Ok(None)
        }
    }

    fn parse_limit_clause(&mut self) -> Result<(Option<usize>, Option<usize>)> {
        let mut limit = None;
        let mut offset = None;
        if self.peek_keyword(Keyword::Limit) {
            self.next();
            limit = Some(self.expect_number()? as usize);
            if self.peek_keyword(Keyword::Offset) {
                self.next();
                offset = Some(self.expect_number()? as usize);
            }
        }
        Ok((limit, offset))
    }

    fn consume_semicolon(&mut self) {
        if self.peek(Token::Semicolon) {
            self.next();
        }
    }

    fn parse_select_fields(&mut self) -> Result<Vec<SelectField>> {
        let mut fields = Vec::new();
        loop {
            if self.peek(Token::Star) {
                self.next();
                fields.push(SelectField::All);
            } else if self.peek_ident("score") {
                self.next();
                fields.push(SelectField::Score);
            } else {
                let ident = self.expect_ident()?;
                fields.push(SelectField::Field(FieldRef::parse(&ident)));
            }
            if self.peek(Token::Comma) {
                self.next();
            } else {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_table(&mut self) -> Result<Table> {
        let ident = self.expect_ident()?;
        match ident.to_lowercase().as_str() {
            "doc" => Ok(Table::Doc),
            "chunk" => Ok(Table::Chunk),
            _ => anyhow::bail!("unknown table {ident}"),
        }
    }

    fn parse_filter_expr(&mut self) -> Result<FilterExpr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<FilterExpr> {
        let mut left = self.parse_and()?;
        while self.peek_keyword(Keyword::Or) {
            self.next();
            let right = self.parse_and()?;
            left = FilterExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<FilterExpr> {
        let mut left = self.parse_not()?;
        while self.peek_keyword(Keyword::And) {
            self.next();
            let right = self.parse_not()?;
            left = FilterExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<FilterExpr> {
        if self.peek_keyword(Keyword::Not) {
            self.next();
            let expr = self.parse_term()?;
            Ok(FilterExpr::Not(Box::new(expr)))
        } else {
            self.parse_term()
        }
    }

    fn parse_term(&mut self) -> Result<FilterExpr> {
        if self.peek(Token::LParen) {
            self.next();
            let expr = self.parse_filter_expr()?;
            self.expect(Token::RParen)?;
            return Ok(expr);
        }
        let field = FieldRef::parse(&self.expect_ident()?);
        if self.peek_keyword(Keyword::In) {
            self.next();
            self.expect(Token::LParen)?;
            let mut values = Vec::new();
            loop {
                values.push(self.expect_value()?);
                if self.peek(Token::Comma) {
                    self.next();
                } else {
                    break;
                }
            }
            self.expect(Token::RParen)?;
            Ok(FilterExpr::Predicate(Predicate::In { field, values }))
        } else {
            let op = self.parse_cmp_op()?;
            let value = self.expect_value()?;
            Ok(FilterExpr::Predicate(Predicate::Cmp { field, op, value }))
        }
    }

    fn parse_cmp_op(&mut self) -> Result<CmpOp> {
        if self.peek(Token::Eq) {
            self.next();
            Ok(CmpOp::Eq)
        } else if self.peek(Token::Ne) {
            self.next();
            Ok(CmpOp::Ne)
        } else if self.peek(Token::Lte) {
            self.next();
            Ok(CmpOp::Lte)
        } else if self.peek(Token::Gte) {
            self.next();
            Ok(CmpOp::Gte)
        } else if self.peek(Token::Lt) {
            self.next();
            Ok(CmpOp::Lt)
        } else if self.peek(Token::Gt) {
            self.next();
            Ok(CmpOp::Gt)
        } else if self.peek_keyword(Keyword::Like) {
            self.next();
            Ok(CmpOp::Like)
        } else if self.peek_keyword(Keyword::Glob) {
            self.next();
            Ok(CmpOp::Glob)
        } else {
            anyhow::bail!("expected comparison operator")
        }
    }

    fn expect_value(&mut self) -> Result<Value> {
        if let Some(Token::String(s)) = self.peek_token() {
            self.next();
            Ok(Value::String(s))
        } else if let Some(Token::Number(n)) = self.peek_token() {
            self.next();
            Ok(Value::Number(n))
        } else {
            anyhow::bail!("expected value")
        }
    }

    fn expect_string(&mut self) -> Result<String> {
        if let Some(Token::String(s)) = self.peek_token() {
            self.next();
            Ok(s)
        } else {
            anyhow::bail!("expected string literal")
        }
    }

    fn expect_ident(&mut self) -> Result<String> {
        if let Some(Token::Ident(s)) = self.peek_token() {
            self.next();
            Ok(s)
        } else {
            anyhow::bail!("expected identifier")
        }
    }

    fn expect_number(&mut self) -> Result<f64> {
        if let Some(Token::Number(n)) = self.peek_token() {
            self.next();
            Ok(n)
        } else {
            anyhow::bail!("expected number")
        }
    }

    fn expect_keyword(&mut self, kw: Keyword) -> Result<()> {
        if self.peek_keyword(kw.clone()) {
            self.next();
            Ok(())
        } else {
            anyhow::bail!("expected keyword {:?}", kw)
        }
    }

    fn peek(&self, token: Token) -> bool {
        self.peek_token().as_ref() == Some(&token)
    }

    fn peek_keyword(&self, kw: Keyword) -> bool {
        matches!(self.peek_token(), Some(Token::Keyword(k)) if k == kw)
    }

    fn peek_ident(&self, ident: &str) -> bool {
        matches!(self.peek_token(), Some(Token::Ident(s)) if s.eq_ignore_ascii_case(ident))
    }

    fn peek_token(&self) -> Option<Token> {
        self.tokens.get(self.pos).cloned()
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos >= self.tokens.len() {
            None
        } else {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        }
    }

    fn expect(&mut self, token: Token) -> Result<()> {
        if self.peek(token.clone()) {
            self.next();
            Ok(())
        } else {
            anyhow::bail!("expected token {:?}", token)
        }
    }
}

fn lex(input: &str) -> Result<Vec<Token>> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == ',' {
            chars.next();
            tokens.push(Token::Comma);
            continue;
        }
        if ch == '(' {
            chars.next();
            tokens.push(Token::LParen);
            continue;
        }
        if ch == ')' {
            chars.next();
            tokens.push(Token::RParen);
            continue;
        }
        if ch == '*' {
            chars.next();
            tokens.push(Token::Star);
            continue;
        }
        if ch == ';' {
            chars.next();
            tokens.push(Token::Semicolon);
            continue;
        }
        if ch == '=' {
            chars.next();
            tokens.push(Token::Eq);
            continue;
        }
        if ch == '!' {
            chars.next();
            if chars.peek() == Some(&'=') {
                chars.next();
                tokens.push(Token::Ne);
                continue;
            }
            anyhow::bail!("unexpected '!'");
        }
        if ch == '<' {
            chars.next();
            if chars.peek() == Some(&'=') {
                chars.next();
                tokens.push(Token::Lte);
            } else {
                tokens.push(Token::Lt);
            }
            continue;
        }
        if ch == '>' {
            chars.next();
            if chars.peek() == Some(&'=') {
                chars.next();
                tokens.push(Token::Gte);
            } else {
                tokens.push(Token::Gt);
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            let quote = ch;
            chars.next();
            let mut buf = String::new();
            while let Some(c) = chars.next() {
                if c == quote {
                    break;
                }
                if c == '\\' {
                    if let Some(esc) = chars.next() {
                        buf.push(esc);
                    }
                } else {
                    buf.push(c);
                }
            }
            tokens.push(Token::String(buf));
            continue;
        }
        if ch.is_ascii_digit() {
            let mut buf = String::new();
            while let Some(c) = chars.peek().copied() {
                if c.is_ascii_digit() || c == '.' {
                    buf.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let num: f64 = buf.parse().context("parse number")?;
            tokens.push(Token::Number(num));
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            let mut buf = String::new();
            while let Some(c) = chars.peek().copied() {
                if c.is_alphanumeric() || c == '_' || c == '.' {
                    buf.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let kw = match buf.to_lowercase().as_str() {
                "select" => Some(Keyword::Select),
                "from" => Some(Keyword::From),
                "using" => Some(Keyword::Using),
                "semantic" => Some(Keyword::Semantic),
                "lexical" => Some(Keyword::Lexical),
                "filter" => Some(Keyword::Filter),
                "order" => Some(Keyword::Order),
                "by" => Some(Keyword::By),
                "limit" => Some(Keyword::Limit),
                "offset" => Some(Keyword::Offset),
                "asc" => Some(Keyword::Asc),
                "desc" => Some(Keyword::Desc),
                "and" => Some(Keyword::And),
                "or" => Some(Keyword::Or),
                "not" => Some(Keyword::Not),
                "in" => Some(Keyword::In),
                "like" => Some(Keyword::Like),
                "glob" => Some(Keyword::Glob),
                _ => None,
            };
            if let Some(k) = kw {
                tokens.push(Token::Keyword(k));
            } else {
                tokens.push(Token::Ident(buf));
            }
            continue;
        }
        anyhow::bail!("unexpected character {ch}");
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_rql() {
        let q = parse_rql("FROM doc FILTER doc.tag = 'x' LIMIT 2 SELECT doc.id;").unwrap();
        assert_eq!(q.table, Table::Doc);
        assert_eq!(q.limit, Some(2));
        assert!(q.filter.is_some());
    }

    #[test]
    fn parse_legacy_select_first_rql() {
        let q = parse_rql("SELECT doc.id FROM doc FILTER doc.tag = 'x' LIMIT 2;").unwrap();
        assert_eq!(q.table, Table::Doc);
        assert_eq!(q.limit, Some(2));
        assert!(q.filter.is_some());
    }

    #[test]
    fn parse_filter_expr() {
        let f = parse_filter("doc.tag = 'x' AND chunk.tokens <= 128").unwrap();
        match f {
            FilterExpr::And(_, _) => {}
            _ => panic!("expected and"),
        }
    }
}
