initSidebarItems({"enum":[["AdtDef",""],["DefWithBody","The defs which have a body."],["FieldSource",""],["ImplItem",""],["ImportSource",""],["ModuleDef","The defs which can be visible in the module."],["ModuleSource",""],["Namespace",""],["PathKind",""],["Resolution",""],["Ty","A type."],["TypeCtor","A type constructor or type name: this might be something like the primitive type `bool`, a struct like `Vec`, or things like function pointers or tuples."]],"macro":[["crate_graph",""]],"mod":[["db",""],["diagnostics",""],["mock",""],["source_binder",""]],"struct":[["ApplicationTy","A nominal type with (maybe 0) type parameters. This might be a primitive type like `bool`, a struct, tuple, function pointer, reference or several other things."],["AstIdMap","Maps items' `SyntaxNode`s to `ErasedFileAstId`s and back."],["Const",""],["ConstSignature","The declared signature of a const."],["Crate","hir::Crate describes a single crate. It's the main interface with which a crate's dependencies interact. Mostly, it should be just a proxy for the root module."],["CrateDependency",""],["Documentation","Holds documentation"],["Enum",""],["EnumVariant",""],["ErasedFileAstId",""],["ExprScopes",""],["FnSignature","The declared signature of a function."],["Function",""],["HirFileId","hir makes heavy use of ids: integer (u32) handlers to various things. You can think of id as a pointer (but without a lifetime) or a file descriptor (but for hir objects)."],["ImplBlock",""],["ImportId",""],["MacroCallId","`MacroCallId` identifies a particular macro invocation, like `println!(\"Hello, {}\", world)`."],["MacroCallLoc",""],["MacroDefId",""],["Module",""],["Name","`Name` is a wrapper around string, which is used in hir for both references and declarations. In theory, names should also carry hygiene info, but we are not there yet!"],["Path",""],["PerNs",""],["Resolver",""],["ScopeEntryWithSyntax",""],["ScopesWithSourceMap",""],["Static",""],["Struct",""],["StructField",""],["Substs","A list of substitutions for generic parameters."],["Trait",""],["TypeAlias",""]],"trait":[["Docs",""],["HirDisplay",""]]});