/**
 * Self-reference resolution for class fields.
 *
 * Implements spec requirement 3.6 (v0.5.7-p1-defects): class fields
 * MAY reference the enclosing class via `Self` only when the
 * reference is wrapped in an indirection container (`Box`,
 * `Option`, `List`, or any other generic type constructor). Bare
 * `Self` as a field type is rejected because it would create an
 * infinite-size type.
 *
 * The resolver is invoked from `typechecker::inference` after the
 * field annotation is converted to a `Type`. It walks the type
 * structurally:
 *
 *   - `Type::Self_` at the top level (no wrapper) → error.
 *   - `Type::Self_` nested inside `Box` / `Option` / `List` or any
 *     `Type::Generic` argument list → resolved to the enclosing
 *     class type, propagating the `is_indirect = true` context into
 *     the recursion so deeper nested `Self` references remain legal.
 *   - `Type::Nullable(Self_)` → also allowed (the `Option` indirection).
 *   - Any other shape → returned unchanged.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use crate::typechecker::types::Type;

/// Context for resolving `Self` at an element (field or method) position.
///
/// Tracks whether the current type position is allowed to contain a
/// bare `Self` reference. Field positions are by default `is_indirect =
/// false`; the resolver flips the flag to `true` whenever it descends
/// into a known indirection container (`Box`, `Option`, `List`) so that
/// `Self` references nested inside are accepted and resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementContext {
    /// When `true`, `Self` at this position is allowed and resolves to
    /// the enclosing class type. The top-level field annotation is
    /// called with `is_indirect = false`; the resolver propagates
    /// `is_indirect = true` into the arguments of `Box` / `Option` /
    /// `List` and any other `Generic` constructor.
    pub is_indirect: bool,
}

impl ElementContext {
    /// Construct a context that rejects bare `Self` (the default for
    /// the top-level field annotation).
    pub const fn direct() -> Self {
        Self { is_indirect: false }
    }

    /// Construct a context that accepts `Self` and resolves it to the
    /// enclosing class type. Used inside the indirection containers.
    pub const fn indirect() -> Self {
        Self { is_indirect: true }
    }
}

/// Returns `true` when `name` is one of the recognized indirection
/// containers that legitimize a `Self` reference nested inside.
fn is_indirection_container(name: &str) -> bool {
    matches!(name, "Box" | "Option" | "List")
}

/// Resolve `Self` occurrences inside `ann` against the enclosing class
/// type. The recursion descends into known indirection containers
/// (`Box`, `Option`, `List`) and any other `Type::Generic` so that
/// nested `Self` references in user-defined containers are also
/// resolved.
///
/// Returns `Err` with the bare-`Self` rejection message when `ann`
/// is `Type::Self_` at a position where `ctx.is_indirect == false`.
/// On success, returns the rewritten type (with every resolvable
/// `Self` substituted by `enclosing`).
pub fn resolve(ann: &Type, enclosing: &Type, ctx: ElementContext) -> Result<Type, String> {
    match ann {
        Type::Self_ => {
            if ctx.is_indirect {
                Ok(enclosing.clone())
            } else {
                Err("bare Self not allowed in field type".to_string())
            }
        }
        Type::Nullable(inner) => {
            // `Option<T>`-shaped: `T?` and `Option<T>` both treat the
            // inner `Self` as indirect.
            let resolved_inner = resolve(inner, enclosing, ElementContext::indirect())?;
            Ok(Type::Nullable(Box::new(resolved_inner)))
        }
        Type::Generic { base, args } if is_indirection_container(base) => {
            let resolved_args: Vec<Type> = args
                .iter()
                .map(|a| resolve(a, enclosing, ElementContext::indirect()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Generic {
                base: base.clone(),
                args: resolved_args,
            })
        }
        Type::Generic { base, args } => {
            // User-defined generic containers — descend so nested
            // `Self` references still resolve, but keep the container
            // shape intact.
            let resolved_args: Vec<Type> = args
                .iter()
                .map(|a| resolve(a, enclosing, ElementContext::indirect()))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Type::Generic {
                base: base.clone(),
                args: resolved_args,
            })
        }
        Type::Array(inner) => {
            let resolved_inner = resolve(inner, enclosing, ElementContext::indirect())?;
            Ok(Type::Array(Box::new(resolved_inner)))
        }
        Type::Function {
            params,
            return_type,
        } => Ok(Type::Function {
            params: params
                .iter()
                .map(|p| resolve(p, enclosing, ElementContext::indirect()))
                .collect::<Result<Vec<_>, _>>()?,
            return_type: Box::new(resolve(
                return_type,
                enclosing,
                ElementContext::indirect(),
            )?),
        }),
        Type::Object(fields) => {
            let mut resolved_fields = Vec::with_capacity(fields.len());
            for f in fields {
                let ty = resolve(&f.ty, enclosing, ElementContext::indirect())?;
                resolved_fields.push(crate::typechecker::types::ObjectField {
                    name: f.name.clone(),
                    ty,
                    optional: f.optional,
                });
            }
            Ok(Type::Object(resolved_fields))
        }
        Type::Future(inner) => {
            let resolved_inner = resolve(inner, enclosing, ElementContext::indirect())?;
            Ok(Type::Future(Box::new(resolved_inner)))
        }
        other => Ok(other.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> Type {
        Type::Named(name.to_string(), vec![])
    }

    #[test]
    fn bare_self_at_field_position_is_rejected() {
        let ann = Type::Self_;
        let enclosing = named("Node");
        let err = resolve(&ann, &enclosing, ElementContext::direct()).unwrap_err();
        assert!(err.contains("bare Self not allowed"));
    }

    #[test]
    fn self_inside_box_is_resolved_to_enclosing() {
        let ann = Type::Generic {
            base: "Box".to_string(),
            args: vec![Type::Self_],
        };
        let enclosing = named("Tree");
        let resolved = resolve(&ann, &enclosing, ElementContext::direct()).unwrap();
        assert_eq!(
            resolved,
            Type::Generic {
                base: "Box".to_string(),
                args: vec![named("Tree")],
            }
        );
    }

    #[test]
    fn self_inside_option_is_resolved_to_enclosing() {
        let ann = Type::Nullable(Box::new(Type::Self_));
        let enclosing = named("Node");
        let resolved = resolve(&ann, &enclosing, ElementContext::direct()).unwrap();
        assert_eq!(resolved, Type::Nullable(Box::new(named("Node"))));
    }

    #[test]
    fn self_inside_list_is_resolved_to_enclosing() {
        let ann = Type::Array(Box::new(Type::Self_));
        let enclosing = named("Tree");
        let resolved = resolve(&ann, &enclosing, ElementContext::direct()).unwrap();
        assert_eq!(resolved, Type::Array(Box::new(named("Tree"))));
    }

    #[test]
    fn non_self_type_is_returned_unchanged() {
        let ann = Type::Int;
        let enclosing = named("Node");
        let resolved = resolve(&ann, &enclosing, ElementContext::direct()).unwrap();
        assert_eq!(resolved, Type::Int);
    }
}
