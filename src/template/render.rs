//! Condition evaluation + minijinja rendering + static variable extraction.
//! OWNER: engine-dev.
//!
//! Two strictly separated languages (spec §7.2/§7.5):
//!  * templates: `{{ var }}`, `{% if %}` / `{% elif %}` / `{% else %}`,
//!    `{% for p in platforms %}` — pure variable output and branching only;
//!  * manifest conditions (`when`, `[conditions]`): a tiny expression language
//!    with no engine behind it (no Rhai, spec §7.5).
use crate::error::{Error, Result, EXIT_CONFIG};
use minijinja::{AutoEscape, Environment, UndefinedBehavior};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub type Ctx = Map<String, Value>;

/// Reserved jinja keywords — never variables.
const KEYWORDS: [&str; 40] = [
    "if", "elif", "elseif", "else", "endif", "for", "endfor", "in", "set", "endset", "with",
    "endwith", "raw", "endraw", "not", "and", "or", "is", "defined", "true", "false", "none",
    "loop", "block", "endblock", "macro", "endmacro", "filter", "endfilter", "do", "include",
    "import", "extends", "call", "endcall", "break", "continue", "recursive", "self", "num",
];

/// Built-in predicates available to templates and conditions (§7.2, Lead-ruled).
const BUILTIN_FUNCS: [&str; 4] = ["is_server_platform", "len", "any_of", "all_of"];

fn config_error(code: &'static str, msg: impl Into<String>) -> Error {
    Error::new(code, EXIT_CONFIG, msg)
}

/// `mc_version` -> `mcVersion` (single mapping function, no aliases, §7.8 note).
pub fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
            continue;
        }
        if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Context lookup: exact key first (switches are snake_case), then camelCase.
pub fn lookup<'a>(ctx: &'a Ctx, name: &str) -> Option<&'a Value> {
    if let Some(v) = ctx.get(name) {
        return Some(v);
    }
    let camel = camel(name);
    if camel != name {
        if let Some(v) = ctx.get(&camel) {
            return Some(v);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Condition language (§7.5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    /// Raw numeric text: `1.16.5` is a *version*, not a float.
    Num(String),
    LParen,
    RParen,
    Comma,
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
    Not,
    And,
    Or,
    Contains,
    NotContains,
}

fn lex(expr: &str) -> Result<Vec<Tok>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            ',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            '=' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::Eq);
                    i += 2;
                } else {
                    return Err(config_error(
                        "template.manifest_invalid",
                        format!("条件表达式 `{expr}`: 只支持 `==`，不支持 `=`"),
                    ));
                }
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::Ne);
                    i += 2;
                } else {
                    out.push(Tok::Not);
                    i += 1;
                }
            }
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::Ge);
                    i += 2;
                } else {
                    out.push(Tok::Gt);
                    i += 1;
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Tok::Le);
                    i += 2;
                } else {
                    out.push(Tok::Lt);
                    i += 1;
                }
            }
            '&' => {
                if chars.get(i + 1) == Some(&'&') {
                    out.push(Tok::And);
                    i += 2;
                } else {
                    return Err(config_error(
                        "template.manifest_invalid",
                        format!("条件表达式 `{expr}`: 单个 `&` 非法，用 `and`/`&&`"),
                    ));
                }
            }
            '|' => {
                if chars.get(i + 1) == Some(&'|') {
                    out.push(Tok::Or);
                    i += 2;
                } else {
                    return Err(config_error(
                        "template.manifest_invalid",
                        format!("条件表达式 `{expr}`: 单个 `|` 非法，用 `or`/`||`"),
                    ));
                }
            }
            '\'' | '"' => {
                let quote = c;
                i += 1;
                let mut s = String::new();
                while i < chars.len() && chars[i] != quote {
                    s.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(config_error(
                        "template.manifest_invalid",
                        format!("条件表达式 `{expr}`: 字符串字面量没有闭合"),
                    ));
                }
                i += 1;
                out.push(Tok::Str(s));
            }
            _ if c.is_ascii_digit() || (c == '-' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit())) => {
                let start = i;
                if c == '-' {
                    i += 1;
                }
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let raw: String = chars[start..i].iter().collect();
                if raw.chars().all(|c| c == '.') {
                    return Err(config_error(
                        "template.manifest_invalid",
                        format!("条件表达式 `{expr}`: 非法数字 `{raw}`"),
                    ));
                }
                out.push(Tok::Num(raw));
            }
            _ if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric()
                        || chars[i] == '_'
                        || chars[i] == '-'
                        || chars[i] == '.')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                out.push(match word.as_str() {
                    "and" => Tok::And,
                    "or" => Tok::Or,
                    "not" => Tok::Not,
                    "contains" => Tok::Contains,
                    "not_contains" => Tok::NotContains,
                    _ => Tok::Ident(word),
                });
            }
            other => {
                return Err(config_error(
                    "template.manifest_invalid",
                    format!("条件表达式 `{expr}`: 非法字符 `{other}`"),
                ));
            }
        }
    }
    Ok(out)
}

