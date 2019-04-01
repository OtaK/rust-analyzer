use ra_db::SourceDatabase;
use ra_syntax::{
    Direction, SyntaxNode, TextRange, TextUnit, AstNode, SyntaxElement,
    algo::{find_covering_element, find_token_at_offset, TokenAtOffset},
    SyntaxKind::*, SyntaxToken,
    ast::Comment,
};

use crate::{FileRange, db::RootDatabase};

// FIXME: restore macro support
pub(crate) fn extend_selection(db: &RootDatabase, frange: FileRange) -> TextRange {
    let source_file = db.parse(frange.file_id);
    try_extend_selection(source_file.syntax(), frange.range).unwrap_or(frange.range)
}

fn try_extend_selection(root: &SyntaxNode, range: TextRange) -> Option<TextRange> {
    let string_kinds = [COMMENT, STRING, RAW_STRING, BYTE_STRING, RAW_BYTE_STRING];
    let list_kinds = [
        FIELD_PAT_LIST,
        MATCH_ARM_LIST,
        NAMED_FIELD_DEF_LIST,
        POS_FIELD_DEF_LIST,
        NAMED_FIELD_LIST,
        ENUM_VARIANT_LIST,
        USE_TREE_LIST,
        TYPE_PARAM_LIST,
        TYPE_ARG_LIST,
        PARAM_LIST,
        ARG_LIST,
        ARRAY_EXPR,
    ];

    if range.is_empty() {
        let offset = range.start();
        let mut leaves = find_token_at_offset(root, offset);
        if leaves.clone().all(|it| it.kind() == WHITESPACE) {
            return Some(extend_ws(root, leaves.next()?, offset));
        }
        let leaf_range = match leaves {
            TokenAtOffset::None => return None,
            TokenAtOffset::Single(l) => {
                if string_kinds.contains(&l.kind()) {
                    extend_single_word_in_comment_or_string(l, offset).unwrap_or_else(|| l.range())
                } else {
                    l.range()
                }
            }
            TokenAtOffset::Between(l, r) => pick_best(l, r).range(),
        };
        return Some(leaf_range);
    };
    let node = match find_covering_element(root, range) {
        SyntaxElement::Token(token) => {
            if token.range() != range {
                return Some(token.range());
            }
            if let Some(comment) = Comment::cast(token) {
                if let Some(range) = extend_comments(comment) {
                    return Some(range);
                }
            }
            token.parent()
        }
        SyntaxElement::Node(node) => node,
    };
    if node.range() != range {
        return Some(node.range());
    }

    // Using shallowest node with same range allows us to traverse siblings.
    let node = node.ancestors().take_while(|n| n.range() == node.range()).last().unwrap();

    if node.parent().map(|n| list_kinds.contains(&n.kind())) == Some(true) {
        if let Some(range) = extend_list_item(node) {
            return Some(range);
        }
    }

    node.parent().map(|it| it.range())
}

fn extend_single_word_in_comment_or_string(
    leaf: SyntaxToken,
    offset: TextUnit,
) -> Option<TextRange> {
    let text: &str = leaf.text();
    let cursor_position: u32 = (offset - leaf.range().start()).into();

    let (before, after) = text.split_at(cursor_position as usize);

    fn non_word_char(c: char) -> bool {
        !(c.is_alphanumeric() || c == '_')
    }

    let start_idx = before.rfind(non_word_char)? as u32;
    let end_idx = after.find(non_word_char).unwrap_or(after.len()) as u32;

    let from: TextUnit = (start_idx + 1).into();
    let to: TextUnit = (cursor_position + end_idx).into();

    let range = TextRange::from_to(from, to);
    if range.is_empty() {
        None
    } else {
        Some(range + leaf.range().start())
    }
}

