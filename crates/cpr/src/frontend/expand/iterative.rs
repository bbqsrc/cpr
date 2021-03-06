#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_mut)]

use super::{ExpandError, HS, THS};
use crate::frontend::{
    grammar::{Define, MacroParams, Token, TokenSeq},
    Context, SymbolState,
};
use std::collections::{HashSet, VecDeque};

pub trait Expandable2 {
    fn as_ths<'a>(&'a self) -> Box<dyn Iterator<Item = THS> + 'a>;
}

impl Expandable2 for TokenSeq {
    fn as_ths<'a>(&'a self) -> Box<dyn Iterator<Item = THS> + 'a> {
        Box::new(
            self.0
                .iter()
                .map(|tok| THS(tok.clone(), Default::default())),
        )
    }
}

/// Main expand routine, calls `subst`
pub fn expand<'a>(
    mut is: Box<dyn Iterator<Item = THS> + 'a>,
    os: &'a mut Vec<THS>,
    ctx: &'a Context,
    depth: usize,
) -> Result<(), ExpandError> {
    let mut cycle = 0;

    'expand_all: loop {
        cycle += 1;
        log::trace!(
            "[depth {}, cycle {}] expand (out len {})",
            depth,
            cycle,
            os.len(),
        );

        macro_rules! apply_outcome {
            ($outcome: expr, $saved: expr) => {
                match $outcome {
                    BranchOutcome::Advance(rest) => {
                        is = rest;
                        continue 'expand_all;
                    }
                    BranchOutcome::Rewind(rest) => {
                        is = Box::new($saved.into_iter().chain(rest));
                    }
                }
            };
        }

        let first = match is.next() {
            // First, if TS is the empty set, the result is the empty set.
            None => return Ok(()),
            Some(x) => x,
        };

        // Otherwise, if the token sequence begins with a token whose hide set
        // contains that token, then the result is the token sequence beginning
        // with that token (including its hide set) followed by the result of
        // expand on the rest of the token sequence.
        if first.hides(&first.0) {
            log::trace!("macro {} is hidden by hideset of {:?}", first.0, first);
            os.push(first.clone());
            continue 'expand_all;
        }

        // Otherwise, if the token sequence begins with an object-like macro, the
        // result is the expansion of the rest of the token sequence beginning with
        // the sequence returned by subst invoked with the replacement token
        // sequence for the macro, two empty sets, the union of the macro’s hide set
        // and the macro itself, and an empty set.
        if let Token::Name(name) = &first.0 {
            if let SymbolState::Defined(def) = ctx.lookup(name) {
                let mut saved = vec![];

                let outcome =
                    expand_single_macro_invocation(is, os, name, &first, def, &mut saved, depth)?;
                apply_outcome!(outcome, saved);
            }
        }

        // Concatenate strings
        if let THS(Token::Str(l), hs_l) = &first {
            let mut parts = vec![l.to_string()];
            let mut hs = hs_l.clone();

            'concat_strings: loop {
                let mut saved = vec![];
                match skip_ws(&mut is, &mut saved) {
                    None => {
                        // rewind
                        is = Box::new(saved.into_iter().chain(is));
                        break 'concat_strings;
                    }
                    Some(THS(Token::Str(r), hs_r)) => {
                        parts.push(r.to_string());
                        hs = super::hs_union(&hs, &hs_r);
                    }
                    Some(tok) => {
                        saved.push(tok);
                        // rewind
                        is = Box::new(saved.into_iter().chain(is));
                        break 'concat_strings;
                    }
                }
            }

            os.push(THS(Token::Str(parts.join("")), hs));
            continue 'expand_all;
        }

        // Expand `DEFINED x`, `DEFINED(x)`, `DEFINED (x)`, `DEFINED(  x)`, etc.
        if let Token::Defined = &first.0 {
            let mut saved = vec![];
            let outcome = expand_defined(is, os, &mut saved, ctx, &first, depth)?;
            apply_outcome!(outcome, saved);
        }

        // Verbatim token
        os.push(first);
    }
}

