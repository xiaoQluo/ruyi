/**
 * Core type representations for the Ruyi gradual type system.
 *
 * Defines all type variants including primitives, nullable, function,
 * generic, trait, and dynamic types. Implements subtyping, consistency,
 * and least-upper-bound (lub) per the Ruyi spec (Sections 8-11).
 *
 * @author Ruyi Team
 * @date 2026-05-01
 */
use std::collections::HashMap;
use std::fmt;

/// Core type representation for the Ruyi type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// 64-bit signed integer (`int`)
    Int,
    /// 64-bit IEEE 754 floating point (`float`)
    Float,
    /// Boolean (`bool`)
    Bool,
    /// UTF-8 string (`string`)
    String,
    /// Null type (only value: null)
    Null,
    /// Void type (no return value)
    Void,
    /// Never type (bottom type, unreachable code)
    Never,
    /// BigInt type (arbitrary precision integer)
    BigInt,
    /// Nullable type `T?` = T | null
    Nullable(Box<Type>),
    /// Array type `Array<T>`
    Array(Box<Type>),
    /// Object type with named fields `{ k1: T1, k2: T2, ... }`
    Object(Vec<ObjectField>),
    /// Function type `fn(T1, T2, ...) -> R`
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
    /// Named type reference (e.g., class name, type alias)
    Named(String),
    /// Generic type instantiation `Name<T1, T2, ...>`
    Generic { base: String, args: Vec<Type> },
    /// Type parameter (e.g., `T` in `fn identity<T>(x: T): T`)
    TypeVar(TypeVar),
    /// Trait type `dyn TraitName`
    Trait(String),
    /// Dynamic type (`dyn`) — runtime checked
    Dynamic,
    /// Future type `Future<T>` — represents an async computation
    Future(Box<Type>),
    /// Error type — used for error recovery to prevent cascading errors
    Error,
}

/// Object field for structural object types.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
}

/// Type variable for generic inference.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    pub id: u32,
    pub name: String,
}

/// Type constraint for generic inference.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeConstraint {
    /// Type variable must implement the given trait.
    Implements {
        type_var: TypeVar,
        trait_name: String,
    },
    /// Two types must be equal.
    Equal(Type, Type),
    /// First type must be a subtype of the second.
    Subtype { sub: Type, sup: Type },
}

impl TypeVar {
    /// Creates a new type variable with the given id and name.
    pub fn new(id: u32, name: String) -> Self {
        Self { id, name }
    }
}

