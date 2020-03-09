use lang_c::ast;
use lang_c::span::Node;
use matches::matches;

pub(crate) fn nodes<'a, T>(nodes: &'a [Node<T>]) -> impl Iterator<Item = &'a T> + 'a
where
    T: 'a,
{
    nodes.iter().map(borrow_node)
}

pub(crate) fn borrow_node<T>(node: &Node<T>) -> &T {
    &node.node
}

pub(crate) trait AsSpecifierQualifier {
    fn as_specqual(&self) -> Option<ast::SpecifierQualifier>;
}

impl AsSpecifierQualifier for ast::SpecifierQualifier {
    fn as_specqual(&self) -> Option<ast::SpecifierQualifier> {
        Some(self.clone())
    }
}

impl AsSpecifierQualifier for ast::DeclarationSpecifier {
    fn as_specqual(&self) -> Option<ast::SpecifierQualifier> {
        match self {
            Self::TypeSpecifier(ts) => Some(ast::SpecifierQualifier::TypeSpecifier(ts.clone())),
            Self::TypeQualifier(tq) => Some(ast::SpecifierQualifier::TypeQualifier(tq.clone())),
            _ => None,
        }
    }
}

pub(crate) trait SpecifierQualifierExt {
    fn is_const(&self) -> bool;
}

impl SpecifierQualifierExt for ast::SpecifierQualifier {
    fn is_const(&self) -> bool {
        matches!(
            self,
            Self::TypeQualifier(Node {
                node: ast::TypeQualifier::Const,
                ..
            })
        )
    }
}

pub(crate) trait DeclarationExt {
    fn get_storage_class(&self) -> Option<&ast::StorageClassSpecifier>;
}

impl DeclarationExt for ast::Declaration {
    fn get_storage_class(&self) -> Option<&ast::StorageClassSpecifier> {
        for dspec in nodes(&self.specifiers[..]) {
            if let ast::DeclarationSpecifier::StorageClass(sc) = dspec {
                return Some(&sc.node);
            }
        }
        None
    }
}

pub(crate) trait DeclaratorExt {
    fn has_pointer(&self) -> bool;
    fn get_function(&self) -> Option<&ast::FunctionDeclarator>;
    fn get_identifier(&self) -> Option<&ast::Identifier>;
}

impl DeclaratorExt for ast::Declarator {
    fn has_pointer(&self) -> bool {
        nodes(&self.derived[..]).any(|der| matches!(der, ast::DerivedDeclarator::Pointer(_)))
    }

    fn get_function(&self) -> Option<&ast::FunctionDeclarator> {
        for derived in nodes(&self.derived[..]) {
            if let ast::DerivedDeclarator::Function(fd) = derived {
                return Some(&fd.node);
            }
        }

        None
    }

    fn get_identifier(&self) -> Option<&ast::Identifier> {
        if let ast::DeclaratorKind::Identifier(Node { node: id, .. }) = &self.kind.node {
            Some(id)
        } else {
            None
        }
    }
}

pub(crate) trait VoidExt {
    fn takes_nothing(&self) -> bool;
    fn returns_nothing(&self) -> bool;
}

impl VoidExt for ast::FunctionDeclarator {
    fn takes_nothing(&self) -> bool {
        if self.parameters.len() != 1 {
            return false;
        }

        let Node { node: param, .. } = &self.parameters[0];
        param.is_void()
    }

    fn returns_nothing(&self) -> bool {
        unimplemented!()
    }
}

pub(crate) trait Typed {
    fn declarator(&self) -> Option<&ast::Declarator>;
    fn specifiers(&self) -> Box<dyn Iterator<Item = &dyn AsSpecifierQualifier> + '_>;

    fn is_void(&self) -> bool {
        let is_pointer = self
            .declarator()
            .map(|d| d.has_pointer())
            .unwrap_or_default();

        let has_void = self.specifiers().any(|spec| {
            matches!(
                spec.as_specqual(),
                Some(ast::SpecifierQualifier::TypeSpecifier(Node {
                    node: ast::TypeSpecifier::Void,
                    ..
                }))
            )
        });

        has_void && !is_pointer
    }
}

impl Typed for ast::ParameterDeclaration {
    fn specifiers(&self) -> Box<dyn Iterator<Item = &dyn AsSpecifierQualifier> + '_> {
        Box::new(nodes(&self.specifiers[..]).map(|x| x as &dyn AsSpecifierQualifier))
    }

    fn declarator(&self) -> Option<&lang_c::ast::Declarator> {
        self.declarator.as_ref().map(borrow_node)
    }
}