pub enum BranchOutcome<'a> {
    Advance(Box<dyn Iterator<Item = THS> + 'a>),
    Rewind(Box<dyn Iterator<Item = THS> + 'a>),
}

// Expand `DEFINED x`, `DEFINED(x)`, `DEFINED (x)`, `DEFINED(  x)`, etc.
fn expand_defined<'a>(
    mut is: Box<dyn Iterator<Item = THS> + 'a>,
    os: &mut Vec<THS>,
    saved: &mut Vec<THS>,
    ctx: &Context,
    first: &THS,
    depth: usize,
) -> Result<BranchOutcome<'a>, ExpandError> {
    let next = skip_ws(&mut is, saved)
        .ok_or_else(|| ExpandError::InvalidDefined("EOF immediately after `defined`".into()))?;

    let def = match &next.0 {
        Token::Name(name) => ctx.lookup(name),
        Token::Pun('(') => {
            let next = skip_ws(&mut is, saved).ok_or_else(|| {
                ExpandError::InvalidDefined("EOF immediately after `defined(`".into())
            })?;

            let name = match &next.0 {
                Token::Name(name) => name,
                tok => {
                    return Err(ExpandError::InvalidDefined(format!(
                        "unexpected token after `defined(`: expected name, got {:#?}",
                        tok
                    )))
                }
            };
            let next = skip_ws(&mut is, saved)
                .ok_or_else(|| ExpandError::InvalidDefined("EOF after `defined(NAME`".into()))?;
            match &next.0 {
                Token::Pun(')') => {} // good!
                tok => {
                    return Err(ExpandError::InvalidDefined(format!(
                        "unexpected token after `defined(NAME`: expected `)`, got {:?}",
                        tok
                    )))
                }
            }
            ctx.lookup(name)
        }
        tok => {
            return Err(ExpandError::InvalidDefined(format!(
                "unexpected token after defined operator: {:?}",
                tok
            )))
        }
    };

    let val = match def {
        SymbolState::Undefined => 0,
        SymbolState::Defined(_) => 1,
    };

    os.push(THS(Token::Int(val), first.1.clone()));
    Ok(BranchOutcome::Advance(is))
}

/// Expands a single macro invocation, either object-like or function-like
fn expand_single_macro_invocation<'a>(
    mut is: Box<dyn Iterator<Item = THS> + 'a>,
    os: &mut Vec<THS>,
    name: &str,
    first: &THS,
    def: &Define,
    saved: &mut Vec<THS>,
    depth: usize,
) -> Result<BranchOutcome<'a>, ExpandError> {
    match def {
        Define::ObjectLike { value, .. } => {
            log::trace!("expanding object-like macro {}", def.name());
            let mut hs = first.1.clone();
            hs.insert(name.to_string());
            let mut temp = Vec::new();
            subst(value.as_ths(), None, &hs, &mut temp, depth + 1)?;
            is = Box::new(temp.into_iter().chain(is));
            Ok(BranchOutcome::Advance(is))
        }
        Define::FunctionLike {
            value,
            name,
            params,
        } => {
            match skip_ws(&mut is, saved) {
                Some(THS(Token::Pun('('), _)) => {
                    // looks like a function invocation, continue
                }
                mut val => {
                    if let Some(tok) = val.take() {
                        saved.push(tok)
                    }
                    // rewind
                    return Ok(BranchOutcome::Rewind(is));
                }
            }

            log::trace!("parsing actuals for macro {:?}", first);
            let mut actuals = parse_actuals(&mut is, saved, name)?;
            // panic check: this unwrap can never panic - parse_actuals can only return
            // Ok if it assigns something to it.
            let closparen_hs = actuals.closparen_hs.take().unwrap();

            log::trace!("actuals = {:?}", actuals);

            let mut hs = HashSet::new();
            hs.insert(name.into());

            let sub_hs = super::hs_union(&super::hs_intersection(&first.1, &closparen_hs), &hs);
            let mut temp = Vec::new();
            subst(
                value.as_ths(),
                Some(Params {
                    fp: &params,
                    ap: &actuals,
                }),
                &sub_hs,
                &mut temp,
                depth + 1,
            )?;

            Ok(BranchOutcome::Advance(Box::new(temp.into_iter().chain(is))))
        }
    }
}

