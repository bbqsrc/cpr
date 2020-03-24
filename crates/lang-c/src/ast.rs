//! Abstract syntax tree
//!
//! Types in this module represent various pieces a C program can
//! contain after preprocessing phase. They mostly follow C11 grammar
//! naming conventions.
//!
//! References to C11 standard given in parenthesis refer to the
//! [ISO/IEC 9899:201x
//! draft](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf)
//! published on April 12, 2011.
//!
//! A number of GNU extensions to the standard C are included here.
//! Types, struct fields or enum variants specific to GNU are marked as
//! "GNU extension" with a link to the relevant section of gcc
//! documentation. Supported extensions are:
//!
//! - attributes in various positions
//! - inline assembly statements and asm labels
//! - extensions to the initializer list syntax
//! - statement expressions
//! - `typeof` type specifiers

use span::Node;

// From 6.4 Lexical elements

/// Variable, function and other names that are not type names
///
/// (C11 6.4.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub name: String,
}

/// Constant literals
///
/// C11 places string literals under primary expressions, thus they
/// are not included here.
///
/// (C11 6.4.4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant {
    Integer(Integer),
    Float(Float),
    Character(String),
}

/// Integer number literal
///
/// (C11 6.4.4.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Integer {
    pub base: IntegerBase,
    pub number: Box<str>,
    pub suffix: IntegerSuffix,
}

/// Base of the integer literal
///
/// (C11 6.4.4.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IntegerBase {
    Decimal,
    Octal,
    Hexademical,
}

/// Suffix of an integer literal
///
/// (C11 6.4.4.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerSuffix {
    /// Minimum size of the integer literal
    pub size: IntegerSize,
    /// Integer literal has unsigned type
    pub unsigned: bool,
    /// Integer literal is an imaginary part of a complex number
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Complex.html) suffixes `i` and `j`.
    pub imaginary: bool,
}

/// Size part of a integer literal suffix
///
/// (C11 6.4.4.1)
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum IntegerSize {
    /// no `l` or `ll`
    Int = 0,
    /// `l`
    Long,
    /// `ll`
    LongLong,
}

/// Floating point number literal
///
/// (C11 6.4.4.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Float {
    pub base: FloatBase,
    pub number: Box<str>,
    pub suffix: FloatSuffix,
}

/// Floating point number base
///
/// (C11 6.4.4.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloatBase {
    Decimal,
    Hexademical,
}

/// Floating point number suffix
///
/// (C11 6.4.4.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FloatSuffix {
    pub format: FloatFormat,
    /// Integer literal is an imaginary part of a complex number
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Complex.html) suffixes `i` and `j`.
    pub imaginary: bool,
}

/// Floating point literal format specified by the suffix
///
/// (C11 6.4.4.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FloatFormat {
    /// `f` suffix
    Float,
    /// no suffix
    Double,
    /// `l` suffix
    LongDouble,
    /// [ISO/IEC TS 18661-2:2015](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1912.pdf)
    /// `df`, `dd`, `dl` suffixes
    ///
    /// [ISO/IEC TS 18661-3:2015](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1945.pdf)
    /// `fN`, `fNx`, `dN`, `dNx` suffixes
    TS18661Format(TS18661FloatType),
}

/// String literal
///
/// (C11 6.4.5)
pub type StringLiteral = Vec<String>;

// From 6.5 Expressions