/// Evaluate a §7.5 condition against the render context.
pub fn eval_condition(expr: &str, ctx: &Ctx) -> Result<bool> {
    let toks = lex(expr)?;
    let mut p = Parser { expr, toks: &toks, pos: 0, ctx };
    let v = p.parse_or()?;
    if p.pos != p.toks.len() {
        return Err(config_error(
            "template.manifest_invalid",
            format!("条件表达式 `{expr}`: 末尾有多余记号"),
        ));
    }
    Ok(v)
}

/// Built-in predicate (also exposed as a template function).
pub fn is_server_platform(platform: &str) -> bool {
    !crate::template::vars::is_proxy(platform)
}

struct Parser<'a> {
    expr: &'a str,
    toks: &'a [Tok],
    pos: usize,
    ctx: &'a Ctx,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, t: &Tok, what: &str) -> Result<()> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(config_error(
                "template.manifest_invalid",
                format!("条件表达式 `{}`: 期望 {what}", self.expr),
            ))
        }
    }

    fn parse_or(&mut self) -> Result<bool> {
        let mut v = self.parse_and()?;
        while self.eat(&Tok::Or) {
            let r = self.parse_and()?;
            v = v || r;
        }
        Ok(v)
    }

    fn parse_and(&mut self) -> Result<bool> {
        let mut v = self.parse_not()?;
        while self.eat(&Tok::And) {
            let r = self.parse_not()?;
            v = v && r;
        }
        Ok(v)
    }

    fn parse_not(&mut self) -> Result<bool> {
        if self.eat(&Tok::Not) {
            return Ok(!self.parse_not()?);
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<bool> {
        if self.eat(&Tok::LParen) {
            let v = self.parse_or()?;
            self.expect(&Tok::RParen, "`)`")?;
            return Ok(v);
        }
        if let Some(Tok::Ident(name)) = self.peek().cloned() {
            // Built-in predicates / combinators are function-call shaped.
            if matches!(name.as_str(), "any_of" | "all_of" | "is_server_platform") {
                if self.toks.get(self.pos + 1) == Some(&Tok::LParen) {
                    return self.parse_builtin(&name);
                }
            }
            if name == "true" || name == "false" {
                self.pos += 1;
                return Ok(name == "true");
            }
        }
        self.parse_comparison()
    }

    fn parse_builtin(&mut self, name: &str) -> Result<bool> {
        self.pos += 1; // name
        self.expect(&Tok::LParen, "`(`")?;
        match name {
            "any_of" | "all_of" => {
                let mut any = false;
                let mut all = true;
                loop {
                    let v = self.parse_or()?;
                    any |= v;
                    all &= v;
                    if self.eat(&Tok::Comma) {
                        continue;
                    }
                    break;
                }
                self.expect(&Tok::RParen, "`)`")?;
                Ok(if name == "any_of" { any } else { all })
            }
            "is_server_platform" => {
                let arg = match self.peek().cloned() {
                    Some(Tok::Str(s)) => {
                        self.pos += 1;
                        s
                    }
                    Some(Tok::Ident(id)) => {
                        // Allow a variable holding a platform id.
                        self.pos += 1;
                        match lookup(self.ctx, &id).map(as_string) {
                            Some(s) => s,
                            None => {
                                return Err(undefined(self.expr, &id));
                            }
                        }
                    }
                    _ => {
                        return Err(config_error(
                            "template.manifest_invalid",
                            format!(
                                "条件表达式 `{}`: is_server_platform(...) 需要一个字符串或变量",
                                self.expr
                            ),
                        ));
                    }
                };
                self.expect(&Tok::RParen, "`)`")?;
                Ok(is_server_platform(&arg))
            }
            _ => Err(config_error(
                "template.manifest_invalid",
                format!("条件表达式 `{}`: `{name}` 不能单独作谓词", self.expr),
            )),
        }
    }

    fn parse_comparison(&mut self) -> Result<bool> {
        let left = self.parse_operand()?;
        let op = match self.peek().cloned() {
            Some(Tok::Eq) => Tok::Eq,
            Some(Tok::Ne) => Tok::Ne,
            Some(Tok::Ge) => Tok::Ge,
            Some(Tok::Le) => Tok::Le,
            Some(Tok::Gt) => Tok::Gt,
            Some(Tok::Lt) => Tok::Lt,
            Some(Tok::Contains) => Tok::Contains,
            Some(Tok::NotContains) => Tok::NotContains,
            _ => {
                // Bare operand: must be a boolean.
                return match &left {
                    Val::Bool(b) => Ok(*b),
                    other => Err(config_error(
                        "template.manifest_invalid",
                        format!(
                            "条件表达式 `{}`: `{}` 不是布尔开关，必须写成比较（== / >= / contains ...）",
                            self.expr,
                            other.describe()
                        ),
                    )),
                };
            }
        };
        self.pos += 1;
        let right = self.parse_operand()?;
        match op {
            Tok::Contains | Tok::NotContains => {
                let needle = right.as_string().ok_or_else(|| {
                    config_error(
                        "template.manifest_invalid",
                        format!("条件表达式 `{}`: contains 右侧需要值", self.expr),
                    )
                })?;
                let hit = match &left {
                    Val::List(items) => items.iter().any(|i| i == &needle),
                    Val::Str(s) => s.contains(&needle),
                    Val::Num(n) => n.to_string() == needle,
                    Val::Bool(b) => b.to_string() == needle,
                };
                Ok(if op == Tok::Contains { hit } else { !hit })
            }
            Tok::Eq => Ok(left.equals(&right)),
            Tok::Ne => Ok(!left.equals(&right)),
            _ => {
                if let (Some(a), Some(b)) = (left.as_number(), right.as_number()) {
                    return Ok(match op {
                        Tok::Ge => a >= b,
                        Tok::Le => a <= b,
                        Tok::Gt => a > b,
                        Tok::Lt => a < b,
                        _ => unreachable!(),
                    });
                }
                // Version-like strings ("1.21.11", "26.2") compare segment-wise.
                if let (Some(a), Some(b)) = (left.as_version(), right.as_version()) {
                    let ord = cmp_version(&a, &b);
                    return Ok(match op {
                        Tok::Ge => ord != std::cmp::Ordering::Less,
                        Tok::Le => ord != std::cmp::Ordering::Greater,
                        Tok::Gt => ord == std::cmp::Ordering::Greater,
                        Tok::Lt => ord == std::cmp::Ordering::Less,
                        _ => unreachable!(),
                    });
                }
                Err(config_error(
                    "template.manifest_invalid",
                    format!(
                        "条件表达式 `{}`: `{}` 与右侧无法做数值比较",
                        self.expr,
                        left.describe()
                    ),
                ))
            }
        }
    }

    fn parse_operand(&mut self) -> Result<Val> {
        match self.peek().cloned() {
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Val::Str(s))
            }
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(Val::Num(n))
            }
            Some(Tok::Ident(id)) => {
                // `len(authors)` is the only counted form in §7.5.
                if id == "len" && self.toks.get(self.pos + 1) == Some(&Tok::LParen) {
                    self.pos += 2;
                    let inner = match self.peek().cloned() {
                        Some(Tok::Ident(name)) => {
                            self.pos += 1;
                            name
                        }
                        _ => {
                            return Err(config_error(
                                "template.manifest_invalid",
                                format!("条件表达式 `{}`: len(...) 需要一个变量名", self.expr),
                            ));
                        }
                    };
                    self.expect(&Tok::RParen, "`)`")?;
                    let value = lookup(self.ctx, &inner).ok_or_else(|| undefined(self.expr, &inner))?;
                    let n = match value {
                        Value::Array(a) => a.len() as f64,
                        Value::String(s) => s.chars().count() as f64,
                        Value::Object(o) => o.len() as f64,
                        Value::Null => 0.0,
                        other => other.as_f64().unwrap_or(0.0),
                    };
                    return Ok(Val::Num(n.to_string()));
                }
                self.pos += 1;
                match id.as_str() {
                    "true" => return Ok(Val::Bool(true)),
                    "false" => return Ok(Val::Bool(false)),
                    _ => {}
                }
                let value = lookup(self.ctx, &id).ok_or_else(|| undefined(self.expr, &id))?;
                Ok(match value {
                    Value::Bool(b) => Val::Bool(*b),
                    Value::Number(n) => Val::Num(n.to_string()),
                    Value::String(s) => Val::Str(s.clone()),
                    Value::Array(a) => Val::List(
                        a.iter()
                            .map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string()))
                            .collect(),
                    ),
                    Value::Null => Val::Str(String::new()),
                    other => Val::Str(other.to_string()),
                })
            }
            other => Err(config_error(
                "template.manifest_invalid",
                format!("条件表达式 `{}`: 期望变量或字面量，得到 {other:?}", self.expr),
            )),
        }
    }
}

