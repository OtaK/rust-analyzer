use ra_syntax::{
    SourceFile, TextUnit,
    algo::find_token_at_offset,
    SyntaxKind::{self, *},
    ast::AstNode,
};

pub fn matching_brace(file: &SourceFile, offset: TextUnit) -> Option<TextUnit> {
    const BRACES: &[SyntaxKind] =
        &[L_CURLY, R_CURLY, L_BRACK, R_BRACK, L_PAREN, R_PAREN, L_ANGLE, R_ANGLE];
    let (brace_node, brace_idx) = find_token_at_offset(file.syntax(), offset)
        .filter_map(|node| {
            let idx = BRACES.iter().position(|&brace| brace == node.kind())?;
            Some((node, idx))
        })
        .next()?;
    let parent = brace_node.parent();
    let matching_kind = BRACES[brace_idx ^ 1];
    let matching_node = parent.children_with_tokens().find(|node| node.kind() == matching_kind)?;
    Some(matching_node.range().start())
}

#[cfg(test)]
mod tests {
    use test_utils::{add_cursor, assert_eq_text, extract_offset};

    use super::*;

    #[test]
    fn test_matching_brace() {
        fn do_check(before: &str, after: &str) {
            let (pos, before) = extract_offset(before);
            let file = SourceFile::parse(&before);
            let new_pos = match matching_brace(&file, pos) {
                None => pos,
                Some(pos) => pos,
            };
            let actual = add_cursor(&before, new_pos);
            assert_eq_text!(after, &actual);
        }

        do_check("struct Foo { a: i32, }<|>", "struct Foo <|>{ a: i32, }");
    }
}