/// Expressions
///
/// (C11 6.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    /// Identifier
    ///
    /// May be a variable, function name or enumerator. The latter is
    /// different from the standard, where enumerators are classified
    /// as constants.
    ///
    /// (C11 6.5.1)
    Identifier(Box<Node<Identifier>>),
    /// Numeric and character constants
    ///
    /// Enumerator constants, being valid identifiers, are reprented
    /// as `Identifier` in this enum.
    ///
    /// (C11 6.5.1)
    Constant(Box<Node<Constant>>),

    /// String literal
    ///
    /// (C11 6.5.1)
    StringLiteral(Box<Node<StringLiteral>>),

    /// Generic selection
    ///
    /// (C11 6.5.1.1)
    GenericSelection(Box<Node<GenericSelection>>),

    /// Structure and union members
    ///
    /// Both direct (`.`) and indirect (`->`) access.
    ///
    /// (C11 6.5.2)
    Member(Box<Node<MemberExpression>>),

    /// Function call expression
    ///
    /// (C11 6.5.2)
    Call(Box<Node<CallExpression>>),

    /// Compound literal
    ///
    /// (C11 6.5.2)
    CompoundLiteral(Box<Node<CompoundLiteral>>),

    /// Size of a type
    ///
    /// Note: size of an expression is represented with `UnaryOperator::SizeOf`.
    ///
    /// (C11 6.5.3)
    SizeOf(Box<Node<TypeName>>),

    /// Alignment of a type
    ///
    /// (C11 6.5.3)
    AlignOf(Box<Node<TypeName>>),

    /// Unary operators
    ///
    /// This represents both postfix and prefix unary oprators. Postfix expressions that take
    /// additional operands are represented by a separate entry in this enum.
    ///
    /// (C11 6.5.2, c11 6.5.3)
    UnaryOperator(Box<Node<UnaryOperatorExpression>>),

    /// Cast expression
    ///
    /// `(type) expr`
    ///
    /// (C11 6.5.4)
    Cast(Box<Node<CastExpression>>),

    /// Binary operators
    ///
    /// All of C binary operators that can be applied to two expressions.
    ///
    /// (C11 6.5.5 -- 6.5.16)
    BinaryOperator(Box<Node<BinaryOperatorExpression>>),

    /// Conditional operator
    ///
    /// (C11 6.5.15)
    Conditional(Box<Node<ConditionalExpression>>),

    /// Comma operator
    ///
    /// (C11 6.5.17)
    Comma(Box<Vec<Node<Expression>>>),

    /// Member offset expression
    ///
    /// Result of expansion of `offsetof` macro.
    ///
    /// (C11 7.19 §3).
    OffsetOf(Box<Node<OffsetOfExpression>>),

    /// Variable argument list access
    ///
    /// Result of expansion of `va_arg` macro.
    ///
    /// (C11 7.16.1.1).
    VaArg(Box<Node<VaArgExpression>>),

    /// Statement expression
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Statement-Exprs.html)
    Statement(Box<Node<Statement>>),
}

/// Struct or union member access
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemberOperator {
    /// `expression.identifier`
    Direct,
    /// `expression->identifier`
    Indirect,
}

/// Generic selection expression
///
/// (C11 6.5.1.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericSelection {
    pub expression: Box<Node<Expression>>,
    pub associations: Vec<Node<GenericAssociation>>,
}

/// Single element of a generic selection expression
///
/// (C11 6.5.1.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GenericAssociation {
    Type(Node<GenericAssociationType>),
    Default(Box<Node<Expression>>),
}

/// Type match case in a generic selection expression
///
/// (C11 6.5.1.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenericAssociationType {
    pub type_name: Node<TypeName>,
    pub expression: Box<Node<Expression>>,
}

/// Structure and union members
///
/// Both direct (`.`) and indirect (`->`) access.
///
/// (C11 6.5.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemberExpression {
    pub operator: Node<MemberOperator>,
    pub expression: Box<Node<Expression>>,
    pub identifier: Node<Identifier>,
}

/// Function call expression
///
/// (C11 6.5.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallExpression {
    pub callee: Box<Node<Expression>>,
    pub arguments: Vec<Node<Expression>>,
}

/// Compound literal
///
/// (C11 6.5.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CompoundLiteral {
    pub type_name: Node<TypeName>,
    pub initializer_list: Vec<Node<Initializer>>,
}

/// All operators with one operand
///
/// (C11 6.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    /// `operand++`
    PostIncrement,
    /// `operand--`
    PostDecrement,
    /// `++operand`
    PreIncrement,
    /// `--operand`
    PreDecrement,
    /// `&operand`
    Address,
    /// `*operand`
    Indirection,
    /// `+operand`
    Plus,
    /// `-operand`
    Minus,
    /// `~operand`
    Complement,
    /// `!operand`
    Negate,
    /// `sizeof operand`
    SizeOf,
}

/// Unary operator expression
///
/// This represents both postfix and prefix unary oprators. Postfix expressions that take
/// additional operands are represented by a separate entry in this enum.
///
/// (C11 6.5.2, c11 6.5.3)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnaryOperatorExpression {
    pub operator: Node<UnaryOperator>,
    pub operand: Box<Node<Expression>>,
}

