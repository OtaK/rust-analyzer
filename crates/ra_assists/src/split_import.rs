use hir::db::HirDatabase;
use ra_syntax::{
    TextUnit, AstNode, SyntaxKind::COLONCOLON,
    ast,
    algo::generate,
};

use crate::{AssistCtx, Assist, AssistId};

pub(crate) fn split_import(mut ctx: AssistCtx<impl HirDatabase>) -> Option<Assist> {
    let colon_colon = ctx.token_at_offset().find(|leaf| leaf.kind() == COLONCOLON)?;
    let path = ast::Path::cast(colon_colon.parent())?;
    let top_path = generate(Some(path), |it| it.parent_path()).last()?;

    let use_tree = top_path.syntax().ancestors().find_map(ast::UseTree::cast);
    if use_tree.is_none() {
        return None;
    }

    let l_curly = colon_colon.range().end();
    let r_curly = match top_path.syntax().parent().and_then(ast::UseTree::cast) {
        Some(tree) => tree.syntax().range().end(),
        None => top_path.syntax().range().end(),
    };

    ctx.add_action(AssistId("split_import"), "split import", |edit| {
        edit.target(colon_colon.range());
        edit.insert(l_curly, "{");
        edit.insert(r_curly, "}");
        edit.set_cursor(l_curly + TextUnit::of_str("{"));
    });

    ctx.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::{check_assist, check_assist_target};

    #[test]
    fn test_split_import() {
        check_assist(
            split_import,
            "use crate::<|>db::RootDatabase;",
            "use crate::{<|>db::RootDatabase};",
        )
    }

    #[test]
    fn split_import_works_with_trees() {
        check_assist(
            split_import,
            "use algo:<|>:visitor::{Visitor, visit}",
            "use algo::{<|>visitor::{Visitor, visit}}",
        )
    }

    #[test]
    fn split_import_target() {
        check_assist_target(split_import, "use algo::<|>visitor::{Visitor, visit}", "::");
    }
}
