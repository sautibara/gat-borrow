use std::{borrow::Borrow, ops::Deref};

pub use gat_borrow_derive::{Reborrow, derive_reborrow};

// for macros
use crate as gat_borrow;

pub trait ReborrowType<'a>: 'a {
    type Reborrow<'b>;
}

pub trait ReborrowMethods<'a>: ReborrowType<'a> {
    fn reborrow<'b>(self) -> Self::Reborrow<'b>
    where
        'a: 'b;

    fn reborrow_ref<'b>(&'b self) -> &'b Self::Reborrow<'b>
    where
        'a: 'b;
}

pub trait Reborrow<'a>:
    for<'b, 'c> ReborrowMethods<
        'a,
        Reborrow<'b>: ReborrowMethods<'b, Reborrow<'c>: Into<Self::Reborrow<'c>>>,
    >
{
}

impl<'a, T> Reborrow<'a> for T
where
    T: ReborrowMethods<'a>,
    // We want the reborrow to be the same as [`Self`] aside from lifetimes, but we can't just use
    //   [`Into`] or some other trait here; [`Self`] has a lifetime that we can't change - we
    //   can't just write a `Self::Reborrow<'b>: Into<Self<'b>>` bound for example.
    // Instead, we keep going down the chain in case users of the trait need to use [`Reborrow`]
    //   on the reborrows too. This isn't as good as saying that the types are the same (other
    //   traits that Self implements won't be included), but it's better than nothing.
    // Also, if users of the trait need reborrows to implement traits, they could just add those
    //   bounds in a where clause or something.
    for<'b> T::Reborrow<'b>: ReborrowMethods<'b>,
    // This ensures that the reborrow's [`Reborrow`] implementation effectively uses the same
    //   types. We can do this here because the reborrow type can be given an arbitrary lifetime
    //   now, so we don't run into the same problem as earlier :).
    //   (We're using [`Into`] to signify that the types should be equal)
    for<'b, 'c> <T::Reborrow<'b> as ReborrowType<'b>>::Reborrow<'c>: Into<T::Reborrow<'c>>,
{
}

derive_reborrow! {
    #[allow(type_alias_bounds)]
    type Ref<'a, T: 'static + ?Sized> = &'a T;
}

pub trait IntoOwnedImpl<'a>: ReborrowMethods<'a> + Clone {
    type Owned: for<'b> ToRef<'b, Self::Reborrow<'b>>;

    #[inline]
    fn into_owned(self) -> Self::Owned {
        self.to_own()
    }

    #[inline]
    fn to_own(&self) -> Self::Owned {
        self.clone().into_owned()
    }
}

pub trait IntoOwned<'a>:
    for<'b> IntoOwnedImpl<'a, Reborrow<'b>: IntoOwnedImpl<'b, Owned: Into<Self::Owned>>>
{
}

impl<'a, T> IntoOwned<'a> for T
where
    // These closely mirror the same bounds on [`Reborrow`] above, for similar reasons.
    T: IntoOwnedImpl<'a>,
    for<'b> Self::Reborrow<'b>: IntoOwnedImpl<'b> + Clone,
    for<'b> <Self::Reborrow<'b> as IntoOwnedImpl<'b>>::Owned: Into<Self::Owned>,
{
}

impl<'a, T: ToOwned + 'static + ?Sized> IntoOwnedImpl<'a> for &'a T {
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

pub enum Boo<'a, R: IntoOwnedImpl<'a>> {
    Borrowed(R),
    Owned(R::Owned),
}

impl<'a, R: IntoOwnedImpl<'a>> Boo<'a, R> {
    pub fn into_owned(self) -> R::Owned {
        match self {
            Self::Borrowed(reference) => reference.into_owned(),
            Self::Owned(owned) => owned,
        }
    }

    pub fn to_mut(&mut self) -> &mut R::Owned {
        match self {
            Self::Borrowed(reference) => {
                *self = Self::Owned(reference.to_own());
                match self {
                    Self::Borrowed(_) => unreachable!(),
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
            Self::Borrowed(reference) => BooRef::Reference(reference.reborrow_ref()),
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

    /// Returns `true` if the boo is [`Borrowed`].
    ///
    /// [`Borrowed`]: Boo::Borrowed
    #[must_use]
    pub const fn is_borrowed(&self) -> bool {
        matches!(self, Self::Borrowed(..))
    }

    /// Returns `true` if the boo is [`Owned`].
    ///
    /// [`Owned`]: Boo::Owned
    #[must_use]
    pub const fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(..))
    }
}

impl<'b, 'a: 'b, R: IntoOwnedImpl<'a>> ToRef<'b, BooRef<'b, 'a, R>> for Boo<'a, R> {
    fn to_ref(&'b self) -> BooRef<'b, 'a, R> {
        self.to_ref()
    }
}

pub enum BooRef<'b, 'a: 'b, R: IntoOwnedImpl<'a>> {
    Reference(&'b R::Reborrow<'b>),
    Owned(R::Reborrow<'b>),
}

impl<'b, 'a: 'b, R: IntoOwnedImpl<'a>> Deref for BooRef<'b, 'a, R> {
    type Target = R::Reborrow<'b>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Reference(reference) => reference,
            Self::Owned(owned) => owned,
        }
    }
}

impl<'b, 'a: 'b, R: IntoOwnedImpl<'a>> BooRef<'b, 'a, R> {
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