/// Cast expression
///
/// `(type) expr`
///
/// (C11 6.5.4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CastExpression {
    pub type_name: Node<TypeName>,
    pub expression: Box<Node<Expression>>,
}

/// All operators with two operands
///
/// (C11 6.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    /// `lhs[rhs]`
    Index,
    /// `lhs * rhs`
    Multiply,
    /// `lhs / rhs`
    Divide,
    /// `lhs % rhs`
    Modulo,
    /// `lhs + rhs`
    Plus,
    /// `lhs - rhs`
    Minus,
    /// `lhs << rhs`
    ShiftLeft,
    /// `lhs >> rhs`
    ShiftRight,
    /// `lhs < rhs`
    Less,
    /// `lhs > rhs`
    Greater,
    /// `lhs <= rhs`
    LessOrEqual,
    /// `lhs >= rhs`
    GreaterOrEqual,
    /// `lhs == rhs`
    Equals,
    /// `lhs != rhs`
    NotEquals,
    /// `lhs & rhs`
    BitwiseAnd,
    /// `lhs ^ rhs`
    BitwiseXor,
    /// `lhs | rhs`
    BitwiseOr,
    /// `lhs && rhs`
    LogicalAnd,
    /// `lhs || rhs`
    LogicalOr,
    /// `lhs = rhs`
    Assign,
    /// `lhs *= rhs`
    AssignMultiply,
    /// `lhs /= rhs`
    AssignDivide,
    /// `lhs %= rhs`
    AssignModulo,
    /// `lhs += rhs`
    AssignPlus,
    /// `lhs -= rhs`
    AssignMinus,
    /// `lhs <<= rhs`
    AssignShiftLeft,
    /// `lhs >>= rhs`
    AssignShiftRight,
    /// `lhs &= rhs`
    AssignBitwiseAnd,
    /// `lhs ^= rhs`
    AssignBitwiseXor,
    /// `lhs |= rhs`
    AssignBitwiseOr,
}

/// Binary operators
///
/// All of C binary operators that can be applied to two expressions.
///
/// (C11 6.5.5 -- 6.5.16)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BinaryOperatorExpression {
    pub operator: Node<BinaryOperator>,
    pub lhs: Box<Node<Expression>>,
    pub rhs: Box<Node<Expression>>,
}

/// Conditional operator
///
/// (C11 6.5.15)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConditionalExpression {
    pub condition: Box<Node<Expression>>,
    pub then_expression: Box<Node<Expression>>,
    pub else_expression: Box<Node<Expression>>,
}

/// Variable argument list access
///
/// Result of expansion of `va_arg` macro.
///
/// (C11 7.16.1.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VaArgExpression {
    pub va_list: Box<Node<Expression>>,
    pub type_name: Node<TypeName>,
}

/// Member offset expression
///
/// Result of expansion of `offsetof` macro.
///
/// (C11 7.19 §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OffsetOfExpression {
    pub type_name: Node<TypeName>,
    pub designator: Node<OffsetDesignator>,
}

/// Offset designator in a `offsetof` macro expansion
///
/// (C11 7.19 §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OffsetDesignator {
    pub base: Node<Identifier>,
    pub members: Vec<Node<OffsetMember>>,
}

/// Single element of an offset designator
///
/// (C11 7.19 §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OffsetMember {
    Member(Node<Identifier>),
    IndirectMember(Node<Identifier>),
    Index(Node<Expression>),
}

// From 6.7 Declarations

/// Variable, function or type declaration
///
/// (C11 6.7)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Declaration {
    pub specifiers: Vec<Node<DeclarationSpecifier>>,
    pub declarators: Vec<Node<InitDeclarator>>,
}

/// Common part of a declaration
///
/// These apply to all declarators in a declaration.
///
/// (C11 6.7)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclarationSpecifier {
    StorageClass(Node<StorageClassSpecifier>),
    TypeSpecifier(Node<TypeSpecifier>),
    TypeQualifier(Node<TypeQualifier>),
    Function(Node<FunctionSpecifier>),
    Alignment(Node<AlignmentSpecifier>),
    /// Vendor-specific declaration extensions that can be mixed with standard specifiers
    Extension(Vec<Node<Extension>>),
}

