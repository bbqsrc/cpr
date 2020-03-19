use super::*;

fn expr(s: &str) -> Expr {
    Expr::from(s)
}

fn parse(source: &str) -> ParsedUnit {
    ParsedUnit::parse(source.as_ref()).expect("unit should parse")
}

fn chunks(source: &str) -> Vec<Chunk> {
    parse(source).chunks().unwrap()
}

fn test(source: &str, expected_chunks: &[(Expr, &str)]) {
    let actual_chunks = chunks(source);
    assert_eq!(actual_chunks.len(), expected_chunks.len());
    for (i, (actual, expected)) in actual_chunks.iter().zip(expected_chunks.iter()).enumerate() {
        assert_eq!(actual.expr, expected.0, "expr for chunk #{}", i);
        assert_eq!(
            actual.source,
            expected.1.trim().into(),
            "source for chunk #{}",
            i
        );
    }
}

#[test]
fn single_line_comment() {
    test(
        "
// single-line comment
int foo();
    ",
        &[(Expr::True, "int foo();")],
    );
}

#[test]
fn single_line_comment_continued() {
    test(
        "
// single-line comment, \\
continued
int foo();
    ",
        &[(Expr::True, "int foo();")],
    );
}

#[test]
fn multi_line_comment_1_line() {
    test(
        "
/* classic multi-line comment */
int foo();
    ",
        &[(Expr::True, "int foo();")],
    );
}

#[test]
fn multi_line_comment_2_lines() {
    test(
        "
/* classic multi-line comment
 * but on multiple lines */
int foo();
    ",
        &[(Expr::True, "int foo();")],
    );
}

#[test]
fn multi_line_comment_nested() {
    test(
        "
int/* boop */foo();
    ",
        &[(Expr::True, "int foo();")],
    );
}

#[test]
fn string_literal_1() {
    test(
        "
char *c = \"hello /* world */\";
    ",
        &[(Expr::True, "char *c = \"hello /* world */\";")],
    );
}

#[test]
fn string_literal_2() {
    test(
        "
char *c = \"hello // world\";
    ",
        &[(Expr::True, "char *c = \"hello // world\";")],
    );
}

#[test]
fn single_atom_strands() {
    test(
        "
#ifdef FOO
int foo();
#endif

#ifdef BAR
int bar();
#endif
    ",
        &[(expr("FOO"), "int foo();"), (expr("BAR"), "int bar();")],
    )
}

#[test]
fn nested_ifdefs() {
    test(
        "
#ifdef FOO
int foo();
#ifdef BAR
int foobar();
#endif // BAR
#endif // FOO
        ",
        &[
            (expr("FOO"), "int foo();"),
            (expr("FOO") & expr("BAR"), "int foobar();"),
        ],
    );
}

#[test]
fn chunks_gated_struct_field() {
    test(
        "
struct foo {
    int lawful;
#ifdef EVIL
    int evil;
#endif // EVIL
};
        ",
        &[
            (
                expr("EVIL"),
                "struct foo {
int lawful;
int evil;
};",
            ),
            (
                !expr("EVIL"),
                "struct foo {
int lawful;
};",
            ),
        ],
    )
}

#[test]
fn chunks_gated_struct_field_ifelse() {
    test(
        "
struct foo {
    int lawful;
#ifdef EVIL
    int evil;
#else 
    int good;
#endif
};
        ",
        &[
            (
                expr("EVIL"),
                "struct foo {
int lawful;
int evil;
};",
            ),
            (
                !expr("EVIL"),
                "struct foo {
int lawful;
int good;
};",
            ),
        ],
    )
}

#[test]
fn gated_struct_close_ifelse() {
    test(
        "
struct foo {
    int lawful;
#ifdef EVIL
};
#else 
};
#endif
        ",
        &[
            (
                expr("EVIL"),
                "struct foo {
int lawful;
};",
            ),
            (
                !expr("EVIL"),
                "struct foo {
int lawful;
};",
            ),
        ],
    )
}

#[test]
fn gated_struct_close_convoluted_xxx() {
    env_logger::init(); // XXX: remove me

    test(
        "
struct foo {
    int foo;
#ifdef BAR
    struct bar {
        int bar;
#endif // BAR
#ifdef BAZ
    } nested;
#endif // BAZ
};
        ",
        &[
            (
                expr("BAR") & expr("BAZ"),
                "struct foo {
int foo;
struct bar {
int bar;
} nested;
};",
            ),
            (
                !expr("BAR") & !expr("BAZ"),
                "struct foo {
int foo;
};",
            ),
        ],
    )
}