fn undefined(expr: &str, name: &str) -> Error {
    config_error(
        "template.undefined_variable",
        format!("条件表达式 `{expr}`: 变量 `{name}` 未定义"),
    )
    .with_hint("检查拼写，或在清单 [conditions] / vars 中声明它")
}

#[derive(Debug, Clone)]
enum Val {
    Bool(bool),
    Num(String),
    Str(String),
    List(Vec<String>),
}

/// Segment-wise version compare (`1.21.11` > `1.16.5`, `26.2` > `1.21.11`).
fn version_segments(s: &str) -> Option<Vec<u64>> {
    let clean = s.trim();
    if clean.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for seg in clean.split('.') {
        let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.is_empty() {
            return None;
        }
        parts.push(digits.parse::<u64>().ok()?);
    }
    Some(parts)
}

fn cmp_version(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

impl Val {
    fn describe(&self) -> String {
        match self {
            Val::Bool(b) => b.to_string(),
            Val::Num(n) => n.clone(),
            Val::Str(s) => s.clone(),
            Val::List(l) => l.join(","),
        }
    }
    fn as_string(&self) -> Option<String> {
        match self {
            Val::Str(s) => Some(s.clone()),
            Val::Num(n) => Some(n.clone()),
            Val::Bool(b) => Some(b.to_string()),
            Val::List(_) => None,
        }
    }
    fn as_number(&self) -> Option<f64> {
        match self {
            Val::Num(n) => n.parse().ok(),
            Val::Str(s) => s.parse().ok(),
            Val::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Val::List(_) => None,
        }
    }
    fn as_version(&self) -> Option<Vec<u64>> {
        match self {
            Val::Num(n) => version_segments(n),
            Val::Str(s) => version_segments(s),
            _ => None,
        }
    }
    fn equals(&self, other: &Val) -> bool {
        match (self, other) {
            (Val::List(a), Val::List(b)) => a == b,
            _ => {
                let (a, b) = (self.as_string(), other.as_string());
                a.is_some() && a == b
            }
        }
    }
}

fn as_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// minijinja rendering (§7.2)
// ---------------------------------------------------------------------------

/// A configured minijinja environment. One per `build_plan`, so identical
/// inputs produce identical output.
pub struct Renderer {
    env: Environment<'static>,
}

impl std::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Renderer")
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    pub fn new() -> Self {
        let mut env = Environment::new();
        // Undefined variables are hard errors, never empty strings (§7.2).
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        // Text concatenation, no HTML semantics.
        env.set_auto_escape_callback(|_| AutoEscape::None);
        // minijinja strips the trailing newline by default; generated files must
        // keep it (checkstyle `NewlineAtEndOfFile` would fail otherwise).
        env.set_keep_trailing_newline(true);
        env.add_function("is_server_platform", |p: String| is_server_platform(&p));
        env.add_function("len", |v: minijinja::value::Value| v.len().unwrap_or(0));
        Self { env }
    }

    /// Render `src` (named `name` for error messages) with `ctx`.
    pub fn render(&self, name: &str, src: &str, ctx: &Ctx) -> Result<String> {
        self.env
            .render_named_str(name, src, Value::Object(ctx.clone()))
            .map_err(|e| map_render_error(name, e))
    }
}