/// Defines a single name in a declaration
///
/// (C11 6.7.6)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InitDeclarator {
    pub declarator: Node<Declarator>,
    pub initializer: Option<Node<Initializer>>,
}

// From 6.7.1

/// Storage class
///
/// (C11 6.7.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageClassSpecifier {
    /// `typedef`
    Typedef,
    /// `extern`
    Extern,
    /// `static`
    Static,
    /// `_Thread_local`
    ThreadLocal,
    /// `auto`
    Auto,
    /// `register`
    Register,
}

// From 6.7.2

/// Type specifier
///
/// (C11 6.7.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeSpecifier {
    /// `void`
    Void,
    /// `char`
    Char,
    /// `short`
    Short,
    /// `int`
    Int,
    /// `long`
    Long,
    /// `float`
    Float,
    /// `double`
    Double,
    /// `signed`
    ///
    /// `__signed`, `__signed__` (GNU extension)
    Signed,
    /// `unsigned`
    Unsigned,
    /// `_Bool`
    Bool,
    /// `_Complex`
    ///
    /// `__complex`, `__complex__` (GNU extension)
    Complex,
    /// `_Atomic(typename)`
    Atomic(Node<TypeName>),
    /// `struct identifier { … }`
    ///
    /// `union identifier { … }`
    Struct(Node<StructType>),
    /// `enum identifier { … }`
    Enum(Node<EnumType>),
    /// Name of a previously defined type
    TypedefName(Node<Identifier>),
    /// Specifies type of another type or expression
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Typeof.html)
    TypeOf(Node<TypeOf>),
    /// Floating point types with guaranteed width and representation
    ///
    /// `_Float16`, `_Float32`, `_Float64`, `_Float128`
    ///
    /// `_Float16x`, `_Float32x`, `_Float64x`, `_Float128x`
    ///
    /// `_Decimal16`, `_Decimal32`, `_Decimal64`, `_Decimal128`
    ///
    /// `_Decimal16x`, `_Decimal32x`, `_Decimal64x`, `_Decimal128x`
    ///
    /// [ISO/IEC TS 18661-3:2015](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1945.pdf)
    TS18661Float(TS18661FloatType),
}

/// Floating point type with guaranteed width and format
///
/// [ISO/IEC TS 18661-3:2015](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1945.pdf)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TS18661FloatType {
    pub format: TS18661FloatFormat,
    pub width: usize,
}

/// Floating point formats
///
/// [ISO/IEC TS 18661-3:2015](http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1945.pdf)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TS18661FloatFormat {
    BinaryInterchange,
    BinaryExtended,
    DecimalInterchange,
    DecimalExtended,
}

// From 6.7.2.1

/// Structure or union type specifier
///
/// (C11 6.7.2.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    pub kind: Node<StructKind>,
    pub identifier: Option<Node<Identifier>>,
    /// List of structure of union members, when present.
    ///
    /// A [GNU extension](https://gcc.gnu.org/onlinedocs/gcc-8.1.0/gcc/Empty-Structures.html) allows the list to be empty.
    pub declarations: Option<Vec<Node<StructDeclaration>>>,
    pub extensions: Vec<Node<Extension>>,
}

/// The only difference between a `struct` and a `union`
///
/// (C11 6.7.2.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StructKind {
    Struct,
    Union,
}

/// Single declaration in a struct or a union
///
/// (C11 6.7.2.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StructDeclaration {
    Field(Node<StructField>),
    StaticAssert(Node<StaticAssert>),
}

/// Struct field declaration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    pub specifiers: Vec<Node<SpecifierQualifier>>,
    pub declarators: Vec<Node<StructDeclarator>>,
    pub extensions: Vec<Node<Extension>>,
}

/// Type and qualifiers for a struct declaration
///
/// C11 also uses this type in a few other places.
///
/// (C11 6.7.2.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpecifierQualifier {
    TypeSpecifier(Node<TypeSpecifier>),
    TypeQualifier(Node<TypeQualifier>),
}

/// Field declarator for a struct or a union
///
/// (C11 6.7.2.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDeclarator {
    pub declarator: Option<Node<Declarator>>,
    pub bit_width: Option<Box<Node<Expression>>>,
}

// From 6.7.2.2