impl Type {
    /// Returns `true` if this type is the dynamic type.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Type::Dynamic)
    }

    /// Returns `true` if this type is the error recovery type.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Returns `true` if this type is the never (bottom) type.
    pub fn is_never(&self) -> bool {
        matches!(self, Type::Never)
    }

    /// Returns `true` if this type is nullable (T? or null or dyn).
    pub fn is_nullable(&self) -> bool {
        matches!(self, Type::Nullable(_) | Type::Null | Type::Dynamic)
    }

    /// Returns the inner type of a nullable, or `self` if not nullable.
    /// For `T?`, returns `T`. For non-nullable types, returns `self`.
    pub fn unwrap_nullable(&self) -> &Type {
        match self {
            Type::Nullable(inner) => inner,
            _ => self,
        }
    }

    /// Wraps this type in a nullable: `T` becomes `T?`.
    /// Per spec rule `(T?)? = T?`, nullable of nullable collapses.
    pub fn make_nullable(self) -> Type {
        match self {
            Type::Nullable(_) => self,
            Type::Null => Type::Null,
            Type::Dynamic => Type::Dynamic,
            other => Type::Nullable(Box::new(other)),
        }
    }

    /// Checks if `self` is a subtype of `other` per the Ruyi spec.
    ///
    /// Subtyping rules (Section 8.3):
    /// - Reflexive: T <: T
    /// - int <: float (widening)
    /// - T <: T? (nullable supertype)
    /// - Never <: T (bottom type)
    /// - dyn ~ T (consistency, not strict subtyping)
    /// - Object subtyping is structural
    /// - Function subtyping is contravariant in params, covariant in return
    /// - Array<T> <: Array<U> if T <: U (covariant)
    pub fn is_subtype_of(&self, other: &Type) -> bool {
        // Reflexive: T <: T
        if self == other {
            return true;
        }

        // Never <: T (bottom type is subtype of everything)
        if self.is_never() {
            return true;
        }

        // dyn is consistent with everything but not a strict subtype
        // For gradual typing, we treat dyn as compatible with everything
        if self.is_dynamic() || other.is_dynamic() {
            return true;
        }

        // Error type is compatible with everything (error recovery)
        if self.is_error() || other.is_error() {
            return true;
        }

        match (self, other) {
            // int <: float (widening coercion)
            (Type::Int, Type::Float) => true,

            // T <: T? (nullable supertype)
            (t, Type::Nullable(inner)) => t.is_subtype_of(inner) || t.is_subtype_of(&Type::Null),

            // Object subtyping (structural): { more fields } <: { fewer fields }
            // Per spec: { f1: T1, ..., fn: Tn, ... } <: { f1: U1, ..., fm: Um }
            // if m <= n and for each fi in the supertype: Ti <: Ui
            (Type::Object(self_fields), Type::Object(other_fields)) => {
                let self_map: HashMap<&str, &Type> = self_fields
                    .iter()
                    .map(|f| (f.name.as_str(), &f.ty))
                    .collect();

                for other_field in other_fields {
                    match self_map.get(other_field.name.as_str()) {
                        Some(self_ty) => {
                            if !self_ty.is_subtype_of(&other_field.ty) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
                true
            }

            // Function subtyping (contravariant params, covariant return)
            (
                Type::Function {
                    params: self_params,
                    return_type: self_ret,
                },
                Type::Function {
                    params: other_params,
                    return_type: other_ret,
                },
            ) => {
                // Contravariant in parameters
                if self_params.len() != other_params.len() {
                    return false;
                }
                // Params are contravariant: other_param <: self_param
                for (sp, op) in self_params.iter().zip(other_params.iter()) {
                    if !op.is_subtype_of(sp) {
                        return false;
                    }
                }
                // Covariant in return type: self_ret <: other_ret
                self_ret.is_subtype_of(other_ret)
            }

            // Array subtyping (covariant)
            (Type::Array(self_elem), Type::Array(other_elem)) => {
                self_elem.is_subtype_of(other_elem)
            }

            // Generic type subtyping
            (
                Type::Generic {
                    base: self_base,
                    args: self_args,
                },
                Type::Generic {
                    base: other_base,
                    args: other_args,
                },
            ) => {
                if self_base != other_base {
                    return false;
                }
                if self_args.len() != other_args.len() {
                    return false;
                }
                // For now, require exact match on generic args
                // (variance depends on the type constructor)
                self_args
                    .iter()
                    .zip(other_args.iter())
                    .all(|(s, o)| s.is_subtype_of(o))
            }

            // Trait subtyping: named type can be subtype of trait
            (Type::Named(name), Type::Trait(trait_name)) => {
                // In a real implementation, we'd check if `name` implements `trait_name`
                // For now, we allow this for gradual typing
                let _ = (name, trait_name);
                true
            }

            // Generic type <: trait
            (Type::Generic { base, .. }, Type::Trait(trait_name)) => {
                let _ = (base, trait_name);
                true
            }

            _ => false,
        }
    }

    /// Checks gradual typing consistency (`~`).
    ///
    /// Two types are consistent if one is a subtype of the other,
    /// or if either is `dyn`.
    pub fn is_consistent_with(&self, other: &Type) -> bool {
        if self.is_dynamic() || other.is_dynamic() {
            return true;
        }
        if self.is_error() || other.is_error() {
            return true;
        }
        self.is_subtype_of(other) || other.is_subtype_of(self)
    }

    /// Computes the least upper bound (lub) of two types.
    ///
    /// Per spec Section 8.2.2:
    /// - lub(T, T) = T
    /// - lub(int, float) = float
    /// - lub(T, dyn) = dyn
    /// - lub(T, U) where T <: U = U
    /// - lub(T, U) where U <: T = T
    /// - lub(T, U) where unrelated = dyn
    pub fn least_upper_bound(&self, other: &Type) -> Type {
        // Same type
        if self == other {
            return self.clone();
        }

        // Error recovery
        if self.is_error() {
            return other.clone();
        }
        if other.is_error() {
            return self.clone();
        }

        // dyn absorbs everything
        if self.is_dynamic() || other.is_dynamic() {
            return Type::Dynamic;
        }

        // Never is identity for lub
        if self.is_never() {
            return other.clone();
        }
        if other.is_never() {
            return self.clone();
        }

        // int + float = float
        if matches!(
            (self, other),
            (Type::Int, Type::Float) | (Type::Float, Type::Int)
        ) {
            return Type::Float;
        }

        // Subtyping: lub(T, U) = U if T <: U
        if self.is_subtype_of(other) {
            return other.clone();
        }
        if other.is_subtype_of(self) {
            return self.clone();
        }

        // Nullable types: lub(T?, U?) = lub(T, U)?
        match (self, other) {
            (Type::Nullable(t1), Type::Nullable(t2)) => t1.least_upper_bound(t2).make_nullable(),
            (Type::Nullable(t1), t2) => t1.least_upper_bound(t2).make_nullable(),
            (t1, Type::Nullable(t2)) => t1.least_upper_bound(t2).make_nullable(),
            _ => Type::Dynamic,
        }
    }

    /// Removes the nullable wrapper from a type.
    /// `T?` becomes `T`, non-nullable types remain unchanged.
    pub fn non_null(&self) -> Type {
        match self {
            Type::Nullable(inner) => *inner.clone(),
            Type::Null => Type::Never, // non_null(null) = never (unreachable)
            other => other.clone(),
        }
    }

    /// Converts a TypeAnnotation from the parser into a Type.
    pub fn from_annotation(annotation: &crate::parser::ast::TypeAnnotation) -> Type {
        let result = match annotation {
            crate::parser::ast::TypeAnnotation::Identifier(name) => match name.as_str() {
                "int" => Type::Int,
                "float" => Type::Float,
                "bool" => Type::Bool,
                "string" => Type::String,
                "null" => Type::Null,
                "void" => Type::Void,
                "never" => Type::Never,
                "bigint" => Type::BigInt,
                "dyn" => Type::Dynamic,
                _ => Type::Named(name.clone()),
            },
            crate::parser::ast::TypeAnnotation::Nullable(inner) => {
                Type::from_annotation(inner).make_nullable()
            }
            crate::parser::ast::TypeAnnotation::Function {
                params,
                return_type,
            } => Type::Function {
                params: params.iter().map(Type::from_annotation).collect(),
                return_type: Box::new(Type::from_annotation(return_type)),
            },
            crate::parser::ast::TypeAnnotation::Generic { base, args } => Type::Generic {
                base: base.clone(),
                args: args.iter().map(Type::from_annotation).collect(),
            },
            crate::parser::ast::TypeAnnotation::Object(fields) => Type::Object(
                fields
                    .iter()
                    .map(|f| ObjectField {
                        name: f.name.clone(),
                        ty: Type::from_annotation(&f.ty),
                        optional: false,
                    })
                    .collect(),
            ),
            crate::parser::ast::TypeAnnotation::Array(elem) => {
                Type::Array(Box::new(Type::from_annotation(elem)))
            }
            crate::parser::ast::TypeAnnotation::Tuple(types) => {
                // Tuples are represented as arrays of union types for now
                // In a full implementation, tuples would be a separate type
                if types.is_empty() {
                    Type::Array(Box::new(Type::Dynamic))
                } else if types.len() == 1 {
                    Type::Array(Box::new(Type::from_annotation(&types[0])))
                } else {
                    // Tuple with multiple element types -> Array<dyn> for now
                    Type::Array(Box::new(Type::Dynamic))
                }
            }
            crate::parser::ast::TypeAnnotation::Dyn(inner) => match inner.as_ref() {
                crate::parser::ast::TypeAnnotation::Identifier(name) => Type::Trait(name.clone()),
                crate::parser::ast::TypeAnnotation::Generic { base, .. } => {
                    Type::Trait(base.clone())
                }
                _ => Type::Dynamic,
            },
        };
        if let Type::Generic { base, args } = &result {
            if base == "Future" && args.len() == 1 {
                return Type::Future(Box::new(args[0].clone()));
            }
        }
        result
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Null => write!(f, "null"),
            Type::Void => write!(f, "void"),
            Type::Never => write!(f, "never"),
            Type::BigInt => write!(f, "bigint"),
            Type::Nullable(inner) => write!(f, "{}?", inner),
            Type::Array(elem) => write!(f, "Array<{}>", elem),
            Type::Object(fields) => {
                write!(f, "{{ ")?;
                let parts: Vec<String> = fields
                    .iter()
                    .map(|f| {
                        if f.optional {
                            format!("{}?: {}", f.name, f.ty)
                        } else {
                            format!("{}: {}", f.name, f.ty)
                        }
                    })
                    .collect();
                write!(f, "{} }}", parts.join(", "))
            }
            Type::Function {
                params,
                return_type,
            } => {
                let param_strs: Vec<String> = params.iter().map(|p| p.to_string()).collect();
                write!(f, "fn({}) -> {}", param_strs.join(", "), return_type)
            }
            Type::Named(name) => write!(f, "{}", name),
            Type::Generic { base, args } => {
                let arg_strs: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{}<{}>", base, arg_strs.join(", "))
            }
            Type::TypeVar(var) => write!(f, "{}", var.name),
            Type::Trait(name) => write!(f, "dyn {}", name),
            Type::Dynamic => write!(f, "dyn"),
            Type::Future(inner) => write!(f, "Future<{}>", inner),
            Type::Error => write!(f, "<error>"),
        }
    }
}

impl fmt::Display for TypeConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeConstraint::Implements {
                type_var,
                trait_name,
            } => {
                write!(f, "{}: {}", type_var.name, trait_name)
            }
            TypeConstraint::Equal(t1, t2) => write!(f, "{} = {}", t1, t2),
            TypeConstraint::Subtype { sub, sup } => write!(f, "{} <: {}", sub, sup),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtype_reflexive() {
        assert!(Type::Int.is_subtype_of(&Type::Int));
        assert!(Type::String.is_subtype_of(&Type::String));
        assert!(Type::Dynamic.is_subtype_of(&Type::Dynamic));
    }

    #[test]
    fn test_subtype_int_to_float() {
        assert!(Type::Int.is_subtype_of(&Type::Float));
        assert!(!Type::Float.is_subtype_of(&Type::Int));
    }

    #[test]
    fn test_subtype_nullable() {
        assert!(Type::Int.is_subtype_of(&Type::Nullable(Box::new(Type::Int))));
        assert!(Type::Null.is_subtype_of(&Type::Nullable(Box::new(Type::Int))));
        assert!(Type::String.is_subtype_of(&Type::Nullable(Box::new(Type::String))));
    }

    #[test]
    fn test_subtype_never() {
        assert!(Type::Never.is_subtype_of(&Type::Int));
        assert!(Type::Never.is_subtype_of(&Type::String));
        assert!(Type::Never.is_subtype_of(&Type::Dynamic));
    }

    #[test]
    fn test_subtype_dynamic() {
        assert!(Type::Int.is_subtype_of(&Type::Dynamic));
        assert!(Type::Dynamic.is_subtype_of(&Type::Int));
        assert!(Type::Dynamic.is_subtype_of(&Type::Dynamic));
    }

    #[test]
    fn test_consistency() {
        assert!(Type::Int.is_consistent_with(&Type::Int));
        assert!(Type::Int.is_consistent_with(&Type::Dynamic));
        assert!(Type::Dynamic.is_consistent_with(&Type::String));
        assert!(!Type::Int.is_consistent_with(&Type::String));
    }

    #[test]
    fn test_lub_same_type() {
        assert_eq!(Type::Int.least_upper_bound(&Type::Int), Type::Int);
        assert_eq!(Type::String.least_upper_bound(&Type::String), Type::String);
    }

    #[test]
    fn test_lub_int_float() {
        assert_eq!(Type::Int.least_upper_bound(&Type::Float), Type::Float);
        assert_eq!(Type::Float.least_upper_bound(&Type::Int), Type::Float);
    }

    #[test]
    fn test_lub_with_dyn() {
        assert_eq!(Type::Int.least_upper_bound(&Type::Dynamic), Type::Dynamic);
        assert_eq!(
            Type::Dynamic.least_upper_bound(&Type::String),
            Type::Dynamic
        );
    }

    #[test]
    fn test_lub_unrelated() {
        assert_eq!(Type::Int.least_upper_bound(&Type::String), Type::Dynamic);
    }

    #[test]
    fn test_nullable_collapse() {
        // T?? = T?
        let inner = Type::Nullable(Box::new(Type::Int));
        let result = inner.make_nullable();
        assert_eq!(result, Type::Nullable(Box::new(Type::Int)));
    }

    #[test]
    fn test_non_null() {
        assert_eq!(Type::Nullable(Box::new(Type::Int)).non_null(), Type::Int);
        assert_eq!(Type::Int.non_null(), Type::Int);
        assert_eq!(Type::Null.non_null(), Type::Never);
    }

    #[test]
    fn test_display() {
        assert_eq!(Type::Int.to_string(), "int");
        assert_eq!(Type::Nullable(Box::new(Type::Int)).to_string(), "int?");
        assert_eq!(
            Type::Array(Box::new(Type::String)).to_string(),
            "Array<string>"
        );
        assert_eq!(Type::Dynamic.to_string(), "dyn");
    }

    #[test]
    fn test_from_annotation_primitive() {
        assert_eq!(
            Type::from_annotation(&crate::parser::ast::TypeAnnotation::Identifier(
                "int".into()
            )),
            Type::Int
        );
        assert_eq!(
            Type::from_annotation(&crate::parser::ast::TypeAnnotation::Identifier(
                "float".into()
            )),
            Type::Float
        );
        assert_eq!(
            Type::from_annotation(&crate::parser::ast::TypeAnnotation::Identifier(
                "dyn".into()
            )),
            Type::Dynamic
        );
    }

    #[test]
    fn test_from_annotation_nullable() {
        let inner = crate::parser::ast::TypeAnnotation::Identifier("int".into());
        let nullable = crate::parser::ast::TypeAnnotation::Nullable(Box::new(inner));
        assert_eq!(
            Type::from_annotation(&nullable),
            Type::Nullable(Box::new(Type::Int))
        );
    }

    #[test]
    fn test_from_annotation_function() {
        let fn_type = crate::parser::ast::TypeAnnotation::Function {
            params: vec![crate::parser::ast::TypeAnnotation::Identifier("int".into())],
            return_type: Box::new(crate::parser::ast::TypeAnnotation::Identifier(
                "string".into(),
            )),
        };
        let result = Type::from_annotation(&fn_type);
        assert_eq!(
            result,
            Type::Function {
                params: vec![Type::Int],
                return_type: Box::new(Type::String),
            }
        );
    }

    #[test]
    fn test_from_annotation_generic() {
        let gen_type = crate::parser::ast::TypeAnnotation::Generic {
            base: "Array".into(),
            args: vec![crate::parser::ast::TypeAnnotation::Identifier("int".into())],
        };
        assert_eq!(
            Type::from_annotation(&gen_type),
            Type::Generic {
                base: "Array".into(),
                args: vec![Type::Int],
            }
        );
    }
}
