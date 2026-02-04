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

use anyhow::Result;
use anyhow::bail;
use rusqlite::types::Value as SqlValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlTable {
    Doc,
    Chunk,
    ChunkFts,
}

impl SqlTable {
    pub fn as_str(self) -> &'static str {
        match self {
            SqlTable::Doc => "doc",
            SqlTable::Chunk => "chunk",
            SqlTable::ChunkFts => "chunk_fts",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlColumn {
    DocId,
    DocPath,
    DocMtime,
    DocHash,
    DocTag,
    DocSource,
    DocMeta,
    DocDeleted,
    ChunkRowid,
    ChunkId,
    ChunkDocId,
    ChunkOffset,
    ChunkTokens,
    ChunkText,
    ChunkDeleted,
    ChunkFtsRowid,
}

impl SqlColumn {
    pub fn sql(self) -> &'static str {
        match self {
            SqlColumn::DocId => "doc.id",
            SqlColumn::DocPath => "doc.path",
            SqlColumn::DocMtime => "doc.mtime",
            SqlColumn::DocHash => "doc.hash",
            SqlColumn::DocTag => "doc.tag",
            SqlColumn::DocSource => "doc.source",
            SqlColumn::DocMeta => "doc.meta",
            SqlColumn::DocDeleted => "doc.deleted",
            SqlColumn::ChunkRowid => "chunk.rowid",
            SqlColumn::ChunkId => "chunk.id",
            SqlColumn::ChunkDocId => "chunk.doc_id",
            SqlColumn::ChunkOffset => "chunk.offset",
            SqlColumn::ChunkTokens => "chunk.tokens",
            SqlColumn::ChunkText => "chunk.text",
            SqlColumn::ChunkDeleted => "chunk.deleted",
            SqlColumn::ChunkFtsRowid => "chunk_fts.rowid",
        }
    }
}

#[derive(Clone, Debug)]
pub enum SqlExpr {
    Column(SqlColumn),
    JsonExtract { column: SqlColumn, key: String },
    Raw(&'static str),
    Alias(&'static str),
}

impl SqlExpr {
    pub fn column(column: SqlColumn) -> Self {
        Self::Column(column)
    }

    pub fn json_extract(column: SqlColumn, key: String) -> Self {
        Self::JsonExtract { column, key }
    }

    pub fn raw(sql: &'static str) -> Self {
        Self::Raw(sql)
    }

    pub fn alias(alias: &'static str) -> Self {
        Self::Alias(alias)
    }

    pub fn to_sql(&self) -> String {
        match self {
            SqlExpr::Column(column) => column.sql().to_string(),
            SqlExpr::JsonExtract { column, key } => {
                format!("json_extract({}, '$.{}')", column.sql(), key)
            }
            SqlExpr::Raw(sql) => (*sql).to_string(),
            SqlExpr::Alias(alias) => (*alias).to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SqlSelectItem {
    expr: SqlExpr,
    alias: Option<&'static str>,
}

impl SqlSelectItem {
    pub fn new(expr: SqlExpr) -> Self {
        Self { expr, alias: None }
    }

    pub fn alias(mut self, alias: &'static str) -> Self {
        self.alias = Some(alias);
        self
    }

    fn to_sql(&self) -> String {
        let expr = self.expr.to_sql();
        if let Some(alias) = self.alias {
            format!("{} AS {}", expr, alias)
        } else {
            expr
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlOrderDir {
    Asc,
    Desc,
}

impl SqlOrderDir {
    fn as_str(self) -> &'static str {
        match self {
            SqlOrderDir::Asc => "ASC",
            SqlOrderDir::Desc => "DESC",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SqlOrderBy {
    expr: SqlExpr,
    dir: SqlOrderDir,
}

impl SqlOrderBy {
    pub fn new(expr: SqlExpr, dir: SqlOrderDir) -> Self {
        Self { expr, dir }
    }

    pub fn asc(expr: SqlExpr) -> Self {
        Self::new(expr, SqlOrderDir::Asc)
    }

    pub fn desc(expr: SqlExpr) -> Self {
        Self::new(expr, SqlOrderDir::Desc)
    }

    fn to_sql(&self) -> String {
        format!("{} {}", self.expr.to_sql(), self.dir.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct SqlFragment {
    pub sql: String,
    pub params: Vec<SqlValue>,
}

impl SqlFragment {
    pub fn raw(sql: impl Into<String>) -> Self {
        Self {
            sql: sql.into(),
            params: Vec::new(),
        }
    }

    pub fn raw_with_params(sql: impl Into<String>, params: Vec<SqlValue>) -> Self {
        Self {
            sql: sql.into(),
            params,
        }
    }

    pub fn cmp(expr: SqlExpr, op: &str, value: SqlValue) -> Self {
        let sql = format!("{} {} ?", expr.to_sql(), op);
        Self {
            sql,
            params: vec![value],
        }
    }

    pub fn in_list(expr: SqlExpr, values: Vec<SqlValue>) -> Result<Self> {
        if values.is_empty() {
            bail!("IN list cannot be empty");
        }
        let placeholders = vec!["?"; values.len()].join(", ");
        let sql = format!("{} IN ({})", expr.to_sql(), placeholders);
        Ok(Self {
            sql,
            params: values,
        })
    }

    pub fn and(self, other: SqlFragment) -> SqlFragment {
        let sql = format!("({}) AND ({})", self.sql, other.sql);
        let mut params = self.params;
        params.extend(other.params);
        SqlFragment { sql, params }
    }

    pub fn or(self, other: SqlFragment) -> SqlFragment {
        let sql = format!("({}) OR ({})", self.sql, other.sql);
        let mut params = self.params;
        params.extend(other.params);
        SqlFragment { sql, params }
    }

    pub fn not(self) -> SqlFragment {
        let sql = format!("NOT ({})", self.sql);
        SqlFragment {
            sql,
            params: self.params,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SqlJoinKind {
    Inner,
}

impl SqlJoinKind {
    fn as_str(self) -> &'static str {
        match self {
            SqlJoinKind::Inner => "INNER",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SqlJoin {
    kind: SqlJoinKind,
    table: SqlTable,
    left: SqlColumn,
    right: SqlColumn,
}

impl SqlJoin {
    pub fn inner(table: SqlTable, left: SqlColumn, right: SqlColumn) -> Self {
        Self {
            kind: SqlJoinKind::Inner,
            table,
            left,
            right,
        }
    }

    fn to_sql(&self) -> String {
        format!(
            "{} JOIN {} ON {} = {}",
            self.kind.as_str(),
            self.table.as_str(),
            self.left.sql(),
            self.right.sql()
        )
    }
}

#[derive(Clone, Debug)]
pub struct SqlSelectBuilder {
    select: Vec<SqlSelectItem>,
    from: SqlTable,
    joins: Vec<SqlJoin>,
    where_clause: Option<SqlFragment>,
    order_by: Vec<SqlOrderBy>,
    limit: Option<usize>,
    offset: Option<usize>,
}

impl SqlSelectBuilder {
    pub fn new(from: SqlTable) -> Self {
        Self {
            select: Vec::new(),
            from,
            joins: Vec::new(),
            where_clause: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    pub fn select<I>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = SqlSelectItem>,
    {
        self.select.extend(items);
        self
    }

    pub fn join(mut self, join: SqlJoin) -> Self {
        self.joins.push(join);
        self
    }

    pub fn where_clause(mut self, clause: SqlFragment) -> Self {
        self.where_clause = Some(clause);
        self
    }

    pub fn order_by(mut self, order: SqlOrderBy) -> Self {
        self.order_by.push(order);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn build(self) -> (String, Vec<SqlValue>) {
        let mut sql = String::new();
        sql.push_str("SELECT ");
        if self.select.is_empty() {
            sql.push('*');
        } else {
            let mut first = true;
            for item in &self.select {
                if !first {
                    sql.push_str(", ");
                }
                first = false;
                sql.push_str(&item.to_sql());
            }
        }
        sql.push_str(" FROM ");
        sql.push_str(self.from.as_str());
        for join in &self.joins {
            sql.push(' ');
            sql.push_str(&join.to_sql());
        }

        let mut params = Vec::new();
        if let Some(where_clause) = self.where_clause {
            sql.push_str(" WHERE ");
            sql.push_str(&where_clause.sql);
            params.extend(where_clause.params);
        }
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let mut first = true;
            for order in &self.order_by {
                if !first {
                    sql.push_str(", ");
                }
                first = false;
                sql.push_str(&order.to_sql());
            }
        }
        if let Some(limit) = self.limit {
            sql.push_str(" LIMIT ?");
            params.push(SqlValue::from(limit as i64));
        }
        if let Some(offset) = self.offset {
            sql.push_str(" OFFSET ?");
            params.push(SqlValue::from(offset as i64));
        }

        (sql, params)
    }
}
