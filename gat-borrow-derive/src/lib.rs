use quote::{ToTokens, quote};
use syn::{
    Attribute, Generics, Ident, Item, Lifetime, Token, Type, fold::Fold, parse::Parse,
    spanned::Spanned,
};

struct ConvertLifetimes<'a> {
    from: &'a Lifetime,
    to: &'a Lifetime,
}

impl Fold for ConvertLifetimes<'_> {
    fn fold_lifetime(&mut self, lt: Lifetime) -> Lifetime {
        if lt == *self.from {
            self.to.clone()
        } else {
            lt
        }
    }
}

struct Opts {
    reborrow: Type,
}

impl Parse for Opts {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "reborrow" {
            return Err(syn::Error::new_spanned(
                ident,
                "expected 'reborrow = <type>'",
            ));
        }

        let _: Token![=] = input.parse()?;

        let reborrow: Type = input.parse()?;

        Ok(Self { reborrow })
    }
}

fn implement_reborrow(
    ident: &Ident,
    generics: Generics,
    attrs: &[Attribute],
) -> syn::Result<proc_macro2::TokenStream> {
    let mut lifetimes = generics.lifetimes();
    let (Some(reborrow_lt), None) = (lifetimes.next(), lifetimes.next()) else {
        return Err(syn::Error::new_spanned(
            generics,
            "expected exactly one lifetime",
        ));
    };

    let lt_span = reborrow_lt.span();
    let reborrow_lt = reborrow_lt.clone();

    let opts: Vec<Opts> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("gat_borrow"))
        .map(Attribute::parse_args)
        .collect::<syn::Result<_>>()?;

    let reborrow_ty = if let Some(opts) = opts.into_iter().next_back() {
        opts.reborrow.into_token_stream()
    } else {
        let (_, reborrow_ty_generics, _) = generics.split_for_impl();
        quote! { #ident #reborrow_ty_generics }
    };

    let outer_lifetime = Lifetime::new("'gat_borrow", lt_span);
    let generics = ConvertLifetimes {
        from: &reborrow_lt.lifetime,
        to: &outer_lifetime,
    }
    .fold_generics(generics);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let tokens = quote! {
        impl #impl_generics gat_borrow::ReborrowType<#outer_lifetime> for #ident #ty_generics #where_clause {
            type Reborrow<#reborrow_lt> = #reborrow_ty;
        }

        impl #impl_generics gat_borrow::ReborrowMethods<#outer_lifetime> for #ident #ty_generics #where_clause {
            fn reborrow<'b>(self) -> Self::Reborrow<'b>
            where
                #outer_lifetime: 'b,
            {
                self
            }

            fn reborrow_ref<'b>(&'b self) -> &'b Self::Reborrow<'b>
            where
                #outer_lifetime: 'b,
            {
                self
            }
        }
    };

    Ok(tokens)
}

fn parse_item(item: proc_macro2::TokenStream) -> syn::Result<(Ident, Generics, Vec<Attribute>)> {
    let item: Item = syn::parse2(item)?;
    match item {
        Item::Struct(strct) => Ok((strct.ident, strct.generics, strct.attrs)),
        Item::Enum(enm) => Ok((enm.ident, enm.generics, enm.attrs)),
        Item::Union(enm) => Ok((enm.ident, enm.generics, enm.attrs)),
        Item::Type(typ) => Ok((typ.ident, typ.generics, typ.attrs)),
        _ => Err(syn::Error::new_spanned(
            item,
            "expected a struct, enum, union, or typedef",
        )),
    }
}

#[proc_macro_derive(Reborrow, attributes(gat_borrow))]
pub fn derive_reborrow_attribute(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    parse_item(item.into())
        .and_then(|(ident, generics, attrs)| implement_reborrow(&ident, generics, &attrs))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
pub fn derive_reborrow(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item: proc_macro2::TokenStream = item.into();

    let reborrow = parse_item(item.clone())
        .and_then(|(ident, generics, attrs)| implement_reborrow(&ident, generics, &attrs))
        .unwrap_or_else(syn::Error::into_compile_error);

    quote! {
        #item
        #reborrow
    }
    .into()
}