/// Enumeration type specifier
///
/// (C11 6.7.2.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    pub identifier: Option<Node<Identifier>>,
    pub enumerators: Vec<Node<Enumerator>>,
}

/// Single constant inside a `enum` definition
///
/// (C11 6.7.2.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Enumerator {
    pub identifier: Node<Identifier>,
    pub expression: Option<Box<Node<Expression>>>,
}

// From 6.7.3

/// Type qualifier
///
/// (C11 6.7.3)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeQualifier {
    /// `const`
    ///
    /// `__const` (GNU extension)
    Const,
    /// `restrict`
    ///
    /// `__restrict`, `__restrict__` (GNU extension)
    Restrict,
    /// `volatile`
    ///
    /// `__volatile`, `__volatile__` (GNU extension)
    Volatile,
    /// '_Nonnull' (Clang extension)
    ///
    /// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html)
    Nonnull,
    /// '_Null_unspecified' (Clang extension)
    ///
    /// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html)
    NullUnspecified,
    /// '_Nullable' (Clang extension)
    ///
    /// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html)
    Nullable,
    /// `_Atomic`
    Atomic,
    /// Calling convention
    ///
    /// [MSVC extension](https://docs.microsoft.com/en-us/cpp/cpp/calling-conventions)
    CallingConvention(CallingConvention),
}

// From 6.7.4

/// Function specifier
///
/// (C11 6.7.4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionSpecifier {
    /// `inline`
    ///
    /// `__inline`, `__inline__` (GNU extension)
    Inline,
    /// `_Noreturn`
    Noreturn,
    /// `__forceinline` [MSVC extension](https://docs.microsoft.com/en-us/cpp/cpp/inline-functions-cpp)
    ForceInline,
}

// From 6.7.5

/// Alignment specifier
///
/// (C11 6.7.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AlignmentSpecifier {
    /// `_Alignas(typename)`
    Type(Node<TypeName>),
    /// `_Alignas(expression)`
    Constant(Box<Node<Expression>>),
    /// [MSVC extension](https://docs.microsoft.com/en-us/cpp/cpp/unaligned?view=vs-2019) `__unaligned`
    Unaligned,
}

// From 6.7.6 Declarators

/// Single item in a declaration
///
/// Represents both normal and abstract declarators.
///
/// (C11 6.7.6, 6.7.7)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Declarator {
    /// What is being declared
    pub kind: Node<DeclaratorKind>,
    /// Contains pointer, array and function declarator elements
    pub derived: Vec<Node<DerivedDeclarator>>,
    /// Vendor-specific extensions
    pub extensions: Vec<Node<Extension>>,
}

/// Name of a declarator
///
/// (C11 6.7.6, 6.7.7)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclaratorKind {
    /// Unnamed declarator
    ///
    /// E.g. part of a function prototype without parameter names.
    Abstract,
    /// Named declarator
    ///
    /// E.g. a variable or a named function parameter.
    Identifier(Node<Identifier>),
    /// Nested declarator
    ///
    /// Any group of parenthesis inside a declarator. E.g. pointer to
    /// a function.
    Declarator(Box<Node<Declarator>>),
}

/// Modifies declarator type
///
/// (C11 6.7.6)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DerivedDeclarator {
    /// `* qualifiers …`
    Pointer(Vec<Node<PointerQualifier>>),
    /// `… []`
    Array(Node<ArrayDeclarator>),
    /// `… ( parameters )`
    Function(Node<FunctionDeclarator>),
    /// `… ( identifiers )`
    KRFunction(Vec<Node<Identifier>>),
}

/// Array part of a declarator
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArrayDeclarator {
    pub qualifiers: Vec<Node<TypeQualifier>>,
    pub size: ArraySize,
}

/// Function parameter part of a declarator
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDeclarator {
    pub parameters: Vec<Node<ParameterDeclaration>>,
    pub ellipsis: Ellipsis,
}

/// List of qualifiers that can follow a `*` in a declaration
///
/// (C11 6.7.6.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointerQualifier {
    TypeQualifier(Node<TypeQualifier>),
    Extension(Vec<Node<Extension>>),
}