/// Skip whitespace, pushing skipped tokens to `saved` for possible rewinding.
fn skip_ws(is: &mut dyn Iterator<Item = THS>, saved: &mut Vec<THS>) -> Option<THS> {
    let mut next = is.next();
    loop {
        match next {
            // as long as we match whitespace, save it and skip it
            Some(t @ THS(Token::WS, _)) => {
                saved.push(t);
                next = is.next();
            }
            // anything else: stop and return it
            Some(t) => return Some(t),
            None => return None,
        }
    }
}

#[derive(Debug)]
struct ParsedActuals {
    actuals: VecDeque<VecDeque<THS>>,
    closparen_hs: Option<HS>,
}

impl ParsedActuals {
    fn new() -> Self {
        let mut actuals = VecDeque::new();
        actuals.push_back(VecDeque::new());
        Self {
            actuals,
            closparen_hs: None,
        }
    }

    fn push(&mut self, tok: THS) {
        // panic check: this unwrap can never panic - actuals is
        // initialized with one element and no elements are ever
        // removed from it.
        self.actuals.back_mut().unwrap().push_back(tok);
    }

    fn next_arg(&mut self) {
        self.actuals.push_back(VecDeque::new());
    }
}

/// Parse arguments for macro invocations
///
///     FOO(BAR(A, B), C)
///         ^           ^
///         starting    ending here
///         here
///
fn parse_actuals(
    is: &mut dyn Iterator<Item = THS>,
    saved: &mut Vec<THS>,
    name: &str,
) -> Result<ParsedActuals, ExpandError> {
    let mut res = ParsedActuals::new();
    let mut depth = 1;

    // parse until we reach end of input (error), or until all parentheses are balanced
    while depth > 0 {
        match is.next() {
            None => {
                return Err(ExpandError::UnclosedMacroInvocation {
                    name: name.to_string(),
                })
            }
            Some(tok) => {
                match depth {
                    1 => match &tok.0 {
                        Token::Pun(',') => {
                            res.next_arg();
                        }
                        Token::Pun('(') => {
                            depth += 1;
                            res.push(tok.clone());
                        }
                        Token::Pun(')') => {
                            depth -= 1;
                            res.closparen_hs = Some(tok.1.clone());
                        }
                        _ => {
                            res.push(tok.clone());
                        }
                    },
                    _ => {
                        // depth > 1 - keep track of parens but do not advance
                        // arguments
                        match &tok.0 {
                            Token::Pun('(') => depth += 1,
                            Token::Pun(')') => depth -= 1,
                            _ => {}
                        };
                        res.push(tok.clone());
                    }
                }
                saved.push(tok);
            }
        }
    }

    // trim whitespace from all arguments
    for arg in res.actuals.iter_mut() {
        while let Some(THS(Token::WS, _)) = arg.front() {
            arg.pop_front();
        }
        while let Some(THS(Token::WS, _)) = arg.back() {
            arg.pop_back();
        }
    }

    Ok(res)
}

struct Params<'a> {
    /// formal parameters
    fp: &'a MacroParams,
    /// actual parameters (aka arguments)
    ap: &'a ParsedActuals,
}

impl Params<'_> {
    fn lookup<N: AsRef<str>>(&self, name: N) -> Result<Option<&VecDeque<THS>>, ExpandError> {
        if let Some(&index) = self.fp.names.get(name.as_ref()) {
            if let Some(actual) = self.ap.actuals.get(index) {
                return Ok(Some(actual));
            } else {
                return Err(ExpandError::MissingMacroParam(format!(
                    "macro param {} should be passed as an argument",
                    name.as_ref()
                )));
            }
        }

        Ok(None)
    }
}

