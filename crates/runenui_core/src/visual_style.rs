//! Typed authored values for common M9 node-visual style properties.

use crate::{DropShadow, IdentifierError, Outline, SceneOpacity, TokenId};

macro_rules! define_visual_token_ref {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(TokenId);

        impl $name {
            #[must_use]
            pub const fn new(id: TokenId) -> Self {
                Self(id)
            }

            /// Validates a dynamic typed token reference.
            ///
            /// # Errors
            ///
            /// Returns [`IdentifierError`] when the identifier text is invalid.
            pub fn parse(id: impl Into<String>) -> Result<Self, IdentifierError> {
                TokenId::new(id).map(Self)
            }

            #[must_use]
            pub const fn id(&self) -> &TokenId {
                &self.0
            }

            #[must_use]
            pub const fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl From<TokenId> for $name {
            fn from(id: TokenId) -> Self {
                Self(id)
            }
        }
    };
}

define_visual_token_ref!(OutlineToken, "Typed node-outline-token reference.");
define_visual_token_ref!(ShadowToken, "Typed ordered drop-shadow-list-token reference.");
define_visual_token_ref!(OpacityToken, "Typed node-opacity-token reference.");

/// Literal-or-token authored value for the optional node outline.
#[derive(Clone, Debug, PartialEq)]
pub enum OutlineValue {
    Literal(Outline),
    Token(OutlineToken),
}

impl OutlineValue {
    #[must_use]
    pub const fn literal(value: Outline) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: OutlineToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<&Outline> {
        if let Self::Literal(value) = self {
            Some(value)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&OutlineToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<Outline> for OutlineValue {
    fn from(value: Outline) -> Self {
        Self::Literal(value)
    }
}

impl From<OutlineToken> for OutlineValue {
    fn from(value: OutlineToken) -> Self {
        Self::Token(value)
    }
}

/// Literal-or-token authored value for one complete ordered drop-shadow list.
///
/// A later cascade layer replaces the complete list rather than appending to a
/// lower-precedence list, preserving ordinary property-local style precedence.
#[derive(Clone, Debug, PartialEq)]
pub enum ShadowValue {
    Literal(Vec<DropShadow>),
    Token(ShadowToken),
}

impl ShadowValue {
    #[must_use]
    pub const fn literal(value: Vec<DropShadow>) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: ShadowToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<&[DropShadow]> {
        if let Self::Literal(value) = self {
            Some(value.as_slice())
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&ShadowToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<Vec<DropShadow>> for ShadowValue {
    fn from(value: Vec<DropShadow>) -> Self {
        Self::Literal(value)
    }
}

impl From<DropShadow> for ShadowValue {
    fn from(value: DropShadow) -> Self {
        Self::Literal(vec![value])
    }
}

impl From<ShadowToken> for ShadowValue {
    fn from(value: ShadowToken) -> Self {
        Self::Token(value)
    }
}

/// Literal-or-token authored value for effective node opacity.
#[derive(Clone, Debug, PartialEq)]
pub enum OpacityValue {
    Literal(SceneOpacity),
    Token(OpacityToken),
}

impl OpacityValue {
    #[must_use]
    pub const fn literal(value: SceneOpacity) -> Self {
        Self::Literal(value)
    }

    #[must_use]
    pub const fn token(token: OpacityToken) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn as_literal(&self) -> Option<SceneOpacity> {
        if let Self::Literal(value) = self {
            Some(*value)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn as_token(&self) -> Option<&OpacityToken> {
        if let Self::Token(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl From<SceneOpacity> for OpacityValue {
    fn from(value: SceneOpacity) -> Self {
        Self::Literal(value)
    }
}

impl From<OpacityToken> for OpacityValue {
    fn from(value: OpacityToken) -> Self {
        Self::Token(value)
    }
}