fn extend_ws(root: &SyntaxNode, ws: SyntaxToken, offset: TextUnit) -> TextRange {
    let ws_text = ws.text();
    let suffix = TextRange::from_to(offset, ws.range().end()) - ws.range().start();
    let prefix = TextRange::from_to(ws.range().start(), offset) - ws.range().start();
    let ws_suffix = &ws_text.as_str()[suffix];
    let ws_prefix = &ws_text.as_str()[prefix];
    if ws_text.contains('\n') && !ws_suffix.contains('\n') {
        if let Some(node) = ws.next_sibling_or_token() {
            let start = match ws_prefix.rfind('\n') {
                Some(idx) => ws.range().start() + TextUnit::from((idx + 1) as u32),
                None => node.range().start(),
            };
            let end = if root.text().char_at(node.range().end()) == Some('\n') {
                node.range().end() + TextUnit::of_char('\n')
            } else {
                node.range().end()
            };
            return TextRange::from_to(start, end);
        }
    }
    ws.range()
}

fn pick_best<'a>(l: SyntaxToken<'a>, r: SyntaxToken<'a>) -> SyntaxToken<'a> {
    return if priority(r) > priority(l) { r } else { l };
    fn priority(n: SyntaxToken) -> usize {
        match n.kind() {
            WHITESPACE => 0,
            IDENT | SELF_KW | SUPER_KW | CRATE_KW | LIFETIME => 2,
            _ => 1,
        }
    }
}

/// Extend list item selection to include nearby comma and whitespace.
fn extend_list_item(node: &SyntaxNode) -> Option<TextRange> {
    fn is_single_line_ws(node: &SyntaxToken) -> bool {
        node.kind() == WHITESPACE && !node.text().contains('\n')
    }

    fn nearby_comma(node: &SyntaxNode, dir: Direction) -> Option<SyntaxToken> {
        node.siblings_with_tokens(dir)
            .skip(1)
            .skip_while(|node| match node {
                SyntaxElement::Node(_) => false,
                SyntaxElement::Token(it) => is_single_line_ws(it),
            })
            .next()
            .and_then(|it| it.as_token())
            .filter(|node| node.kind() == COMMA)
    }

    if let Some(comma_node) = nearby_comma(node, Direction::Prev) {
        return Some(TextRange::from_to(comma_node.range().start(), node.range().end()));
    }
    if let Some(comma_node) = nearby_comma(node, Direction::Next) {
        // Include any following whitespace when comma if after list item.
        let final_node = comma_node
            .next_sibling_or_token()
            .and_then(|it| it.as_token())
            .filter(|node| is_single_line_ws(node))
            .unwrap_or(comma_node);

        return Some(TextRange::from_to(node.range().start(), final_node.range().end()));
    }

    None
}

fn extend_comments(comment: Comment) -> Option<TextRange> {
    let prev = adj_comments(comment, Direction::Prev);
    let next = adj_comments(comment, Direction::Next);
    if prev != next {
        Some(TextRange::from_to(prev.syntax().range().start(), next.syntax().range().end()))
    } else {
        None
    }
}