fn map_render_error(name: &str, e: minijinja::Error) -> Error {
    use minijinja::ErrorKind;
    let location = match (e.line(), e.name()) {
        (Some(line), _) => format!("{name}:{line}"),
        (None, Some(n)) => n.to_string(),
        _ => name.to_string(),
    };
    let detail = e.detail().unwrap_or("").to_string();
    let code = match e.kind() {
        ErrorKind::UndefinedError => "template.undefined_variable",
        _ => "render.syntax_error",
    };
    let message = if detail.is_empty() {
        format!("模板渲染失败 {location}: {}", e.kind())
    } else {
        format!("模板渲染失败 {location}: {detail}")
    };
    let err = Error::new(code, crate::error::EXIT_CONFIG, message);
    if code == "template.undefined_variable" {
        err.with_hint("未定义变量是硬错误：在清单 vars/[conditions] 或上下文中声明它")
    } else {
        err
    }
}

// ---------------------------------------------------------------------------
// Static extraction (assertion A4 / A1 raw-block exemption)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Extracted {
    /// Raw variables actually consumed (canonical camelCase).
    pub raw: BTreeSet<String>,
    /// Switch names (`cap_*` / `has_*` / `is_*`) actually consumed.
    pub switches: BTreeSet<String>,
}

/// Extract variable names used by a template body.
///
/// Approximation rules (documented, deterministic):
///  * only inside `{{ ... }}` / `{% ... %}`;
///  * `{% raw %}` blocks are skipped entirely;
///  * `{% for X in ... %}` and `{% set X = ... %}` introduce locals;
///  * dotted chains count only their root (`mainClass.paper` -> `mainClass`);
///  * `map[expr]` counts the root and, when the key is an identifier, that
///    identifier too (it is normally a loop local);
///  * built-in functions are not variables.
pub fn extract_vars(src: &str, extra_locals: &[String]) -> Extracted {
    let mut out = Extracted::default();
    let mut locals: BTreeSet<String> = extra_locals.iter().cloned().collect();
    let body = strip_raw_blocks(src);
    for chunk in jinja_chunks(&body) {
        let toks = ident_stream(&chunk);
        // First pass: collect `for`/`set` locals so their uses are ignored.
        for (i, t) in toks.iter().enumerate() {
            match t.as_str() {
                "for" => {
                    if let Some(name) = toks.get(i + 1) {
                        if name != "in" {
                            locals.insert(name.clone());
                        }
                    }
                }
                "set" => {
                    if let Some(name) = toks.get(i + 1) {
                        locals.insert(name.clone());
                    }
                }
                _ => {}
            }
        }
        for (i, t) in toks.iter().enumerate() {
            if KEYWORDS.contains(&t.as_str()) || BUILTIN_FUNCS.contains(&t.as_str()) {
                continue;
            }
            // `for`/`set` target: consumed already as a local.
            let is_assign_target = matches!(toks.get(i.wrapping_sub(1)).map(String::as_str), Some("for") | Some("set"));
            if is_assign_target && locals.contains(t) {
                continue;
            }
            if locals.contains(t) {
                continue;
            }
            if t.starts_with("cap_") || t.starts_with("has_") || t.starts_with("is_") {
                out.switches.insert(t.clone());
            } else {
                out.raw.insert(camel(t));
            }
        }
    }
    out
}