/// Size of an array in a declaration
///
/// (C11 6.7.6.2)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArraySize {
    /// `[]`
    Unknown,
    /// `[*]`
    VariableUnknown,
    /// `[10]`
    VariableExpression(Box<Node<Expression>>),
    /// `[static 10]`
    StaticExpression(Box<Node<Expression>>),
}

/// Complete parameter declaration in a function prototype or declaration
///
/// This is so called "new-style" or "C89" parameter declaration that
/// follows in parenthesis after a function name. "Old-style" or "K&R"
/// function parameter declarations are collected in the
/// `FunctionDefinition::declarations` field.
///
/// (C11 6.7.6.3)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParameterDeclaration {
    pub specifiers: Vec<Node<DeclarationSpecifier>>,
    pub declarator: Option<Node<Declarator>>,
    pub extensions: Vec<Node<Extension>>,
}

/// Whether function signature ends with a `...`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ellipsis {
    Some,
    None,
}

// From 6.7.7 Type names

/// References to types outside of declarations
///
/// Type names contain only abstract declarators.
///
/// (C11 6.7.7)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeName {
    pub specifiers: Vec<Node<SpecifierQualifier>>,
    pub declarator: Option<Node<Declarator>>,
}

// From 6.7.9 Initialization

/// Value that is assigned immediately in a declaration
///
/// (C11 6.7.9)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Initializer {
    Expression(Box<Node<Expression>>),
    List(Vec<Node<InitializerListItem>>),
}

/// Initializes one field or array element in a initializer list
///
/// (C11 6.7.9)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InitializerListItem {
    pub designation: Vec<Node<Designator>>,
    pub initializer: Box<Node<Initializer>>,
}

/// Single element of an designation in an initializer
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Designator {
    /// Array element
    ///
    /// `{ [expression] = … }`
    ///
    /// `{ [expression] … }` (obsolete GNU extension)
    Index(Node<Expression>),

    /// Struct or union member
    ///
    /// `{ .identifier = … }`
    ///
    /// `{ identifier: … }` (obsolete GNU extension)
    Member(Node<Identifier>),

    /// Range of array elements
    ///
    /// `{ [from ... to] … }`
    /// ([GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Designated-Inits.html#Designated-Inits))
    Range(Node<RangeDesignator>),
}

/// Range array designator in an initializer
///
/// `[from ... to]`
///
/// ([GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Designated-Inits.html#Designated-Inits))
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RangeDesignator {
    pub from: Node<Expression>,
    pub to: Node<Expression>,
}

// From 6.7.10 Static assertions

/// Static assertion
///
/// (C11 6.7.10)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StaticAssert {
    pub expression: Box<Node<Expression>>,
    pub message: Node<StringLiteral>,
}

// From 6.8 Statement

/// Element of a function body
///
/// (C11 6.8)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    Labeled(Node<LabeledStatement>),
    Compound(Vec<Node<BlockItem>>),
    Expression(Option<Box<Node<Expression>>>),
    If(Node<IfStatement>),
    Switch(Node<SwitchStatement>),
    While(Node<WhileStatement>),
    DoWhile(Node<DoWhileStatement>),
    For(Node<ForStatement>),
    Goto(Node<Identifier>),
    Continue,
    Break,
    Return(Option<Box<Node<Expression>>>),
    /// Vendor specific inline assembly extensions
    Asm(Node<AsmStatement>),
}

/// Labeled statement
///
/// (C11 6.8.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabeledStatement {
    pub label: Node<Label>,
    pub statement: Box<Node<Statement>>,
}

/// If statement
///
/// (C11 6.8.4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IfStatement {
    pub condition: Box<Node<Expression>>,
    pub then_statement: Box<Node<Statement>>,
    pub else_statement: Option<Box<Node<Statement>>>,
}

/// Switch statement
///
/// (C11 6.8.4)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SwitchStatement {
    pub expression: Box<Node<Expression>>,
    pub statement: Box<Node<Statement>>,
}

/// While statement
///
/// (C11 6.8.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WhileStatement {
    pub expression: Box<Node<Expression>>,
    pub statement: Box<Node<Statement>>,
}

/// Do statement
///
/// (C11 6.8.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DoWhileStatement {
    pub statement: Box<Node<Statement>>,
    pub expression: Box<Node<Expression>>,
}