fn subst<'a>(
    mut is: Box<dyn Iterator<Item = THS> + 'a>,
    params: Option<Params<'a>>,
    hs: &'a HashSet<String>,
    os: &'a mut Vec<THS>,
    depth: usize,
) -> Result<(), ExpandError> {
    let mut cycle = 0;

    'subst_all: loop {
        cycle += 1;
        log::trace!(
            "[depth {}, cycle {}] subst (out len = {})",
            depth,
            cycle,
            os.len()
        );

        match is.next() {
            None => return Ok(()),
            Some(first) => {
                if let Token::Stringize = &first.0 {
                    let mut saved = vec![];
                    let tok = skip_ws(&mut is, &mut saved).ok_or_else(|| {
                        ExpandError::InvalidStringizing("encountered EOF after `#`".into())
                    })?;
                    log::trace!("stringize => tok = {:?}", tok);

                    let name = match &tok.0 {
                        Token::Name(name) => name,
                        tok => {
                            return Err(ExpandError::InvalidStringizing(format!(
                                "expected name after stringizing operator `#`, got {:?}",
                                tok
                            )))
                        }
                    };
                    log::trace!("stringize => name = {:?}", name);

                    if let Some(params) = params.as_ref() {
                        log::trace!("stringize => fp = {:?}, ap = {:?}", params.fp, params.ap);
                        if let Some(sel) = params.lookup(name.as_str())? {
                            log::trace!("stringize => sel = {:?}", sel);

                            let mut s = String::new();
                            use std::fmt::Write;
                            for tok in sel {
                                write!(&mut s, "{}", tok.0).unwrap();
                            }
                            let stringized = THS(Token::Str(s), tok.1.clone());
                            log::trace!("stringized {:?} => {:?}", tok, stringized);
                            os.push(stringized);
                            continue 'subst_all;
                        }
                    }
                }

                if let Token::Paste = &first.0 {
                    let mut saved = vec![];
                    let rhs = skip_ws(&mut is, &mut saved).ok_or_else(|| {
                        ExpandError::InvalidTokenPaste(
                            "encountered EOF immediately after `##`".into(),
                        )
                    })?;

                    let mut lhs = THS(Token::WS, Default::default());
                    while let Token::WS = &lhs.0 {
                        lhs = os.pop().ok_or_else(|| {
                            ExpandError::InvalidTokenPaste(
                                "no left-hand-side operand before `##`".into(),
                            )
                        })?;
                    }

                    if let Token::Name(name) = &rhs.0 {
                        if let Some(params) = params.as_ref() {
                            if let Some(sel) = params.lookup(name.as_str())? {
                                let mut rest = sel.iter().cloned();
                                let rhs = rest.next().ok_or_else(|| ExpandError::InvalidTokenPaste(
                                        format!("no right-hand-side operand after `##` (after substituting argument {:?})", name)
                                    )
                                )?;

                                log::trace!(
                                    "pasting, lhs = {:?}, rhs-argument = {:?}, rest = {:?}",
                                    lhs,
                                    rhs,
                                    rest
                                );
                                os.push(lhs.glue(rhs));
                                os.extend(rest);
                                continue 'subst_all;
                            }
                        }
                    }

                    log::trace!("pasting, lhs = {:?}, rhs = {:?}", lhs, rhs);
                    os.push(lhs.glue(rhs));
                    continue 'subst_all;
                }

                // Regular argument replacement
                if let Some(params) = params.as_ref() {
                    if let THS(Token::Name(name), _) = &first {
                        if let Some(sel) = params.lookup(name)? {
                            log::trace!("regular argument replacement: {} => {:?}", name, sel);
                            os.extend(sel.iter().cloned());
                            continue 'subst_all;
                        }
                    }
                }

                // Verbatim token
                let mut tok = first.clone();
                tok.1.extend(hs.iter().cloned());
                os.push(tok);
                continue 'subst_all;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::grammar;
    use grammar::Directive;

    fn def(ctx: &mut Context, input: &str) {
        let dir = grammar::directive(input)
            .expect("test directive must be parsable")
            .expect("test must specify exactly one directive");
        let def = match dir {
            Directive::Define(d) => d,
            _ => panic!(),
        };
        ctx.push(def)
    }

    fn exp(ctx: &Context, input: &str, output: &str) {
        log::debug!("=============================================");
        let input = grammar::token_stream(input).unwrap();
        let expected = grammar::token_stream(output).unwrap();

        let mut actual = vec![];
        expand(input.as_ths(), &mut actual, &ctx, 0).unwrap();
        let actual: TokenSeq = actual
            .into_iter()
            .map(|tok| tok.0)
            .collect::<Vec<_>>()
            .into();

        log::debug!("expected = {:?}", expected);
        log::debug!("actual = {:?}", actual);
        assert_eq!(actual, expected, "(actual is on the left)");
    }

    #[test]
    fn nested_invocation_rescan() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define ONE TWO");
        def(&mut ctx, "#define TWO THREE");
        def(&mut ctx, "#define THREE FOUR");
        def(&mut ctx, "#define FOUR FIVE");
        def(&mut ctx, "#define FIVE 21");

        exp(&ctx, "ONE+3", "21+3");
    }

    #[test]
    fn macro_invocation_strip_whitespace() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define ADD(x,y) x+y");
        exp(&ctx, "ADD   (  1,  2  )", "1+2");
    }

    #[test]
    fn undefined_macro_invocation_keep_verbatim() {
        let mut ctx = Context::new();
        let input = "ADD   (  1,  2  )";
        exp(&ctx, input, input);
    }

    #[test]
    fn empty() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define EMPTY() ");
        exp(&ctx, "defined EMPTY", "1");
        exp(&ctx, "defined (EMPTY)", "1");
        exp(&ctx, "defined(EMPTY )", "1");
        exp(&ctx, "defined  (    EMPTY  ) ", "1 ");

        exp(&ctx, "EMPTY()", "");
        exp(&ctx, "1+EMPTY()3", "1+3");
    }

    #[test]
    fn identity() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define IDENTITY(x) x");
        exp(&ctx, "IDENTITY(9)+IDENTITY(2)", "9+2");
    }

    #[test]
    fn add_mul() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define ADD(x, y) x+y");
        def(&mut ctx, "#define MUL(x, y) x*y");
        exp(&ctx, "ADD(MUL(1,2),3)", "1*2+3");
        exp(&ctx, "ADD(ADD(ADD(1,2),3),4)", "1+2+3+4");
    }

    #[test]
    fn hideset() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define FOO(x) FOO()");
        exp(&ctx, "FOO(y)", "FOO()");
    }

    #[test]
    fn paste() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define PASTE(x, y) x ## y");
        def(&mut ctx, "#define PASTE_PRE(x) pre ## x");
        def(&mut ctx, "#define PASTE_POST(x) x ## post");
        exp(&ctx, "PASTE(foo,bar)", "foobar");
        exp(&ctx, "PASTE(123,456)", "123456");
        exp(&ctx, "PASTE_PRE(foo)", "prefoo");
        exp(&ctx, "PASTE_POST(foo)", "foopost");
    }

    #[test]
    fn adjacent_string_literals() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define ADJ(a, b) a b");
        exp(&ctx, r#"ADJ("foo", "bar")"#, r#""foobar""#);
    }

    #[test]
    fn stringize() {
        let mut ctx = Context::new();
        def(&mut ctx, "#define STRGZ(x) # x");
        def(&mut ctx, "#define STRGZ2(x, y) # x # y");
        def(&mut ctx, "#define STRGZ3(x, y, z) # x # y # z");
        exp(&ctx, "STRGZ(2 + 3)", r#""2 + 3""#);
        exp(&ctx, "STRGZ(   2 + 3        )", r#""2 + 3""#);
        exp(&ctx, "STRGZ2(  foo ,  bar )", r#""foo" "bar""#);
        exp(&ctx, "STRGZ3( foo, bar , baz)", r#""foo" "bar" "baz""#);
    }
}