fn adj_comments(comment: Comment, dir: Direction) -> Comment {
    let mut res = comment;
    for element in comment.syntax().siblings_with_tokens(dir) {
        let token = match element.as_token() {
            None => break,
            Some(token) => token,
        };
        if let Some(c) = Comment::cast(token) {
            res = c
        } else if token.kind() != WHITESPACE || token.text().contains("\n\n") {
            break;
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use ra_syntax::{SourceFile, AstNode};
    use test_utils::extract_offset;

    use super::*;

    fn do_check(before: &str, afters: &[&str]) {
        let (cursor, before) = extract_offset(before);
        let file = SourceFile::parse(&before);
        let mut range = TextRange::offset_len(cursor, 0.into());
        for &after in afters {
            range = try_extend_selection(file.syntax(), range).unwrap();
            let actual = &before[range];
            assert_eq!(after, actual);
        }
    }

    #[test]
    fn test_extend_selection_arith() {
        do_check(r#"fn foo() { <|>1 + 1 }"#, &["1", "1 + 1", "{ 1 + 1 }"]);
    }

    #[test]
    fn test_extend_selection_list() {
        do_check(r#"fn foo(<|>x: i32) {}"#, &["x", "x: i32"]);
        do_check(r#"fn foo(<|>x: i32, y: i32) {}"#, &["x", "x: i32", "x: i32, "]);
        do_check(r#"fn foo(<|>x: i32,y: i32) {}"#, &["x", "x: i32", "x: i32,"]);
        do_check(r#"fn foo(x: i32, <|>y: i32) {}"#, &["y", "y: i32", ", y: i32"]);
        do_check(r#"fn foo(x: i32, <|>y: i32, ) {}"#, &["y", "y: i32", ", y: i32"]);
        do_check(r#"fn foo(x: i32,<|>y: i32) {}"#, &["y", "y: i32", ",y: i32"]);

        do_check(r#"const FOO: [usize; 2] = [ 22<|> , 33];"#, &["22", "22 , "]);
        do_check(r#"const FOO: [usize; 2] = [ 22 , 33<|>];"#, &["33", ", 33"]);
        do_check(r#"const FOO: [usize; 2] = [ 22 , 33<|> ,];"#, &["33", ", 33"]);

        do_check(
            r#"
const FOO: [usize; 2] = [
    22,
    <|>33,
]"#,
            &["33", "33,"],
        );

        do_check(
            r#"
const FOO: [usize; 2] = [
    22
    , 33<|>,
]"#,
            &["33", ", 33"],
        );
    }

    #[test]
    fn test_extend_selection_start_of_the_line() {
        do_check(
            r#"
impl S {
<|>    fn foo() {

    }
}"#,
            &["    fn foo() {\n\n    }\n"],
        );
    }

    #[test]
    fn test_extend_selection_doc_comments() {
        do_check(
            r#"
struct A;

/// bla
/// bla
struct B {
    <|>
}
            "#,
            &["\n    \n", "{\n    \n}", "/// bla\n/// bla\nstruct B {\n    \n}"],
        )
    }

    #[test]
    fn test_extend_selection_comments() {
        do_check(
            r#"
fn bar(){}

// fn foo() {
// 1 + <|>1
// }

// fn foo(){}
    "#,
            &["1", "// 1 + 1", "// fn foo() {\n// 1 + 1\n// }"],
        );

        do_check(
            r#"
// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum Direction {
//  <|>   Next,
//     Prev
// }
"#,
            &[
                "//     Next,",
                "// #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n// pub enum Direction {\n//     Next,\n//     Prev\n// }",
            ],
        );

        do_check(
            r#"
/*
foo
_bar1<|>*/
"#,
            &["_bar1", "/*\nfoo\n_bar1*/"],
        );

        do_check(r#"//!<|>foo_2 bar"#, &["foo_2", "//!foo_2 bar"]);

        do_check(r#"/<|>/foo bar"#, &["//foo bar"]);
    }

    #[test]
    fn test_extend_selection_prefer_idents() {
        do_check(
            r#"
fn main() { foo<|>+bar;}
"#,
            &["foo", "foo+bar"],
        );
        do_check(
            r#"
fn main() { foo+<|>bar;}
"#,
            &["bar", "foo+bar"],
        );
    }

    #[test]
    fn test_extend_selection_prefer_lifetimes() {
        do_check(r#"fn foo<<|>'a>() {}"#, &["'a", "<'a>"]);
        do_check(r#"fn foo<'a<|>>() {}"#, &["'a", "<'a>"]);
    }

    #[test]
    fn test_extend_selection_select_first_word() {
        do_check(r#"// foo bar b<|>az quxx"#, &["baz", "// foo bar baz quxx"]);
        do_check(
            r#"
impl S {
fn foo() {
// hel<|>lo world
}
}
"#,
            &["hello", "// hello world"],
        );
    }

    #[test]
    fn test_extend_selection_string() {
        do_check(
            r#"
fn bar(){}

" fn f<|>oo() {"
"#,
            &["foo", "\" fn foo() {\""],
        );
    }
}
