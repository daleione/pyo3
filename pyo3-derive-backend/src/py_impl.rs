// Copyright (c) 2017-present PyO3 Project and Contributors

use proc_macro2::TokenStream;
use py_method;
use syn;

pub fn build_py_methods(ast: &mut syn::ItemImpl) -> TokenStream {
    if ast.trait_.is_some() {
        panic!("#[pymethods] can not be used only with trait impl block");
    } else if ast.generics != Default::default() {
        panic!("#[pymethods] can not ve used with lifetime parameters or generics");
    } else {
        impl_methods(&ast.self_ty, &mut ast.items)
    }
}

pub fn impl_methods(ty: &syn::Type, impls: &mut Vec<syn::ImplItem>) -> TokenStream {
    // get method names in impl block
    let mut methods = Vec::new();
    for iimpl in impls.iter_mut() {
        if let syn::ImplItem::Method(ref mut meth) = iimpl {
            let name = meth.sig.ident.clone();
            methods.push(py_method::gen_py_method(
                ty,
                &name,
                &mut meth.sig,
                &mut meth.attrs,
            ));
        }
    }

    quote! {
        impl ::pyo3::class::methods::PyMethodsProtocolImpl for #ty {
            fn py_methods() -> &'static [::pyo3::class::PyMethodDefType] {
                static METHODS: &'static [::pyo3::class::PyMethodDefType] = &[
                    #(#methods),*
                ];
                METHODS
            }
        }
    }
}
