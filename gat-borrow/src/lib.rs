use std::{borrow::Borrow, ops::Deref};

pub use gat_borrow_derive::{Reborrow, derive_reborrow};

use crate as gat_borrow;

pub trait ReborrowType<'a>: 'a {
    type Reborrow<'b>;
}

pub trait Reborrow<'a>: ReborrowType<'a> {
    fn reborrow<'b>(self) -> Self::Reborrow<'b>
    where
        'a: 'b;

    fn reborrow_ref<'b>(&'b self) -> &'b Self::Reborrow<'b>
    where
        'a: 'b;
}

derive_reborrow! {
    #[allow(type_alias_bounds)]
    type Ref<'a, T: 'static + ?Sized> = &'a T;
}

pub trait IntoOwned<'a>: Reborrow<'a> {
    type Owned: for<'b> ToRef<'b, Self::Reborrow<'b>>;

    fn into_owned(self) -> Self::Owned;
}

impl<'a, T: ToOwned + 'static + ?Sized> IntoOwned<'a> for &'a T {
    type Owned = T::Owned;

    fn into_owned(self) -> Self::Owned {
        self.to_owned()
    }
}

pub trait ToRef<'a, Reference: 'a> {
    fn to_ref(&'a self) -> Reference;
}

impl<'a, O: Borrow<B> + 'static, B: ?Sized> ToRef<'a, &'a B> for O {
    fn to_ref(&'a self) -> &'a B {
        self.borrow()
    }
}

pub enum Boo<'a, R: IntoOwned<'a>> {
    Reference(R),
    Owned(R::Owned),
}

impl<'a, R: IntoOwned<'a>> Boo<'a, R> {
    pub fn into_owned(self) -> R::Owned {
        match self {
            Self::Reference(reference) => reference.into_owned(),
            Self::Owned(owned) => owned,
        }
    }

    pub fn to_mut(&mut self) -> &mut R::Owned
    where
        R: Clone,
    {
        match self {
            Self::Reference(reference) => {
                *self = Self::Owned(reference.clone().into_owned());
                match self {
                    Self::Reference(_) => unreachable!(),
                    Self::Owned(owned) => owned,
                }
            }
            Self::Owned(owned) => owned,
        }
    }

    pub fn to_ref<'b>(&'b self) -> BooRef<'b, 'a, R>
    where
        'a: 'b,
    {
        match self {
            Self::Reference(reference) => BooRef::Reference(reference.reborrow_ref()),
            Self::Owned(owned) => BooRef::Owned(owned.to_ref()),
        }
    }

    pub fn to_owned_ref<'b>(&'b self) -> R::Reborrow<'b>
    where
        'a: 'b,
        R::Reborrow<'b>: Clone,
    {
        self.to_ref().into_owned_ref()
    }

    /// Returns `true` if the boo is [`Reference`].
    ///
    /// [`Reference`]: Boo::Reference
    #[must_use]
    pub const fn is_reference(&self) -> bool {
        matches!(self, Self::Reference(..))
    }

    /// Returns `true` if the boo is [`Owned`].
    ///
    /// [`Owned`]: Boo::Owned
    #[must_use]
    pub const fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(..))
    }
}

impl<'b, 'a: 'b, R: IntoOwned<'a>> ToRef<'b, BooRef<'b, 'a, R>> for Boo<'a, R> {
    fn to_ref(&'b self) -> BooRef<'b, 'a, R> {
        self.to_ref()
    }
}

pub enum BooRef<'b, 'a: 'b, R: IntoOwned<'a>> {
    Reference(&'b R::Reborrow<'b>),
    Owned(R::Reborrow<'b>),
}

impl<'b, 'a: 'b, R: IntoOwned<'a>> Deref for BooRef<'b, 'a, R> {
    type Target = R::Reborrow<'b>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Reference(reference) => reference,
            Self::Owned(owned) => owned,
        }
    }
}

impl<'b, 'a: 'b, R: IntoOwned<'a>> BooRef<'b, 'a, R> {
    pub fn into_owned_ref(self) -> R::Reborrow<'b>
    where
        R::Reborrow<'b>: Clone,
    {
        match self {
            Self::Reference(reference) => reference.clone(),
            Self::Owned(owned) => owned,
        }
    }
}