/// For statement
///
/// (C11 6.8.5)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForStatement {
    pub initializer: Node<ForInitializer>,
    pub condition: Option<Box<Node<Expression>>>,
    pub step: Option<Box<Node<Expression>>>,
    pub statement: Box<Node<Statement>>,
}

/// Statement labels for `goto` and `switch`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Label {
    /// Goto label
    ///
    /// `ident: …`
    Identifier(Node<Identifier>),
    /// Case in a `switch` statement
    ///
    /// `case 'a': …`
    Case(Box<Node<Expression>>),
    /// Default case in a `switch` statement
    ///
    /// `default: …`
    Default,
}

/// First element of a `for` statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ForInitializer {
    /// `for(; …)`
    Empty,
    /// `for(a = 1; …)`
    Expression(Box<Node<Expression>>),
    /// `for(int a = 1; …)`
    Declaration(Node<Declaration>),
    /// `for(_StaticAssert(…); …)`
    StaticAssert(Node<StaticAssert>),
}

// From 6.8.2

/// Element of a compound statement
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockItem {
    Declaration(Node<Declaration>),
    StaticAssert(Node<StaticAssert>),
    Statement(Node<Statement>),
}

// From 6.9 External definitions

/// Entire C source file after preprocessing
///
/// (C11 6.9)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TranslationUnit(pub Vec<Node<ExternalDeclaration>>);

/// Top-level elements of a C program
///
/// (C11 6.9)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExternalDeclaration {
    Declaration(Node<Declaration>),
    StaticAssert(Node<StaticAssert>),
    FunctionDefinition(Node<FunctionDefinition>),
    Directive(Node<Directive>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Directive {
    pub value: String,
}

/// Function definition
///
/// (C11 6.9.1)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDefinition {
    /// Return type of the function, possibly mixed with other specifiers
    pub specifiers: Vec<Node<DeclarationSpecifier>>,
    /// Contains function name and parameter list
    pub declarator: Node<Declarator>,
    /// K&R style parameter type definitions (C11 6.9.1 §6)
    pub declarations: Vec<Node<Declaration>>,
    /// Body of the function.
    pub statement: Node<Statement>,
}

// Syntax extensions

/// Extended vendor-specific syntax that does not fit elsewhere
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Extension {
    /// Attributes
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Attribute-Syntax.html)
    Attribute(Attribute),
    /// Assembler name for an object
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Asm-Labels.html)
    AsmLabel(Node<StringLiteral>),
    /// Platform availability
    ///
    /// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html#availability)
    AvailabilityAttribute(Node<AvailabilityAttribute>),
    /// Source-code annotation language (SAL) parameter attribute
    ///
    /// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
    SalParamAttribute(SalParamAttribute),
    /// Source-code annotation language (SAL) function attribute
    ///
    /// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
    SalFunctionAttribute(SalFunctionAttribute),
    /// Source-code annotation language (SAL) field attribute
    ///
    /// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
    SalFieldAttribute(SalFieldAttribute),
    /// Source-code annotation language (SAL) struct attribute
    ///
    /// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
    SalStructAttribute(SalStructAttribute),
}

/// Calling convention
///
/// [MSVC extension](https://docs.microsoft.com/en-us/cpp/cpp/calling-conventions)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    // __cdecl
    Cdecl,
    // __fastcall,
    Fastcall,
    // __stdcall
    Stdcall,
    // __clrcall
    Clrcall,
    // __thiscall
    Thiscall,
    // __vectorcall
    Vectorcall,
}

/// Attributes
///
/// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Attribute-Syntax.html)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    pub name: Node<String>,
    pub arguments: Vec<Node<Expression>>,
}

/// Source-code annotation language (SAL) struct attribute
///
/// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SalStructAttribute {
    StructSizeBytes(Node<Expression>),
}

/// Source-code annotation language (SAL) field attribute
///
/// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SalFieldAttribute {
    FieldRange(Node<Expression>, Node<Expression>),
    FieldZ,
    Satisfies(Node<Expression>),
    FieldSize(Node<Expression>),
    FieldSizeOpt(Node<Expression>),
    FieldSizeBytes(Node<Expression>),
    FieldSizeBytesOpt(Node<Expression>),
    FieldSizePart(Node<Expression>, Node<Expression>),
    FieldSizePartOpt(Node<Expression>, Node<Expression>),
    FieldSizeBytesPart(Node<Expression>, Node<Expression>),
    FieldSizeBytesPartOpt(Node<Expression>, Node<Expression>),
    FieldSizeFull(Node<Expression>),
    FieldSizeFullOpt(Node<Expression>),
    FieldSizeBytesFull(Node<Expression>),
    FieldSizeBytesFullOpt(Node<Expression>),
}