/// Identifier names used inside a §7.5 condition expression (`when`,
/// `[conditions]`). Keywords and built-in predicates are filtered out.
pub fn expr_identifiers(expr: &str) -> BTreeSet<String> {
    ident_stream(expr)
        .into_iter()
        .filter(|t| !KEYWORDS.contains(&t.as_str()) && !BUILTIN_FUNCS.contains(&t.as_str()))
        .collect()
}

/// Contents of every `{% raw %}...{% endraw %}` block (assertion A1 exempts
/// placeholders that a template deliberately prints verbatim, §7.2).
pub fn raw_literals(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(start) = rest.find("{% raw %}") {
        let after = &rest[start + "{% raw %}".len()..];
        match after.find("{% endraw %}") {
            Some(end) => {
                out.push(after[..end].to_string());
                rest = &after[end + "{% endraw %}".len()..];
            }
            None => break,
        }
    }
    out
}

fn strip_raw_blocks(src: &str) -> String {
    let mut out = String::new();
    let mut rest = src;
    while let Some(start) = rest.find("{% raw %}") {
        out.push_str(&rest[..start]);
        let after = &rest[start + "{% raw %}".len()..];
        match after.find("{% endraw %}") {
            Some(end) => rest = &after[end + "{% endraw %}".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Iterate over the contents of `{{ ... }}` and `{% ... %}` chunks.
fn jinja_chunks(src: &str) -> Vec<String> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let (open, close) = match (bytes[i], bytes.get(i + 1)) {
            (b'{', Some(b'{')) => ("{{", "}}"),
            (b'{', Some(b'%')) => ("{%", "%}"),
            _ => {
                i += 1;
                continue;
            }
        };
        let body_start = i + open.len();
        match src[body_start..].find(close) {
            Some(off) => {
                out.push(src[body_start..body_start + off].to_string());
                i = body_start + off + close.len();
            }
            None => break,
        }
    }
    out
}

/// Identifier stream, handling dotted chains (root only) and subscripts.
fn ident_stream(chunk: &str) -> Vec<String> {
    let chars: Vec<char> = chunk.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let mut root = word;
            // Dotted chain: keep the root only.
            while chars.get(i) == Some(&'.') {
                i += 1;
                while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                // `is defined` / `is not none` style tests keep working because
                // the trailing word is just dropped.
                if root.is_empty() {
                    root = "_".into();
                }
            }
            out.push(root);
            continue;
        }
        if c.is_ascii_digit() {
            i += 1;
            continue;
        }
        i += 1;
    }
    out
}
