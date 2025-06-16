use std::{borrow::Borrow, ops::Deref};

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

#[macro_export]
macro_rules! derive_reborrow {
    ($ident:ident $(, generics: [$($T:ident),*])?) => {
        impl<'a $(, $($T: 'static + ?Sized)*)?> $crate::ReborrowType<'a> for $ident<'a $(, $($T)*)?> {
            type Reborrow<'b> = $ident<'b $(, $($T)*)?>;
        }

        impl<'a $(, $($T: 'static + ?Sized)*)?> $crate::Reborrow<'a> for $ident<'a $(, $($T)*)?> {
            fn reborrow<'b>(self) -> Self::Reborrow<'b>
            where
                'a: 'b,
            {
                self
            }

            fn reborrow_ref<'b>(&'b self) -> &'b Self::Reborrow<'b>
            where
                'a: 'b,
            {
                self
            }
        }
    };
}

type Ref<'a, T> = &'a T;

derive_reborrow!(Ref, generics: [T]);

pub trait ToOwn<'a>: Reborrow<'a> {
    type Owned: for<'b> ToRef<'b, Self::Reborrow<'b>>;

    fn to_own(&self) -> Self::Owned;
}

impl<'a, T: ToOwned + 'static + ?Sized> ToOwn<'a> for &'a T {
    type Owned = T::Owned;

    fn to_own(&self) -> Self::Owned {
        (*self).to_owned()
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

pub enum Boo<'a, R: ToOwn<'a>> {
    Reference(R),
    Owned(R::Owned),
}

impl<'a, R: ToOwn<'a>> Boo<'a, R> {
    pub fn into_owned(self) -> R::Owned {
        match self {
            Self::Reference(reference) => reference.to_own(),
            Self::Owned(owned) => owned,
        }
    }

    pub fn to_mut(&mut self) -> &mut R::Owned
    where
        R: Clone,
    {
        match self {
            Self::Reference(reference) => {
                *self = Self::Owned(reference.to_own());
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

impl<'b, 'a: 'b, R: ToOwn<'a>> ToRef<'b, BooRef<'b, 'a, R>> for Boo<'a, R> {
    fn to_ref(&'b self) -> BooRef<'b, 'a, R> {
        self.to_ref()
    }
}

pub enum BooRef<'b, 'a: 'b, R: ToOwn<'a>> {
    Reference(&'b R::Reborrow<'b>),
    Owned(R::Reborrow<'b>),
}

impl<'b, 'a: 'b, R: ToOwn<'a>> Deref for BooRef<'b, 'a, R> {
    type Target = R::Reborrow<'b>;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Reference(reference) => reference,
            Self::Owned(owned) => owned,
        }
    }
}

impl<'b, 'a: 'b, R: ToOwn<'a>> BooRef<'b, 'a, R> {
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