/// Source-code annotation language (SAL) function attribute
///
/// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SalFunctionAttribute {
    /// _Success_(<expr>)
    Success(Node<Expression>),
    /// _Return_type_success_(<expr>)
    ReturnTypeSuccess(Node<Expression>),
    /// _Check_return_
    CheckReturn,
    /// _Null_terminated_
    NullTerminated,
    NullNullTerminated,
    MustInspectResult,
    UseDeclAnnotations,
    MaybeRaisesSehException,
    RaisesSehException,
    When(String),
    // At(Node<Expression>),
}

/// Source-code annotation language (SAL) parameter attribute
///
/// [MSVC extension](https://docs.microsoft.com/en-us/cpp/code-quality/understanding-sal)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SalParamAttribute {
    /// _In_
    In,
    /// _Out_
    Out,
    OutPtr,
    OutPtrResultMaybeNull,
    OutPtrResultBytebuffer(Node<Expression>),
    InOut,
    InReads(Node<Expression>),
    InReadsOpt(Node<Expression>),
    InReadsBytes(Node<Expression>),
    InReadsBytesOpt(Node<Expression>),
    OutWrites(Node<Expression>),
    OutWritesOpt(Node<Expression>),
    OutWritesBytes(Node<Expression>),
    OutWritesBytesOpt(Node<Expression>),
    OutWritesTo(Node<Expression>, Node<Expression>),
    OutWritesBytesTo(Node<Expression>, Node<Expression>),
    InOutUpdates(Node<Expression>),
    InOutUpdatesOpt(Node<Expression>),
    InOutUpdatesBytes(Node<Expression>),
    InOutUpdatesBytesOpt(Node<Expression>),
    InOpt,
    OutOpt,
    OutPtrOpt,
    InOutOpt,
    NullTerminated,
    Reserved,
}

/// Platform availability attribute
///
/// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html#availability)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AvailabilityAttribute {
    pub platform: Node<Identifier>,
    pub clauses: Vec<Node<AvailabilityClause>>,
}

/// Platfrom availability attribute clause
///
/// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html#availability)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AvailabilityClause {
    Introduced(Node<AvailabilityVersion>),
    Deprecated(Node<AvailabilityVersion>),
    Obsoleted(Node<AvailabilityVersion>),
    Unavailable,
    Message(Node<StringLiteral>),
    Replacement(Node<StringLiteral>),
}

/// Platfrom version inside availability attribute
///
/// [Clang extension](https://clang.llvm.org/docs/AttributeReference.html#availability)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AvailabilityVersion {
    pub major: String,
    pub minor: Option<String>,
    pub subminor: Option<String>,
}

/// Inline assembler
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AsmStatement {
    /// Basic asm statement with just source code
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Basic-Asm.html)
    GnuBasic(Node<StringLiteral>),

    /// Extended statement that has access to C variables
    ///
    /// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html)
    GnuExtended(GnuExtendedAsmStatement),
}

/// Extended statement that has access to C variables
///
/// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GnuExtendedAsmStatement {
    pub qualifier: Option<Node<TypeQualifier>>,
    pub template: Node<StringLiteral>,
    pub outputs: Vec<Node<GnuAsmOperand>>,
    pub inputs: Vec<Node<GnuAsmOperand>>,
    pub clobbers: Vec<Node<StringLiteral>>,
}

/// Single input or output operand specifier for GNU extended asm statement
///
/// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Extended-Asm.html#Output-Operands)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GnuAsmOperand {
    pub symbolic_name: Option<Node<Identifier>>,
    pub constraints: Node<StringLiteral>,
    pub variable_name: Node<Expression>,
}

/// Type of an expression or type
///
/// [GNU extension](https://gcc.gnu.org/onlinedocs/gcc/Typeof.html)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeOf {
    Expression(Node<Expression>),
    Type(Node<TypeName>),
}